//! HTTP sloj za kategoriju izvora `aton_csd`.

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::aton as db_aton;
use crate::db::domain as db;
use crate::errors::{AppError, AppResult};
use crate::models::aton::{AtonReading, LatestAtonReading};
use crate::models::domain::{JwtClaims, TimeRangeQuery};
use crate::poller::aton::{endpoint_lock, poll_aton_once, AtonStation};

/// GET /api/v1/objects/:id/aton/latest — zadnje dekodirano očitanje.
pub async fn get_latest_aton_reading(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Option<LatestAtonReading>>> {
    Ok(Json(db_aton::get_latest_aton_reading(&pool, id).await?))
}

/// GET /api/v1/objects/:id/aton/readings — povijest očitanja.
pub async fn get_aton_readings(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> AppResult<Json<Vec<AtonReading>>> {
    Ok(Json(db_aton::get_aton_readings(&pool, id, &q).await?))
}

/// POST /api/v1/objects/:id/aton/poll — ručno digni CSD poziv i prozovi RTU.
///
/// Zahtijeva operator/admin prava (poziv troši minute na SIM-u i zauzima
/// liniju). Čeka istu bravu kao periodični poller, pa se nikad ne preklopi
/// s pozivom drugog objekta na istom snopsy_r-u.
pub async fn poll_aton_now(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let uid = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let obj = db::get_object_by_id(&pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("Object {}", id)))?;

    if !db::user_can_control_in_region(&pool, uid, &claims.role, obj.region_id).await? {
        return Err(AppError::Forbidden);
    }

    let cfg = db_aton::get_aton_poll_config(&pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("AtoN objekt {}", id)))?;

    let station = AtonStation::from_poll_config(&cfg).ok_or_else(|| {
        AppError::BadRequest(
            "Objekt nema potpunu AtoN konfiguraciju (snopsy_r endpoint, tel. broj, Modbus adresa)".into(),
        )
    })?;

    let lock = endpoint_lock(&station.endpoint);

    match poll_aton_once(&pool, &station, &lock).await {
        Ok(a) => Ok(Json(serde_json::json!({
            "station_id":  station.station_id,
            "success":     true,
            "temperatura_c":     a.temp_trenutna_c,
            "gl_svj_napon_v":    a.gl_svj.napon_v,
            "gl_svj_struja_a":   a.gl_svj.struja_a,
            "automat_napon_v":   a.automat.napon_v,
            "automat_struja_a":  a.automat.struja_a,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "station_id": station.station_id,
            "success":    false,
            "error":      e.to_string(),
        }))),
    }
}
