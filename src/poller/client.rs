/// Campbell CR300 Web API Client
///
/// CR300 exposes an HTTP server with CGI-style commands:
/// GET http://<ip>/?command=DataQuery&uri=dl:<table>&format=json&mode=most-recent&p1=<n>
///
/// Authentication: HTTP Basic Auth (username:password)
/// Default (old OS): anonymous / no password, read-only access
/// Default (OS >= 12.0): admin / <UID>, full access
///
/// JSON Response format:
/// {
///   "head": {
///     "transaction": 1,
///     "signature": 12345,
///     "environment": {
///       "station_name": "Galija",
///       "table_name": "Measurements_10min",
///       "model": "CR300",
///       "os_version": "CR300.Std.10",
///       "prog_name": "CPU:main.CR300"
///     },
///     "fields": [
///       {"name": "Datalogger_temperature_Avg", "type": "xsd:float", "units": "C", "processing": "Avg"},
///       ...
///     ]
///   },
///   "data": [
///     {"time": "2024-01-15T10:00:00", "vals": [25.3, 12.4, ...]},
///     ...
///   ],
///   "more": false   <-- if true, poll again with since-record
/// }

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::models::{DataloggerPayload, FieldDef, PayloadEnvironment, PayloadHead};

// ============================================================
// Datalogger station config
// ============================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataloggerConfig {
    /// Human-readable name
    pub name: String,
    /// HTTP base URL, e.g. "http://drvenik.ddns.net:8010"
    pub url: String,
    /// HTTP Basic Auth username (default: "anonymous")
    pub username: Option<String>,
    /// HTTP Basic Auth password (default: "")
    pub password: Option<String>,
    /// Poll interval in seconds
    pub poll_interval_sec: u64,
    /// Tables to poll
    pub tables: Vec<TableConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TableConfig {
    pub name: String,
    /// Records to fetch per poll (for most-recent mode on first run)
    pub initial_records: u32,
}

impl Default for DataloggerConfig {
    fn default() -> Self {
        Self {
            name: "Galija".to_string(),
            url: "http://drvenik.ddns.net:8010".to_string(),
            username: Some("anonymous".to_string()),
            password: None,
            poll_interval_sec: 60,
            tables: vec![
                TableConfig { name: "Measurements_10min".to_string(), initial_records: 1 },
                TableConfig { name: "Measurements_1h".to_string(),    initial_records: 1 },
                TableConfig { name: "Measurements_24h".to_string(),   initial_records: 1 },
                TableConfig { name: "Alarms_10min".to_string(),       initial_records: 1 },
                TableConfig { name: "Event_log".to_string(),          initial_records: 10 },
            ],
        }
    }
}

// ============================================================
// Raw CR300 JSON response types
// ============================================================

#[derive(Debug, Deserialize)]
pub struct Cr300Response {
    pub head: Cr300Head,
    /// CR300 ponekad vraća "data": null kad nema podataka — #[serde(default)] to pretvara u prazan Vec
    #[serde(default)]
    pub data: Vec<Cr300DataRow>,
    /// If true, more data is available - poll with since-record
    #[serde(default)]
    pub more: bool,
}

#[derive(Debug, Deserialize)]
pub struct Cr300Head {
    pub transaction: Option<i64>,
    pub signature: Option<i64>,
    pub environment: Option<Cr300Environment>,
    pub fields: Vec<Cr300Field>,
}

#[derive(Debug, Deserialize)]
pub struct Cr300Environment {
    pub station_name: Option<String>,
    pub table_name: Option<String>,
    pub model: Option<String>,
    pub os_version: Option<String>,
    pub prog_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Cr300Field {
    pub name: String,
    pub processing: Option<String>,
    pub units: Option<String>,
    #[serde(rename = "type")]
    pub field_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Cr300DataRow {
    /// Timestamp: "2024-01-15T10:00:00"
    pub time: String,
    /// Values array - parallel to head.fields
    pub vals: Vec<serde_json::Value>,
    /// Record number (used for incremental polling)
    pub no: Option<u64>,
}

// ============================================================
// Last-seen record tracking (per station per table)
// ============================================================

#[derive(Debug, Clone, Default)]
pub struct PollState {
    /// Last record number successfully ingested
    pub last_record_no: Option<u64>,
    /// Last timestamp successfully ingested  
    pub last_timestamp: Option<DateTime<Utc>>,
}

// ============================================================
// CR300 HTTP Client
// ============================================================

pub struct Cr300Client {
    client: Client,
    config: DataloggerConfig,
}

impl Cr300Client {
    pub fn new(config: DataloggerConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client, config })
    }

    /// Poll a single table, returns the parsed payload + last record no
    pub async fn poll_table(
        &self,
        table: &str,
        state: &PollState,
        fallback_records: u32,
    ) -> Result<Option<(DataloggerPayload, Option<u64>)>> {
        let (mode, p1, p2) = self.build_query_params(state, fallback_records);

        let url = format!(
            "{}/?command=DataQuery&uri=dl:{}&format=json&mode={}&p1={}{}",
            self.config.url.trim_end_matches('/'),
            table,
            mode,
            p1,
            p2.map(|v| format!("&p2={}", v)).unwrap_or_default()
        );

        debug!(url = %url, table = %table, "Polling datalogger");

        let mut req = self.client.get(&url);

        // Add Basic Auth if configured
        if let Some(username) = &self.config.username {
            req = req.basic_auth(
                username,
                self.config.password.as_deref(),
            );
        }

        let response = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    station = %self.config.name,
                    table = %table,
                    error = %e,
                    "Failed to reach datalogger"
                );
                return Ok(None);
            }
        };

        let status = response.status();
        if !status.is_success() {
            warn!(
                station = %self.config.name,
                table = %table,
                status = %status,
                "Datalogger returned error status"
            );
            return Ok(None);
        }

        let text = response.text().await.context("Failed to read response body")?;

        if text.trim().is_empty() {
            debug!(station = %self.config.name, table = %table, "Empty response from datalogger");
            return Ok(None);
        }

        let cr300_resp: Cr300Response = serde_json::from_str(&text)
            .with_context(|| {
                let preview = if text.len() > 500 { &text[..500] } else { &text };
                format!(
                    "Failed to parse CR300 JSON response for table {} (station: {}). First 500 chars: {}",
                    table, self.config.name, preview
                )
            })?;

        if cr300_resp.data.is_empty() {
            debug!(
                station = %self.config.name,
                table = %table,
                env_station = ?cr300_resp.head.environment.as_ref().and_then(|e| e.station_name.as_deref()),
                "No new data (empty data array)"
            );
            return Ok(None);
        }

        // Track the last record number for incremental polling
        let last_record_no = cr300_resp
            .data
            .last()
            .and_then(|row| row.no);

        info!(
            station = %self.config.name,
            table = %table,
            rows = cr300_resp.data.len(),
            more = cr300_resp.more,
            "Received data from datalogger"
        );

        // Convert to our DataloggerPayload format (shared with push path)
        let payload = self.convert_to_payload(cr300_resp, table);

        Ok(Some((payload, last_record_no)))
    }

    /// Poll a table and handle pagination (more=true)
    pub async fn poll_table_complete(
        &self,
        table: &str,
        state: &mut PollState,
        fallback_records: u32,
    ) -> Result<Option<DataloggerPayload>> {
        let mut all_data: Vec<serde_json::Value> = Vec::new();
        let mut fields: Option<Vec<FieldDef>> = None;
        let mut environment: Option<PayloadEnvironment> = None;
        let mut current_state = state.clone();

        loop {
            let result = self.poll_table(table, &current_state, fallback_records).await?;

            match result {
                None => break,
                Some((payload, last_record_no)) => {
                    // Save field definitions from first response
                    if fields.is_none() {
                        fields = Some(payload.head.fields.clone());
                        environment = payload.head.environment.clone();
                    }

                    let has_more = payload.data.last()
                        .map(|_| false) // more flag is in cr300_resp.more, handled below
                        .unwrap_or(false);

                    all_data.extend(payload.data);

                    // Update state for next page
                    if let Some(rno) = last_record_no {
                        current_state.last_record_no = Some(rno + 1);
                    }

                    // Update persistent state
                    state.last_record_no = current_state.last_record_no;

                    // Stop if no more pages
                    if !has_more {
                        break;
                    }
                }
            }
        }

        if all_data.is_empty() {
            return Ok(None);
        }

        let combined = DataloggerPayload {
            head: PayloadHead {
                transaction: None,
                signature: None,
                environment,
                fields: fields.unwrap_or_default(),
            },
            data: all_data,
        };

        Ok(Some(combined))
    }

    fn build_query_params(&self, state: &PollState, fallback_records: u32) -> (&'static str, String, Option<String>) {
        if let Some(record_no) = state.last_record_no {
            // Incremental: fetch only new records since last seen
            ("since-record", record_no.to_string(), None)
        } else if let Some(ts) = state.last_timestamp {
            // Time-based fallback
            ("since-time", ts.format("%Y-%m-%dT%H:%M:%S").to_string(), None)
        } else {
            // First run: get last N records
            ("most-recent", fallback_records.to_string(), None)
        }
    }

    /// Convert CR300 native response format to our shared DataloggerPayload
    fn convert_to_payload(&self, resp: Cr300Response, table_name: &str) -> DataloggerPayload {
        use crate::models::{FieldDef, PayloadEnvironment, PayloadHead};

        let fields = resp
            .head
            .fields
            .into_iter()
            .map(|f| FieldDef {
                name: f.name,
                processing: f.processing,
                units: f.units,
                field_type: f.field_type,
            })
            .collect();

        let environment = resp.head.environment.map(|e| PayloadEnvironment {
            station_name: e.station_name.or_else(|| Some(self.config.name.clone())),
            table_name: e.table_name.or_else(|| Some(table_name.to_string())),
            model: e.model,
            os_version: e.os_version,
            prog_name: e.prog_name,
        });

        // Convert each row to our JSON format: {"time": "...", "vals": [...]}
        let data = resp
            .data
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "time": row.time,
                    "vals": row.vals
                })
            })
            .collect();

        DataloggerPayload {
            head: PayloadHead {
                transaction: resp.head.transaction,
                signature: resp.head.signature,
                environment,
                fields,
            },
            data,
        }
    }

    /// BrowseSymbols - list all tables on the datalogger
    pub async fn browse_tables(&self) -> Result<Vec<String>> {
        let url = format!(
            "{}/?command=BrowseSymbols&uri=dl:&format=json",
            self.config.url.trim_end_matches('/')
        );

        let mut req = self.client.get(&url);
        if let Some(username) = &self.config.username {
            req = req.basic_auth(username, self.config.password.as_deref());
        }

        let resp = req.send().await?;
        let text = resp.text().await?;

        #[derive(Deserialize)]
        struct BrowseResponse {
            symbols: Vec<BrowseSymbol>,
        }
        #[derive(Deserialize)]
        struct BrowseSymbol {
            name: String,
            #[serde(rename = "type")]
            sym_type: Option<u32>,
        }

        let browse: BrowseResponse = serde_json::from_str(&text)?;
        // type=6 means Table
        let tables = browse
            .symbols
            .into_iter()
            .filter(|s| s.sym_type == Some(6))
            .map(|s| s.name)
            .collect();

        Ok(tables)
    }

    /// SetValueEx - set a variable on the datalogger
    /// Requires Read/Write or All access level
    pub async fn set_value(&self, table: &str, field: &str, value: &str) -> Result<bool> {
        let url = format!(
            "{}/?command=SetValueEx&uri=dl:{}.{}&value={}&format=json",
            self.config.url.trim_end_matches('/'),
            table,
            field,
            urlencoding::encode(value)
        );

        let mut req = self.client.get(&url);
        if let Some(username) = &self.config.username {
            req = req.basic_auth(username, self.config.password.as_deref());
        }

        let resp = req.send().await?;
        let text = resp.text().await?;

        #[derive(Deserialize)]
        struct SetValueResponse {
            outcome: Option<u32>,
        }

        let result: SetValueResponse = serde_json::from_str(&text)?;
        // outcome=1 means success
        Ok(result.outcome == Some(1))
    }

    /// ClockCheck - get current datalogger time
    pub async fn clock_check(&self) -> Result<Option<DateTime<Utc>>> {
        let url = format!(
            "{}/?command=ClockCheck&format=json",
            self.config.url.trim_end_matches('/')
        );

        let mut req = self.client.get(&url);
        if let Some(username) = &self.config.username {
            req = req.basic_auth(username, self.config.password.as_deref());
        }

        let resp = req.send().await?;
        let text = resp.text().await?;

        #[derive(Deserialize)]
        struct ClockResponse {
            time: Option<String>,
        }

        let result: ClockResponse = serde_json::from_str(&text).unwrap_or(ClockResponse { time: None });

        Ok(result.time.and_then(|t| {
            DateTime::parse_from_rfc3339(&t)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|| {
                    chrono::NaiveDateTime::parse_from_str(&t, "%Y-%m-%dT%H:%M:%S")
                        .ok()
                        .map(|ndt| ndt.and_utc())
                })
        }))
    }
}
