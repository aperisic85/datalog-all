pub mod domain;
pub mod parser;
pub mod poller_handler;

use axum::{extract::State, http::StatusCode, Json};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::db::domain as db;
use crate::errors::AppResult;
use crate::models::{DataloggerPayload, HealthResponse, IngestResponse};

use parser::{
    detect_station_name, detect_table_name,
    parse_alarms, parse_event_logs,
    parse_measurements_10min, parse_measurements_1h, parse_measurements_24h,
};

// ── Health ────────────────────────────────────────────────────────────────

pub async fn health(State(pool): State<PgPool>) -> Json<HealthResponse> {
    let db_ok = sqlx::query("SELECT 1").execute(&pool).await.is_ok();
    Json(HealthResponse {
        status:   "ok",
        version:  env!("CARGO_PKG_VERSION"),
        database: if db_ok { "connected" } else { "error" },
    })
}

// ── Ingest: POST /api/v1/datalogger/alarms ────────────────────────────────

pub async fn ingest_alarms(
    State(pool): State<PgPool>,
    Json(payload): Json<DataloggerPayload>,
) -> AppResult<(StatusCode, Json<IngestResponse>)> {
    let station_id = detect_station_name(&payload).unwrap_or_else(|| "unknown".into());
    info!(station = %station_id, rows = payload.data.len(), "← alarms");

    let records = parse_alarms(&payload, &station_id)?;
    let count   = records.len();

    for rec in &records {
        db::insert_alarm(&pool, rec).await?;
        // Logiraj kritične alarme na server
        if rec.alarm_battery_voltage_flat    > 0 { warn!(station=%station_id, "ALARM: Baterija prazna!"); }
        if rec.alarm_lantern_night_light_off > 0 { warn!(station=%station_id, "ALARM: Fenjer ugašen noću!"); }
        if rec.alarm_station_out_of_radius   > 0 { warn!(station=%station_id, "ALARM: Stanica van radijusa!"); }
    }

    Ok((StatusCode::CREATED, Json(IngestResponse { status: "ok", records_inserted: count, table: "alarms".into() })))
}

// ── Ingest: POST /api/v1/datalogger/measurements ──────────────────────────
// CR300 šalje sve 3 tablice na isti URL, razlikujemo po head.environment.table_name

pub async fn ingest_measurements(
    State(pool): State<PgPool>,
    Json(payload): Json<DataloggerPayload>,
) -> AppResult<(StatusCode, Json<IngestResponse>)> {
    let station_id = detect_station_name(&payload).unwrap_or_else(|| "unknown".into());
    let table_name = detect_table_name(&payload).unwrap_or_else(|| "Measurements_10min".into());
    info!(station = %station_id, table = %table_name, rows = payload.data.len(), "← measurements");

    let tl    = table_name.to_lowercase();
    let count = if tl.contains("10min") {
        let recs = parse_measurements_10min(&payload, &station_id)?;
        let n = recs.len();
        for r in &recs { db::insert_measurement_10min(&pool, r).await?; }
        n
    } else if tl.ends_with("_1h") || tl.contains("_1h") {
        let recs = parse_measurements_1h(&payload, &station_id)?;
        let n = recs.len();
        for r in &recs { db::insert_measurement_1h(&pool, r).await?; }
        n
    } else if tl.contains("24h") {
        let recs = parse_measurements_24h(&payload, &station_id)?;
        let n = recs.len();
        for r in &recs { db::insert_measurement_24h(&pool, r).await?; }
        n
    } else {
        warn!(table=%table_name, "Nepoznata tablica, koristim 10min parser");
        let recs = parse_measurements_10min(&payload, &station_id)?;
        let n = recs.len();
        for r in &recs { db::insert_measurement_10min(&pool, r).await?; }
        n
    };

    Ok((StatusCode::CREATED, Json(IngestResponse { status: "ok", records_inserted: count, table: table_name })))
}

// ── Ingest: POST /api/v1/datalogger/eventlogs ────────────────────────────

pub async fn ingest_event_logs(
    State(pool): State<PgPool>,
    Json(payload): Json<DataloggerPayload>,
) -> AppResult<(StatusCode, Json<IngestResponse>)> {
    let station_id = detect_station_name(&payload).unwrap_or_else(|| "unknown".into());
    info!(station = %station_id, rows = payload.data.len(), "← eventlogs");

    let records = parse_event_logs(&payload, &station_id)?;
    let count   = records.len();
    for rec in &records {
        match rec.log_level {
            4 => tracing::error!(station=%station_id, msg=%rec.log_message, "FATAL"),
            3 => tracing::error!(station=%station_id, msg=%rec.log_message, "ERROR"),
            2 => warn!(station=%station_id, msg=%rec.log_message, "WARN"),
            _ => info!(station=%station_id, msg=%rec.log_message, "INFO"),
        }
        db::insert_event_log(&pool, rec).await?;
    }

    Ok((StatusCode::CREATED, Json(IngestResponse { status: "ok", records_inserted: count, table: "event_logs".into() })))
}
