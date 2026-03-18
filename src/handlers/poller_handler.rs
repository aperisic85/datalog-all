use axum::{extract::{Path, State}, Extension, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::domain as db;
use crate::errors::{AppError, AppResult};
use crate::models::domain::JwtClaims;
use crate::poller::{self, client::{Cr300Client, DataloggerConfig, PollState, TableConfig}, SharedPollerStatus};

/// POST /api/v1/objects/:id/poll
/// Ručno pokreni poll za jedan objekt (dohvati zadnje podatke odmah)
pub async fn poll_object_now(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let _uid = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

    let obj = db::get_object_by_id(&pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("Object {}", id)))?;

    let url = obj.datalogger_url.clone()
        .ok_or_else(|| AppError::BadRequest("Objekt nema konfiguriran datalogger URL".into()))?;

    let tables = vec![
        TableConfig { name: "Measurements_10min".into(), initial_records: 5 },
        TableConfig { name: "Alarms_10min".into(),       initial_records: 5 },
        TableConfig { name: "Event_log".into(),           initial_records: 10 },
    ];

    let config = DataloggerConfig {
        name:              obj.station_id.clone(),
        url,
        username:          Some("anonymous".to_string()),
        password:          None,
        poll_interval_sec: 0,
        tables:            tables.clone(),
    };

    let client = Cr300Client::new(config).map_err(|e| AppError::Internal(e))?;

    let mut results = vec![];
    for table_cfg in &tables {
        let mut state = PollState::default();
        match poller::poll_one_table(&client, &pool, &obj.station_id, table_cfg, &mut state).await {
            Ok(n)  => results.push(serde_json::json!({ "table": table_cfg.name, "records": n })),
            Err(e) => results.push(serde_json::json!({ "table": table_cfg.name, "error": e.to_string() })),
        }
    }

    Ok(Json(serde_json::json!({ "station_id": obj.station_id, "results": results })))
}

/// GET /api/v1/poller/status
pub async fn poller_status(
    State(status): State<SharedPollerStatus>,
) -> Json<serde_json::Value> {
    let s = status.read().await;
    Json(serde_json::json!({
        "stations": s.online.iter().map(|(name, online)| serde_json::json!({
            "name":       name,
            "online":     online,
            "last_poll":  s.last_poll.get(name),
            "last_error": s.last_error.get(name),
        })).collect::<Vec<_>>()
    }))
}

/// POST /api/v1/control/setvalue
/// Pošalji SetValueEx komandu na CR300 (npr. upaliti fenjer ručno)
/// Zahtijeva operator ili admin ulogu
#[derive(Deserialize)]
pub struct SetValueRequest {
    pub object_id:  Uuid,
    pub table:      String,  // "Public"
    pub field:      String,  // "Lan_set_always_on"
    pub value:      String,  // "1"
}

#[derive(Serialize)]
pub struct SetValueResponse {
    pub success: bool,
    pub message: String,
}

pub async fn set_datalogger_value(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<SetValueRequest>,
) -> AppResult<Json<SetValueResponse>> {
    // Provjeri operator ili admin prava na regiju objekta
    let uid = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let obj = db::get_object_by_id(&pool, req.object_id).await?
        .ok_or_else(|| AppError::NotFound(format!("Object {}", req.object_id)))?;

    if !db::user_can_control_in_region(&pool, uid, &claims.role, obj.region_id).await? {
        return Err(AppError::Forbidden);
    }

    let url = obj.datalogger_url.clone().ok_or_else(|| AppError::BadRequest("Objekt nema konfiguriran datalogger URL".into()))?;

    let config = DataloggerConfig {
        name:              obj.name.clone(),
        url,
        username:          obj.datalogger_url.map(|_| "anonymous".to_string()),
        password:          None,
        poll_interval_sec: 0,
        tables:            vec![],
    };

    let client = Cr300Client::new(config).map_err(|e| AppError::Internal(e))?;

    let result = client.set_value(&req.table, &req.field, &req.value).await;

    let _ = db::write_audit(&pool, Some(uid), Some(&claims.username),
        "SET_VALUE", Some("object"), Some(&req.object_id.to_string()),
        Some(serde_json::json!({"table": req.table, "field": req.field, "value": req.value})),
        None).await;

    match result {
        Ok(true) => Ok(Json(SetValueResponse { success: true,  message: format!("Set {}.{} = {}", req.table, req.field, req.value) })),
        Ok(false) => Ok(Json(SetValueResponse { success: false, message: "Datalogger odbio komandu".into() })),
        Err(e)   => Ok(Json(SetValueResponse { success: false, message: e.to_string() })),
    }
}
