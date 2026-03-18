pub mod domain;

// ──────────────────────────────────────────────────────────────────────────
// Campbell CR300 push payload format
// Datalogger šalje via HTTPPost — isti format za sve 3 tablice
// ──────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// Root payload koji CR300 šalje na naše ingest endpointe
#[derive(Debug, Deserialize)]
pub struct DataloggerPayload {
    pub head: PayloadHead,
    pub data: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PayloadHead {
    pub transaction: Option<i64>,
    pub signature:   Option<i64>,
    pub environment: Option<PayloadEnvironment>,
    pub fields:      Vec<FieldDef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PayloadEnvironment {
    pub station_name: Option<String>,
    pub table_name:   Option<String>,
    pub model:        Option<String>,
    pub os_version:   Option<String>,
    pub prog_name:    Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FieldDef {
    pub name:        String,
    pub processing:  Option<String>,
    pub units:       Option<String>,
    #[serde(rename = "type")]
    pub field_type:  Option<String>,
}

// Response types za ingest endpointe
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub status:           &'static str,
    pub records_inserted: usize,
    pub table:            String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status:   &'static str,
    pub version:  &'static str,
    pub database: &'static str,
}
