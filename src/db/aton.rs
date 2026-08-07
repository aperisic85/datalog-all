//! Upiti za izvor `aton_csd` — konfiguracija objekata i očitanja.

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppResult;
use crate::models::aton::{AtonPollConfig, AtonReading, AtonReadingInsert, LatestAtonReading};
use crate::models::domain::TimeRangeQuery;

const POLL_CONFIG_COLS: &str = "id, station_id, name,
     aton_snopsy_endpoint, aton_number, aton_addr, aton_reg_count, aton_sync_clock,
     aton_connect_timeout_sec, aton_response_timeout_sec,
     poll_interval_sec";

/// Svi aktivni AtoN objekti s uključenim prozivanjem i potpunom konfiguracijom.
pub async fn list_pollable_aton_objects(pool: &PgPool) -> AppResult<Vec<AtonPollConfig>> {
    Ok(sqlx::query_as::<_, AtonPollConfig>(&format!(
        "SELECT {POLL_CONFIG_COLS}
         FROM objects
         WHERE is_active = TRUE
           AND polling_enabled = TRUE
           AND source_kind = 'aton_csd'
           AND aton_snopsy_endpoint IS NOT NULL
           AND aton_number IS NOT NULL
           AND aton_addr IS NOT NULL
         ORDER BY aton_snopsy_endpoint, station_id"))
        .fetch_all(pool).await?)
}

/// Konfiguracija jednog AtoN objekta (za ručni poll iz sučelja).
pub async fn get_aton_poll_config(pool: &PgPool, id: Uuid) -> AppResult<Option<AtonPollConfig>> {
    Ok(sqlx::query_as::<_, AtonPollConfig>(&format!(
        "SELECT {POLL_CONFIG_COLS}
         FROM objects
         WHERE id = $1 AND is_active = TRUE AND source_kind = 'aton_csd'"))
        .bind(id)
        .fetch_optional(pool).await?)
}

/// Upiši očitanje. Vraća `true` ako je zapis stvarno umetnut (nije duplikat).
pub async fn insert_aton_reading(pool: &PgPool, r: &AtonReadingInsert) -> AppResult<bool> {
    let result = sqlx::query(
        "INSERT INTO aton_readings (object_id, station_id, recorded_at,
             temp_trenutna_c, temp_0100_c, temp_1300_c,
             gl_svj_napon_v, gl_svj_struja_a, automat_napon_v, automat_struja_a,
             prosjek_napon_gl_svj_v, prosjek_napon_automat_v,
             punjenje_gl_svj_a, punjenje_automat_a,
             potrosnja_gl_svj_a, potrosnja_automat_a,
             potrosnja_izvor_a, dnevna_potrosnja_a, regs)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
         ON CONFLICT (object_id, recorded_at) DO NOTHING")
        .bind(r.object_id).bind(&r.station_id).bind(r.recorded_at)
        .bind(r.temp_trenutna_c).bind(r.temp_0100_c).bind(r.temp_1300_c)
        .bind(r.gl_svj_napon_v).bind(r.gl_svj_struja_a)
        .bind(r.automat_napon_v).bind(r.automat_struja_a)
        .bind(r.prosjek_napon_gl_svj_v).bind(r.prosjek_napon_automat_v)
        .bind(r.punjenje_gl_svj_a).bind(r.punjenje_automat_a)
        .bind(r.potrosnja_gl_svj_a).bind(r.potrosnja_automat_a)
        .bind(r.potrosnja_izvor_a).bind(r.dnevna_potrosnja_a)
        .bind(&r.regs)
        .execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_aton_readings(
    pool: &PgPool, object_id: Uuid, q: &TimeRangeQuery,
) -> AppResult<Vec<AtonReading>> {
    let limit = q.limit.unwrap_or(144).min(1000);
    Ok(sqlx::query_as::<_, AtonReading>(
        "SELECT * FROM aton_readings
         WHERE object_id = $1
           AND ($2::timestamptz IS NULL OR recorded_at >= $2)
           AND ($3::timestamptz IS NULL OR recorded_at <= $3)
         ORDER BY recorded_at DESC LIMIT $4")
        .bind(object_id).bind(q.from).bind(q.to).bind(limit)
        .fetch_all(pool).await?)
}

pub async fn get_latest_aton_reading(
    pool: &PgPool, object_id: Uuid,
) -> AppResult<Option<LatestAtonReading>> {
    Ok(sqlx::query_as::<_, LatestAtonReading>(
        "SELECT * FROM v_latest_aton_readings WHERE object_id = $1")
        .bind(object_id)
        .fetch_optional(pool).await?)
}
