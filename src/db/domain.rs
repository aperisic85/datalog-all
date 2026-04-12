use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppResult;
use crate::models::domain::*;

// ================================================================
// REGIONS
// ================================================================

pub async fn list_regions(pool: &PgPool) -> AppResult<Vec<Region>> {
    Ok(sqlx::query_as!(Region,
        "SELECT * FROM regions WHERE is_active = TRUE ORDER BY name")
        .fetch_all(pool).await?)
}

pub async fn list_user_regions(pool: &PgPool, user_id: Uuid, role: &str) -> AppResult<Vec<Region>> {
    if role == "admin" { return list_regions(pool).await; }
    Ok(sqlx::query_as!(Region,
        r#"SELECT r.* FROM regions r
           JOIN user_region_access ura ON ura.region_id = r.id AND ura.user_id = $1
           WHERE r.is_active = TRUE ORDER BY r.name"#, user_id)
        .fetch_all(pool).await?)
}

pub async fn get_region(pool: &PgPool, id: Uuid) -> AppResult<Option<Region>> {
    Ok(sqlx::query_as!(Region, "SELECT * FROM regions WHERE id = $1", id)
        .fetch_optional(pool).await?)
}

pub async fn create_region(pool: &PgPool, req: &CreateRegionRequest) -> AppResult<Region> {
    Ok(sqlx::query_as!(Region,
        "INSERT INTO regions (name, code, description, color) VALUES ($1, $2, $3, $4) RETURNING *",
        req.name, req.code, req.description,
        req.color.as_deref().unwrap_or("#2563eb"))
        .fetch_one(pool).await?)
}

pub async fn update_region(pool: &PgPool, id: Uuid, req: &UpdateRegionRequest) -> AppResult<Region> {
    Ok(sqlx::query_as!(Region,
        r#"UPDATE regions SET
               name        = COALESCE($2, name),
               description = COALESCE($3, description),
               color       = COALESCE($4, color),
               is_active   = COALESCE($5, is_active)
           WHERE id = $1 RETURNING *"#,
        id, req.name, req.description, req.color, req.is_active)
        .fetch_one(pool).await?)
}

pub async fn list_region_summary(pool: &PgPool, user_id: Uuid, role: &str) -> AppResult<Vec<RegionSummary>> {
    if role == "admin" {
        return Ok(sqlx::query_as!(RegionSummary,
            "SELECT * FROM v_region_summary ORDER BY region_name")
            .fetch_all(pool).await?);
    }
    Ok(sqlx::query_as!(RegionSummary,
        r#"SELECT s.* FROM v_region_summary s
           JOIN user_region_access ura ON ura.region_id = s.region_id AND ura.user_id = $1
           ORDER BY s.region_name"#, user_id)
        .fetch_all(pool).await?)
}

// ================================================================
// STATION TYPES
// ================================================================

pub async fn list_station_types(pool: &PgPool) -> AppResult<Vec<StationType>> {
    Ok(sqlx::query_as!(StationType, "SELECT * FROM station_types ORDER BY name")
        .fetch_all(pool).await?)
}

// ================================================================
// OBJECTS
// ================================================================

pub async fn list_objects(
    pool: &PgPool, user_id: Uuid, role: &str, q: &ObjectsQuery,
) -> AppResult<Page<ObjectView>> {
    let page      = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).min(100);
    let offset    = (page - 1) * page_size;
    let search    = q.search.as_deref().map(|s| format!("%{}%", s));

    let (total, rows) = if role == "admin" {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM v_objects
             WHERE ($1::boolean IS NULL OR is_active   = $1)
               AND ($2::boolean IS NULL OR alarm_active = $2)
               AND ($3::uuid    IS NULL OR region_id   = $3)
               AND ($4::text    IS NULL OR name ILIKE $4 OR station_id ILIKE $4 OR location_name ILIKE $4)")
            .bind(q.active).bind(q.in_alarm).bind(q.region_id).bind(&search)
            .fetch_one(pool).await?;

        let rows: Vec<ObjectView> = sqlx::query_as(
            "SELECT * FROM v_objects
             WHERE ($1::boolean IS NULL OR is_active   = $1)
               AND ($2::boolean IS NULL OR alarm_active = $2)
               AND ($3::uuid    IS NULL OR region_id   = $3)
               AND ($4::text    IS NULL OR name ILIKE $4 OR station_id ILIKE $4 OR location_name ILIKE $4)
             ORDER BY region_name, name LIMIT $5 OFFSET $6")
            .bind(q.active).bind(q.in_alarm).bind(q.region_id).bind(&search)
            .bind(page_size).bind(offset)
            .fetch_all(pool).await?;
        (total, rows)
    } else {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM v_objects o
             JOIN user_region_access ura ON ura.region_id = o.region_id AND ura.user_id = $1
             WHERE ($2::boolean IS NULL OR o.is_active   = $2)
               AND ($3::boolean IS NULL OR o.alarm_active = $3)
               AND ($4::uuid    IS NULL OR o.region_id   = $4)
               AND ($5::text    IS NULL OR o.name ILIKE $5 OR o.station_id ILIKE $5)")
            .bind(user_id).bind(q.active).bind(q.in_alarm).bind(q.region_id).bind(&search)
            .fetch_one(pool).await?;

        let rows: Vec<ObjectView> = sqlx::query_as(
            "SELECT DISTINCT ON (o.id) o.* FROM v_objects o
             JOIN user_region_access ura ON ura.region_id = o.region_id AND ura.user_id = $1
             WHERE ($2::boolean IS NULL OR o.is_active   = $2)
               AND ($3::boolean IS NULL OR o.alarm_active = $3)
               AND ($4::uuid    IS NULL OR o.region_id   = $4)
               AND ($5::text    IS NULL OR o.name ILIKE $5 OR o.station_id ILIKE $5)
             ORDER BY o.id, o.region_name, o.name LIMIT $6 OFFSET $7")
            .bind(user_id).bind(q.active).bind(q.in_alarm).bind(q.region_id).bind(&search)
            .bind(page_size).bind(offset)
            .fetch_all(pool).await?;
        (total, rows)
    };
    Ok(Page::new(rows, total, page, page_size))
}

pub async fn get_object_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<ObjectView>> {
    Ok(sqlx::query_as("SELECT * FROM v_objects WHERE id = $1")
        .bind(id).fetch_optional(pool).await?)
}

pub async fn get_object_by_station_id(pool: &PgPool, sid: &str) -> AppResult<Option<ObjectView>> {
    Ok(sqlx::query_as("SELECT * FROM v_objects WHERE station_id = $1")
        .bind(sid).fetch_optional(pool).await?)
}

pub async fn create_object(pool: &PgPool, req: &CreateObjectRequest, by: Option<Uuid>) -> AppResult<ObjectView> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO objects (station_id, name, short_name, region_id, station_type_id,
             latitude, longitude, location_name, allowed_radius_m, description, notes,
             datalogger_url, datalogger_user, datalogger_pass,
             poll_interval_sec, polling_enabled, commissioned_at, created_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) RETURNING id")
        .bind(&req.station_id).bind(&req.name).bind(&req.short_name)
        .bind(req.region_id).bind(req.station_type_id)
        .bind(req.latitude).bind(req.longitude).bind(&req.location_name)
        .bind(req.allowed_radius_m.unwrap_or(0))
        .bind(&req.description).bind(&req.notes)
        .bind(&req.datalogger_url).bind(&req.datalogger_user).bind(&req.datalogger_pass)
        .bind(req.poll_interval_sec.unwrap_or(60))
        .bind(req.polling_enabled.unwrap_or(false))
        .bind(req.commissioned_at).bind(by)
        .fetch_one(pool).await?;

    get_object_by_id(pool, id).await?
        .ok_or_else(|| crate::errors::AppError::NotFound("Object not found after insert".into()))
}

pub async fn update_object(pool: &PgPool, id: Uuid, req: &UpdateObjectRequest) -> AppResult<ObjectView> {
    sqlx::query(
        "UPDATE objects SET
             name              = COALESCE($2,  name),
             short_name        = COALESCE($3,  short_name),
             region_id         = COALESCE($4,  region_id),
             station_type_id   = COALESCE($5,  station_type_id),
             latitude          = COALESCE($6,  latitude),
             longitude         = COALESCE($7,  longitude),
             location_name     = COALESCE($8,  location_name),
             allowed_radius_m  = COALESCE($9,  allowed_radius_m),
             description       = COALESCE($10, description),
             notes             = COALESCE($11, notes),
             datalogger_url    = COALESCE($12, datalogger_url),
             datalogger_user   = COALESCE($13, datalogger_user),
             datalogger_pass   = COALESCE($14, datalogger_pass),
             poll_interval_sec = COALESCE($15, poll_interval_sec),
             polling_enabled   = COALESCE($16, polling_enabled),
             is_active         = COALESCE($17, is_active),
             program_version   = COALESCE($18, program_version),
             program_features  = COALESCE($19, program_features)
         WHERE id = $1")
        .bind(id).bind(&req.name).bind(&req.short_name).bind(req.region_id)
        .bind(req.station_type_id).bind(req.latitude).bind(req.longitude)
        .bind(&req.location_name).bind(req.allowed_radius_m).bind(&req.description).bind(&req.notes)
        .bind(&req.datalogger_url).bind(&req.datalogger_user).bind(&req.datalogger_pass)
        .bind(req.poll_interval_sec).bind(req.polling_enabled).bind(req.is_active)
        .bind(&req.program_version).bind(&req.program_features)
        .execute(pool).await?;

    get_object_by_id(pool, id).await?
        .ok_or_else(|| crate::errors::AppError::NotFound(format!("Object {} not found", id)))
}

pub async fn soft_delete_object(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE objects SET is_active = FALSE WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(())
}

// ================================================================
// OBJECT POLL CONFIGS (interní — za poller, čita direktno iz objects tablice)
// ================================================================

pub async fn get_object_poll_config(pool: &PgPool, id: Uuid) -> AppResult<Option<ObjectPollConfig>> {
    Ok(sqlx::query_as(
        "SELECT id, station_id, datalogger_url, datalogger_user, datalogger_pass,
                poll_interval_sec, polling_enabled
         FROM objects WHERE id = $1 AND is_active = TRUE")
        .bind(id)
        .fetch_optional(pool).await?)
}

pub async fn list_pollable_objects(pool: &PgPool) -> AppResult<Vec<ObjectPollConfig>> {
    Ok(sqlx::query_as(
        "SELECT id, station_id, datalogger_url, datalogger_user, datalogger_pass,
                poll_interval_sec, polling_enabled
         FROM objects
         WHERE is_active = TRUE AND polling_enabled = TRUE AND datalogger_url IS NOT NULL
         ORDER BY station_id")
        .fetch_all(pool).await?)
}

// ================================================================
// IMAGES
// ================================================================

pub async fn list_object_images(pool: &PgPool, object_id: Uuid) -> AppResult<Vec<ObjectImage>> {
    Ok(sqlx::query_as!(ObjectImage,
        "SELECT * FROM object_images WHERE object_id = $1 ORDER BY is_primary DESC, uploaded_at DESC",
        object_id).fetch_all(pool).await?)
}

pub async fn insert_image(pool: &PgPool, object_id: Uuid, filename: &str,
    original_name: Option<&str>, mime_type: &str, file_size: Option<i32>,
    storage_path: &str, storage_url: Option<&str>, is_primary: bool,
    caption: Option<&str>, taken_at: Option<chrono::NaiveDate>, uploaded_by: Option<Uuid>,
) -> AppResult<ObjectImage> {
    if is_primary {
        sqlx::query("UPDATE object_images SET is_primary = FALSE WHERE object_id = $1")
            .bind(object_id).execute(pool).await?;
    }
    Ok(sqlx::query_as!(ObjectImage,
        "INSERT INTO object_images (object_id, filename, original_name, mime_type,
             file_size_bytes, storage_path, storage_url, is_primary, caption, taken_at, uploaded_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *",
        object_id, filename, original_name, mime_type, file_size,
        storage_path, storage_url, is_primary, caption, taken_at, uploaded_by)
        .fetch_one(pool).await?)
}

pub async fn delete_image(pool: &PgPool, image_id: Uuid, object_id: Uuid) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "DELETE FROM object_images WHERE id = $1 AND object_id = $2 RETURNING storage_path")
        .bind(image_id).bind(object_id).fetch_optional(pool).await?)
}

// ================================================================
// MEASUREMENTS
// ================================================================

pub async fn insert_measurement_10min(pool: &PgPool, r: &Measurement10minInsert) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO measurements_10min (object_id, station_id, recorded_at,
             datalogger_temp_avg, battery_voltage_avg, battery_current_avg,
             battery_status_smp, battery_status_avg, solar_voltage_avg,
             solar_daylight_smp, solar_daylight_avg, modem_power_avg, internet_ok_avg,
             garmin_comm_ok_avg, garmin_satellites_avg, garmin_latitude_avg,
             garmin_longitude_avg, garmin_distance_avg, lantern_comm_ok_avg,
             lantern_light_active_avg, lantern_current_active_avg, lantern_current_avg,
             lantern_latitude_avg, lantern_longitude_avg, lantern_distance_avg,
             visibility_comm_ok_avg, visibility_value_avg, visibility_alarm_avg,
             visibility_error_smp, fog_signal_active_avg, fog_signal_current_avg)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
                 $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31)
         RETURNING id")
        .bind(r.object_id).bind(&r.station_id).bind(r.recorded_at)
        .bind(r.datalogger_temp_avg).bind(r.battery_voltage_avg).bind(r.battery_current_avg)
        .bind(r.battery_status_smp).bind(r.battery_status_avg).bind(r.solar_voltage_avg)
        .bind(r.solar_daylight_smp).bind(r.solar_daylight_avg).bind(r.modem_power_avg)
        .bind(r.internet_ok_avg).bind(r.garmin_comm_ok_avg).bind(r.garmin_satellites_avg)
        .bind(r.garmin_latitude_avg).bind(r.garmin_longitude_avg).bind(r.garmin_distance_avg)
        .bind(r.lantern_comm_ok_avg).bind(r.lantern_light_active_avg).bind(r.lantern_current_active_avg)
        .bind(r.lantern_current_avg).bind(r.lantern_latitude_avg).bind(r.lantern_longitude_avg)
        .bind(r.lantern_distance_avg).bind(r.visibility_comm_ok_avg).bind(r.visibility_value_avg)
        .bind(r.visibility_alarm_avg).bind(r.visibility_error_smp).bind(r.fog_signal_active_avg)
        .bind(r.fog_signal_current_avg)
        .fetch_one(pool).await?)
}

pub async fn insert_measurement_1h(pool: &PgPool, r: &Measurement1hInsert) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO measurements_1h (object_id, station_id, recorded_at,
             datalogger_temp_avg, battery_voltage_avg, battery_current_avg,
             battery_charge_tot, battery_discharge_tot, battery_status_avg,
             solar_voltage_avg, solar_daylight_avg, modem_power_avg, internet_ok_avg,
             lantern_light_active_avg, lantern_current_avg,
             visibility_value_avg, visibility_alarm_avg, fog_signal_current_avg)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) RETURNING id")
        .bind(r.object_id).bind(&r.station_id).bind(r.recorded_at)
        .bind(r.datalogger_temp_avg).bind(r.battery_voltage_avg).bind(r.battery_current_avg)
        .bind(r.battery_charge_tot).bind(r.battery_discharge_tot).bind(r.battery_status_avg)
        .bind(r.solar_voltage_avg).bind(r.solar_daylight_avg).bind(r.modem_power_avg)
        .bind(r.internet_ok_avg).bind(r.lantern_light_active_avg).bind(r.lantern_current_avg)
        .bind(r.visibility_value_avg).bind(r.visibility_alarm_avg).bind(r.fog_signal_current_avg)
        .fetch_one(pool).await?)
}

pub async fn insert_measurement_24h(pool: &PgPool, r: &Measurement24hInsert) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO measurements_24h (object_id, station_id, recorded_at,
             datalogger_temp_avg, battery_voltage_avg, battery_current_avg,
             battery_current_min, battery_current_max, battery_charge_tot,
             battery_discharge_tot, battery_status_avg, solar_daylight_avg,
             modem_power_avg, internet_ok_avg, lantern_light_active_avg, lantern_current_avg,
             visibility_value_avg, fog_signal_current_avg)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) RETURNING id")
        .bind(r.object_id).bind(&r.station_id).bind(r.recorded_at)
        .bind(r.datalogger_temp_avg).bind(r.battery_voltage_avg).bind(r.battery_current_avg)
        .bind(r.battery_current_min).bind(r.battery_current_max).bind(r.battery_charge_tot)
        .bind(r.battery_discharge_tot).bind(r.battery_status_avg).bind(r.solar_daylight_avg)
        .bind(r.modem_power_avg).bind(r.internet_ok_avg).bind(r.lantern_light_active_avg)
        .bind(r.lantern_current_avg).bind(r.visibility_value_avg).bind(r.fog_signal_current_avg)
        .fetch_one(pool).await?)
}

pub async fn get_measurements_10min(
    pool: &PgPool, object_id: Uuid, q: &TimeRangeQuery,
) -> AppResult<Vec<Measurement10min>> {
    let limit = q.limit.unwrap_or(144).min(1000);
    Ok(sqlx::query_as(
        "SELECT * FROM measurements_10min
         WHERE object_id = $1
           AND ($2::timestamptz IS NULL OR recorded_at >= $2)
           AND ($3::timestamptz IS NULL OR recorded_at <= $3)
         ORDER BY recorded_at DESC LIMIT $4")
        .bind(object_id).bind(q.from).bind(q.to).bind(limit)
        .fetch_all(pool).await?)
}

pub async fn get_measurements_1h(
    pool: &PgPool, object_id: Uuid, q: &TimeRangeQuery,
) -> AppResult<Vec<Measurement1h>> {
    let limit = q.limit.unwrap_or(168).min(1000);
    Ok(sqlx::query_as(
        "SELECT * FROM measurements_1h
         WHERE object_id = $1
           AND ($2::timestamptz IS NULL OR recorded_at >= $2)
           AND ($3::timestamptz IS NULL OR recorded_at <= $3)
         ORDER BY recorded_at DESC LIMIT $4")
        .bind(object_id).bind(q.from).bind(q.to).bind(limit)
        .fetch_all(pool).await?)
}

pub async fn get_measurements_24h(
    pool: &PgPool, object_id: Uuid, q: &TimeRangeQuery,
) -> AppResult<Vec<Measurement24h>> {
    let limit = q.limit.unwrap_or(30).min(365);
    Ok(sqlx::query_as(
        "SELECT * FROM measurements_24h
         WHERE object_id = $1
           AND ($2::timestamptz IS NULL OR recorded_at >= $2)
           AND ($3::timestamptz IS NULL OR recorded_at <= $3)
         ORDER BY recorded_at DESC LIMIT $4")
        .bind(object_id).bind(q.from).bind(q.to).bind(limit)
        .fetch_all(pool).await?)
}

pub async fn get_latest_measurement(pool: &PgPool, object_id: Uuid) -> AppResult<Option<LatestMeasurement>> {
    Ok(sqlx::query_as(
        "SELECT * FROM v_latest_measurements WHERE object_id = $1")
        .bind(object_id)
        .fetch_optional(pool).await?)
}

// ================================================================
// ALARMS
// ================================================================

pub async fn insert_alarm(pool: &PgPool, r: &AlarmInsert) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO alarms (object_id, station_id, recorded_at,
             alarm_datalogger_high_temp, alarm_datalogger_high_voltage, alarm_datalogger_other_error,
             alarm_battery_voltage_low, alarm_battery_voltage_flat, alarm_battery_other_error,
             alarm_garmin_comm_failed, alarm_garmin_other_error, alarm_station_out_of_radius,
             alarm_lantern_night_light_off, alarm_lantern_day_light_on,
             alarm_lantern_comm_failed, alarm_lantern_other_error,
             alarm_modem_network_error, alarm_modem_other_error, alarm_station_other_error,
             alarm_visibility_comm_failed, alarm_visibility_error,
             alarm_fog_signal_off_during_fog, alarm_fog_signal_on_while_no_fog)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)
         RETURNING id")
        .bind(r.object_id).bind(&r.station_id).bind(r.recorded_at)
        .bind(r.alarm_datalogger_high_temp).bind(r.alarm_datalogger_high_voltage).bind(r.alarm_datalogger_other_error)
        .bind(r.alarm_battery_voltage_low).bind(r.alarm_battery_voltage_flat).bind(r.alarm_battery_other_error)
        .bind(r.alarm_garmin_comm_failed).bind(r.alarm_garmin_other_error).bind(r.alarm_station_out_of_radius)
        .bind(r.alarm_lantern_night_light_off).bind(r.alarm_lantern_day_light_on)
        .bind(r.alarm_lantern_comm_failed).bind(r.alarm_lantern_other_error)
        .bind(r.alarm_modem_network_error).bind(r.alarm_modem_other_error).bind(r.alarm_station_other_error)
        .bind(r.alarm_visibility_comm_failed).bind(r.alarm_visibility_error)
        .bind(r.alarm_fog_signal_off_during_fog).bind(r.alarm_fog_signal_on_while_no_fog)
        .fetch_one(pool).await?)
}

pub async fn get_alarms(
    pool: &PgPool, object_id: Uuid, q: &TimeRangeQuery,
) -> AppResult<Vec<AlarmRecord>> {
    let limit = q.limit.unwrap_or(100).min(1000);
    Ok(sqlx::query_as(
        "SELECT * FROM alarms
         WHERE object_id = $1
           AND ($2::timestamptz IS NULL OR recorded_at >= $2)
           AND ($3::timestamptz IS NULL OR recorded_at <= $3)
         ORDER BY recorded_at DESC LIMIT $4")
        .bind(object_id).bind(q.from).bind(q.to).bind(limit)
        .fetch_all(pool).await?)
}

pub async fn get_active_alarms(pool: &PgPool, object_id: Uuid) -> AppResult<Vec<AlarmRecord>> {
    Ok(sqlx::query_as(
        "SELECT * FROM alarms WHERE object_id = $1 AND any_alarm_active = TRUE
         ORDER BY recorded_at DESC LIMIT 50")
        .bind(object_id)
        .fetch_all(pool).await?)
}

/// Potvrdi alarm: označava alarm zapise kao potvrđene i resetira cached stanje
pub async fn acknowledge_object_alarm(pool: &PgPool, object_id: Uuid, by: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE alarms SET acknowledged_at = NOW(), acknowledged_by = $2
         WHERE object_id = $1 AND acknowledged_at IS NULL AND any_alarm_active = TRUE")
        .bind(object_id).bind(by).execute(pool).await?;
    sqlx::query(
        "UPDATE objects SET
            alarm_active = FALSE,
            alarm_count = 0,
            alarm_worst_level = NULL,
            alarm_summary = NULL,
            alarm_last_seen_at = NULL
         WHERE id = $1")
        .bind(object_id).execute(pool).await?;
    Ok(())
}

/// Globalni popis alarm zapisa s filtrima po regiji i statusu — bez duplikata
pub async fn list_alarms_global(pool: &PgPool, q: &AlarmListQuery) -> AppResult<Page<AlarmListItem>> {
    let page      = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(50).min(200);
    let offset    = (page - 1) * page_size;
    let status    = q.status.as_deref().unwrap_or("active");

    // DISTINCT ON (object_id) = jedan zapis po objektu (najnoviji)
    // Za potvrđene: DISTINCT ON (object_id, datum potvrde)
    let (distinct_clause, where_status, order_inner, order_outer) = match status {
        "acknowledged" =>
            ("DISTINCT ON (o.id, acknowledged_at::date)",
             "a.acknowledged_at IS NOT NULL",
             "o.id, acknowledged_at::date DESC, a.recorded_at DESC",
             "sub.acknowledged_at DESC NULLS LAST"),
        "all" =>
            ("DISTINCT ON (o.id)",
             "a.any_alarm_active = TRUE",
             "o.id, a.recorded_at DESC",
             "sub.recorded_at DESC"),
        _ =>  // "active"
            ("DISTINCT ON (o.id)",
             "a.any_alarm_active = TRUE AND a.acknowledged_at IS NULL",
             "o.id, a.recorded_at DESC",
             "sub.recorded_at DESC"),
    };

    let cols = "a.id,
            o.id          AS object_id,
            o.name        AS object_name,
            a.station_id,
            r.id          AS region_id,
            r.name        AS region_name,
            r.code        AS region_code,
            r.color       AS region_color,
            o.location_name,
            a.recorded_at,
            a.acknowledged_at,
            a.acknowledged_by,
            a.any_alarm_active,
            a.alarm_datalogger_high_temp,
            a.alarm_datalogger_high_voltage,
            a.alarm_datalogger_other_error,
            a.alarm_battery_voltage_low,
            a.alarm_battery_voltage_flat,
            a.alarm_battery_other_error,
            a.alarm_garmin_comm_failed,
            a.alarm_garmin_other_error,
            a.alarm_station_out_of_radius,
            a.alarm_lantern_night_light_off,
            a.alarm_lantern_day_light_on,
            a.alarm_lantern_comm_failed,
            a.alarm_lantern_other_error,
            a.alarm_modem_network_error,
            a.alarm_modem_other_error,
            a.alarm_station_other_error,
            a.alarm_visibility_comm_failed,
            a.alarm_visibility_error,
            a.alarm_fog_signal_off_during_fog,
            a.alarm_fog_signal_on_while_no_fog";

    let from_join = "FROM alarms a
         JOIN objects o ON o.id = a.object_id
         JOIN regions r ON r.id = o.region_id";

    let sql = format!(
        "SELECT sub.* FROM (
           SELECT {distinct_clause} {cols}
           {from_join}
           WHERE {where_status}
             AND ($1::uuid IS NULL OR r.id = $1)
           ORDER BY {order_inner}
         ) sub
         ORDER BY {order_outer}
         LIMIT $2 OFFSET $3");

    let count_sql = format!(
        "SELECT COUNT(*) FROM (
           SELECT {distinct_clause} a.id
           {from_join}
           WHERE {where_status}
             AND ($1::uuid IS NULL OR r.id = $1)
           ORDER BY {order_inner}
         ) sub");

    let rows: Vec<AlarmListItem> = sqlx::query_as(&sql)
        .bind(q.region_id).bind(page_size).bind(offset)
        .fetch_all(pool).await?;

    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(q.region_id)
        .fetch_one(pool).await?;

    Ok(Page::new(rows, total, page, page_size))
}

/// Briši jedan alarm zapis po ID-u
pub async fn delete_alarm_by_id(pool: &PgPool, alarm_id: i64) -> AppResult<Option<Uuid>> {
    // Vrati object_id da bi mogli ažurirati cached stanje
    let object_id: Option<Uuid> = sqlx::query_scalar(
        "DELETE FROM alarms WHERE id = $1 RETURNING object_id")
        .bind(alarm_id).fetch_optional(pool).await?;

    if let Some(oid) = object_id {
        // Rekalkuliraj cached alarm stanje na objektu
        let still_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM alarms WHERE object_id = $1 AND any_alarm_active = TRUE AND acknowledged_at IS NULL)")
            .bind(oid).fetch_one(pool).await?;

        if !still_active {
            sqlx::query(
                "UPDATE objects SET alarm_active = FALSE, alarm_count = 0,
                    alarm_worst_level = NULL, alarm_summary = NULL, alarm_last_seen_at = NULL
                 WHERE id = $1")
                .bind(oid).execute(pool).await?;
        }
    }
    Ok(object_id)
}

/// Briši sve alarm zapise za objekt i resetira cached stanje
pub async fn clear_object_alarms(pool: &PgPool, object_id: Uuid) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM alarms WHERE object_id = $1")
        .bind(object_id).execute(pool).await?;
    sqlx::query(
        "UPDATE objects SET
            alarm_active = FALSE,
            alarm_count = 0,
            alarm_worst_level = NULL,
            alarm_summary = NULL,
            alarm_last_seen_at = NULL
         WHERE id = $1")
        .bind(object_id).execute(pool).await?;
    Ok(result.rows_affected())
}

// ================================================================
// EVENT LOGS
// ================================================================

pub async fn insert_event_log(pool: &PgPool, r: &EventLogInsert) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO event_logs (object_id, station_id, recorded_at, log_level, log_message)
         VALUES ($1, $2, $3, $4, $5) RETURNING id")
        .bind(r.object_id).bind(&r.station_id).bind(r.recorded_at)
        .bind(r.log_level).bind(&r.log_message)
        .fetch_one(pool).await?)
}

pub async fn get_event_logs(
    pool: &PgPool, object_id: Uuid, min_level: Option<i16>, q: &TimeRangeQuery,
) -> AppResult<Vec<EventLogRecord>> {
    let limit = q.limit.unwrap_or(200).min(1000);
    Ok(sqlx::query_as!(EventLogRecord,
        "SELECT * FROM event_logs
         WHERE object_id = $1
           AND ($2::smallint IS NULL OR log_level >= $2)
           AND ($3::timestamptz IS NULL OR recorded_at >= $3)
           AND ($4::timestamptz IS NULL OR recorded_at <= $4)
         ORDER BY recorded_at DESC LIMIT $5",
        object_id, min_level, q.from, q.to, limit)
        .fetch_all(pool).await?)
}

// ================================================================
// USERS
// ================================================================

pub async fn find_user_by_username(pool: &PgPool, username: &str) -> AppResult<Option<User>> {
    Ok(sqlx::query_as!(User,
        "SELECT * FROM users WHERE username = $1 AND is_active = TRUE", username)
        .fetch_optional(pool).await?)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<UserPublic>> {
    Ok(sqlx::query_as!(UserPublic,
        "SELECT id,username,email,full_name,role,is_active,last_login_at,created_at FROM users WHERE id=$1", id)
        .fetch_optional(pool).await?)
}

pub async fn list_users(pool: &PgPool) -> AppResult<Vec<UserPublic>> {
    Ok(sqlx::query_as!(UserPublic,
        "SELECT id,username,email,full_name,role,is_active,last_login_at,created_at FROM users ORDER BY username")
        .fetch_all(pool).await?)
}

pub async fn create_user(pool: &PgPool, req: &CreateUserRequest, hash: &str, by: Option<Uuid>) -> AppResult<UserPublic> {
    if !["admin","operator","viewer"].contains(&req.role.as_str()) {
        return Err(crate::errors::AppError::Validation("Role must be admin, operator, or viewer".into()));
    }
    Ok(sqlx::query_as!(UserPublic,
        "INSERT INTO users (username, email, password_hash, full_name, role, created_by)
         VALUES ($1,$2,$3,$4,$5,$6)
         RETURNING id,username,email,full_name,role,is_active,last_login_at,created_at",
        req.username, req.email, hash, req.full_name, req.role, by)
        .fetch_one(pool).await?)
}

pub async fn update_last_login(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(())
}

// ================================================================
// USER REGION ACCESS
// ================================================================

pub async fn list_user_region_access(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<UserRegionAccessView>> {
    Ok(sqlx::query_as!(UserRegionAccessView,
        r#"SELECT ura.id, ura.user_id, ura.region_id,
                  r.name AS region_name, r.code AS region_code, r.color AS region_color,
                  ura.permission, ura.granted_at
           FROM user_region_access ura
           JOIN regions r ON ura.region_id = r.id
           WHERE ura.user_id = $1 ORDER BY r.name"#, user_id)
        .fetch_all(pool).await?)
}

pub async fn grant_region_access(pool: &PgPool, req: &GrantRegionAccessRequest, by: Uuid) -> AppResult<UserRegionAccess> {
    if !["operator","viewer"].contains(&req.permission.as_str()) {
        return Err(crate::errors::AppError::Validation("Permission must be operator or viewer".into()));
    }
    Ok(sqlx::query_as!(UserRegionAccess,
        "INSERT INTO user_region_access (user_id, region_id, permission, granted_by)
         VALUES ($1,$2,$3,$4)
         ON CONFLICT (user_id, region_id) DO UPDATE SET
             permission = EXCLUDED.permission, granted_by = EXCLUDED.granted_by, granted_at = NOW()
         RETURNING *",
        req.user_id, req.region_id, req.permission, by)
        .fetch_one(pool).await?)
}

pub async fn revoke_region_access(pool: &PgPool, user_id: Uuid, region_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM user_region_access WHERE user_id=$1 AND region_id=$2")
        .bind(user_id).bind(region_id).execute(pool).await?;
    Ok(())
}

pub async fn user_can_access_region(pool: &PgPool, user_id: Uuid, role: &str, region_id: Uuid) -> AppResult<bool> {
    if role == "admin" { return Ok(true); }
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_region_access WHERE user_id=$1 AND region_id=$2)")
        .bind(user_id).bind(region_id).fetch_one(pool).await?)
}

pub async fn user_can_control_in_region(pool: &PgPool, user_id: Uuid, role: &str, region_id: Uuid) -> AppResult<bool> {
    if role == "admin" { return Ok(true); }
    if role == "viewer" { return Ok(false); }
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_region_access WHERE user_id=$1 AND region_id=$2 AND permission='operator')")
        .bind(user_id).bind(region_id).fetch_one(pool).await?)
}

// ================================================================
// REFRESH TOKENS
// ================================================================

pub async fn store_refresh_token(pool: &PgPool, user_id: Uuid, hash: &str,
    expires_at: chrono::DateTime<Utc>, ip: Option<&str>, ua: Option<&str>) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at, ip_address, user_agent)
         VALUES ($1,$2,$3,$4::inet,$5)")
        .bind(user_id).bind(hash).bind(expires_at).bind(ip).bind(ua)
        .execute(pool).await?;
    Ok(())
}

pub async fn validate_refresh_token(pool: &PgPool, hash: &str) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM refresh_tokens WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at > NOW()")
        .bind(hash).fetch_optional(pool).await?)
}

pub async fn revoke_refresh_token(pool: &PgPool, hash: &str) -> AppResult<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1")
        .bind(hash).execute(pool).await?;
    Ok(())
}

// ================================================================
// AUDIT LOG
// ================================================================

pub async fn write_audit(pool: &PgPool, user_id: Option<Uuid>, username: Option<&str>,
    action: &str, entity_type: Option<&str>, entity_id: Option<&str>,
    details: Option<serde_json::Value>, ip: Option<&str>) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO audit_log (user_id, username, action, entity_type, entity_id, details, ip_address)
         VALUES ($1,$2,$3,$4,$5,$6,$7::inet)")
        .bind(user_id).bind(username).bind(action)
        .bind(entity_type).bind(entity_id).bind(details).bind(ip)
        .execute(pool).await?;
    Ok(())
}
