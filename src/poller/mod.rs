pub mod client;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::db::domain as db;
use crate::handlers::parser::{
    parse_alarms, parse_event_logs,
    parse_measurements_10min, parse_measurements_1h, parse_measurements_24h,
};

use client::{Cr300Client, DataloggerConfig, PollState, TableConfig};

// ── Shared poller status ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct PollerStatus {
    pub last_poll:  HashMap<String, chrono::DateTime<chrono::Utc>>,
    pub last_error: HashMap<String, String>,
    pub online:     HashMap<String, bool>,
}

pub type SharedPollerStatus = Arc<RwLock<PollerStatus>>;

// ── Per-station polling loop ──────────────────────────────────────────────

async fn poll_station(config: DataloggerConfig, pool: PgPool, status: SharedPollerStatus) {
    let mut states: HashMap<String, PollState> = config.tables.iter()
        .map(|t| (t.name.clone(), PollState::default()))
        .collect();

    let client = match Cr300Client::new(config.clone()) {
        Ok(c)  => c,
        Err(e) => { error!(station=%config.name, error=%e, "Failed to create HTTP client"); return; }
    };

    // Ensure station is registered in objects table
    info!(station=%config.name, url=%config.url, interval_sec=config.poll_interval_sec, "Poller started");

    let mut interval = tokio::time::interval(Duration::from_secs(config.poll_interval_sec.max(10)));

    loop {
        interval.tick().await;

        for table_cfg in &config.tables {
            let state = states.entry(table_cfg.name.clone()).or_default();

            match poll_one_table(&client, &pool, &config.name, table_cfg, state).await {
                Ok(n) if n > 0 => {
                    info!(station=%config.name, table=%table_cfg.name, records=n, "Ingested");
                    let mut s = status.write().await;
                    s.online.insert(config.name.clone(), true);
                    s.last_poll.insert(config.name.clone(), chrono::Utc::now());
                    s.last_error.remove(&config.name);
                }
                Ok(_) => {
                    let mut s = status.write().await;
                    s.online.insert(config.name.clone(), true);
                    s.last_poll.insert(config.name.clone(), chrono::Utc::now());
                }
                Err(e) => {
                    warn!(station=%config.name, table=%table_cfg.name, error=%e, "Poll error");
                    let mut s = status.write().await;
                    s.online.insert(config.name.clone(), false);
                    s.last_error.insert(config.name.clone(), e.to_string());
                }
            }
        }
    }
}

async fn poll_one_table(
    client: &Cr300Client,
    pool: &PgPool,
    station_id: &str,
    table_cfg: &TableConfig,
    state: &mut PollState,
) -> anyhow::Result<usize> {
    let result = client.poll_table(&table_cfg.name, state, table_cfg.initial_records).await?;
    let (payload, last_record_no) = match result {
        None => return Ok(0),
        Some(r) => r,
    };

    if let Some(rno) = last_record_no {
        state.last_record_no = Some(rno + 1);
    }

    let count = payload.data.len();
    let tl    = table_cfg.name.to_lowercase();

    if tl.contains("alarm") {
        let recs = parse_alarms(&payload, station_id)?;
        for r in &recs { db::insert_alarm(pool, r).await?; }
    } else if tl.contains("10min") {
        let recs = parse_measurements_10min(&payload, station_id)?;
        for r in &recs { db::insert_measurement_10min(pool, r).await?; }
    } else if tl.ends_with("_1h") || tl.contains("_1h") {
        let recs = parse_measurements_1h(&payload, station_id)?;
        for r in &recs { db::insert_measurement_1h(pool, r).await?; }
    } else if tl.contains("24h") {
        let recs = parse_measurements_24h(&payload, station_id)?;
        for r in &recs { db::insert_measurement_24h(pool, r).await?; }
    } else if tl.contains("event") || tl.contains("log") {
        let recs = parse_event_logs(&payload, station_id)?;
        for r in &recs { db::insert_event_log(pool, r).await?; }
    } else {
        warn!(table=%table_cfg.name, "Unknown table type, using 10min parser");
        let recs = parse_measurements_10min(&payload, station_id)?;
        for r in &recs { db::insert_measurement_10min(pool, r).await?; }
    }

    Ok(count)
}

// ── Start all pollers ─────────────────────────────────────────────────────

pub fn start_pollers(configs: Vec<DataloggerConfig>, pool: PgPool, status: SharedPollerStatus) {
    for config in configs {
        let pool   = pool.clone();
        let status = status.clone();
        tokio::spawn(async move { poll_station(config, pool, status).await; });
    }
}

// ── Load configs from environment ────────────────────────────────────────

pub fn load_configs_from_env() -> Vec<DataloggerConfig> {
    // JSON array override: DATALOGGER_STATIONS='[{...},{...}]'
    if let Ok(json) = std::env::var("DATALOGGER_STATIONS") {
        if let Ok(configs) = serde_json::from_str::<Vec<DataloggerConfig>>(&json) {
            info!("Loaded {} station configs from DATALOGGER_STATIONS", configs.len());
            return configs;
        }
    }

    // Single station env vars
    let url = std::env::var("DATALOGGER_URL").unwrap_or_default();
    if url.is_empty() { return vec![]; }

    vec![DataloggerConfig {
        name:              std::env::var("DATALOGGER_NAME").unwrap_or_else(|_| "Station".into()),
        url,
        username:          std::env::var("DATALOGGER_USER").ok(),
        password:          std::env::var("DATALOGGER_PASS").ok(),
        poll_interval_sec: std::env::var("DATALOGGER_POLL_SEC").ok()
            .and_then(|v| v.parse().ok()).unwrap_or(60),
        tables: vec![
            TableConfig { name: "Measurements_10min".into(), initial_records: 3 },
            TableConfig { name: "Measurements_1h".into(),    initial_records: 1 },
            TableConfig { name: "Measurements_24h".into(),   initial_records: 1 },
            TableConfig { name: "Alarms_10min".into(),       initial_records: 3 },
            TableConfig { name: "Event_log".into(),          initial_records: 20 },
        ],
    }]
}
