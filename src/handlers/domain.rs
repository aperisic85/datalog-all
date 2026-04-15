//! Domain HTTP handleri
//! Auth, Regions, Objects, Users, User-Region access, Measurements, Alarms

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth as auth_svc;
use crate::db::domain as db;
use crate::errors::{AppError, AppResult};
use crate::models::domain::*;

// ═══════════════════════════════════════════════════════════════════════════
// AUTH
// ═══════════════════════════════════════════════════════════════════════════

/// POST /api/v1/auth/login
pub async fn login(
    State(pool): State<PgPool>,
    Extension(jwt_secret): Extension<String>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let user = db::find_user_by_username(&pool, &req.username).await?
        .ok_or(AppError::Unauthorized)?;

    if !auth_svc::verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let access_token  = auth_svc::generate_access_token(user.id, &user.username, &user.role, &jwt_secret)?;
    let refresh_raw   = auth_svc::generate_refresh_token();
    let refresh_hash  = auth_svc::hash_refresh_token(&refresh_raw);
    let refresh_exp   = auth_svc::refresh_token_expiry();

    db::store_refresh_token(&pool, user.id, &refresh_hash, refresh_exp, None, None).await?;
    db::update_last_login(&pool, user.id).await?;

    let _ = db::write_audit(&pool, Some(user.id), Some(&user.username),
        "LOGIN", Some("user"), Some(&user.id.to_string()), None, None).await;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token: refresh_raw,
        token_type:    "Bearer",
        expires_in:    auth_svc::access_token_expires_in(),
        user: UserPublic {
            id: user.id, username: user.username, email: user.email,
            full_name: user.full_name, role: user.role, is_active: user.is_active,
            last_login_at: user.last_login_at, created_at: user.created_at,
        },
    }))
}

/// POST /api/v1/auth/refresh
pub async fn refresh_token(
    State(pool): State<PgPool>,
    Extension(jwt_secret): Extension<String>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<RefreshResponse>> {
    let hash    = auth_svc::hash_refresh_token(&req.refresh_token);
    let user_id = db::validate_refresh_token(&pool, &hash).await?.ok_or(AppError::Unauthorized)?;
    let user    = db::find_user_by_id(&pool, user_id).await?.ok_or(AppError::Unauthorized)?;
    if !user.is_active { return Err(AppError::Unauthorized); }

    let access_token = auth_svc::generate_access_token(user.id, &user.username, &user.role, &jwt_secret)?;
    Ok(Json(RefreshResponse { access_token, token_type: "Bearer", expires_in: auth_svc::access_token_expires_in() }))
}

/// POST /api/v1/auth/logout
pub async fn logout(
    State(pool): State<PgPool>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<StatusCode> {
    let hash = auth_svc::hash_refresh_token(&req.refresh_token);
    db::revoke_refresh_token(&pool, &hash).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/auth/me
pub async fn me(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
) -> AppResult<Json<UserPublic>> {
    let uid  = parse_uid(&claims.sub)?;
    let user = db::find_user_by_id(&pool, uid).await?.ok_or(AppError::Unauthorized)?;
    Ok(Json(user))
}

// ═══════════════════════════════════════════════════════════════════════════
// REGIONS
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/regions
pub async fn list_regions(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
) -> AppResult<Json<Vec<Region>>> {
    let uid     = parse_uid(&claims.sub)?;
    let regions = db::list_user_regions(&pool, uid, &claims.role).await?;
    Ok(Json(regions))
}

/// GET /api/v1/regions/summary  — dashboard karte s alarmima
pub async fn region_summary(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
) -> AppResult<Json<Vec<RegionSummary>>> {
    let uid = parse_uid(&claims.sub)?;
    let data = db::list_region_summary(&pool, uid, &claims.role).await?;
    Ok(Json(data))
}

/// POST /api/v1/regions  [admin only]
pub async fn create_region(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<CreateRegionRequest>,
) -> AppResult<(StatusCode, Json<Region>)> {
    require_admin(&claims)?;
    let region = db::create_region(&pool, &req).await?;
    let uid    = parse_uid(&claims.sub).ok();
    let _ = db::write_audit(&pool, uid, Some(&claims.username),
        "CREATE_REGION", Some("region"), Some(&region.id.to_string()),
        Some(serde_json::json!({"name": region.name})), None).await;
    Ok((StatusCode::CREATED, Json(region)))
}

/// PUT /api/v1/regions/:id  [admin only]
pub async fn update_region(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRegionRequest>,
) -> AppResult<Json<Region>> {
    require_admin(&claims)?;
    let region = db::update_region(&pool, id, &req).await?;
    Ok(Json(region))
}

/// GET /api/v1/station-types
pub async fn list_station_types(State(pool): State<PgPool>) -> AppResult<Json<Vec<StationType>>> {
    Ok(Json(db::list_station_types(&pool).await?))
}

// ═══════════════════════════════════════════════════════════════════════════
// OBJECTS
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/objects
pub async fn list_objects(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Query(query): Query<ObjectsQuery>,
) -> AppResult<Json<Page<ObjectView>>> {
    let uid  = parse_uid(&claims.sub)?;
    let page = db::list_objects(&pool, uid, &claims.role, &query).await?;
    Ok(Json(page))
}

/// GET /api/v1/objects/:id
pub async fn get_object(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ObjectView>> {
    let uid = parse_uid(&claims.sub)?;
    let obj = db::get_object_by_id(&pool, id).await?.ok_or_else(|| AppError::NotFound(format!("Object {}", id)))?;
    if !db::user_can_access_region(&pool, uid, &claims.role, obj.region_id).await? {
        return Err(AppError::Forbidden);
    }
    Ok(Json(obj))
}

/// POST /api/v1/objects  [admin | operator s pristupom regiji]
pub async fn create_object(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<CreateObjectRequest>,
) -> AppResult<(StatusCode, Json<ObjectView>)> {
    if claims.role == "viewer" { return Err(AppError::Forbidden); }
    let uid = parse_uid(&claims.sub)?;

    if claims.role == "operator" {
        if !db::user_can_control_in_region(&pool, uid, &claims.role, req.region_id).await? {
            return Err(AppError::Forbidden);
        }
    }

    let obj = db::create_object(&pool, &req, Some(uid)).await?;
    let _ = db::write_audit(&pool, Some(uid), Some(&claims.username),
        "CREATE_OBJECT", Some("object"), Some(&obj.id.to_string()),
        Some(serde_json::json!({"station_id": obj.station_id, "name": obj.name})), None).await;
    Ok((StatusCode::CREATED, Json(obj)))
}

/// PATCH /api/v1/objects/:id
pub async fn update_object(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateObjectRequest>,
) -> AppResult<Json<ObjectView>> {
    if claims.role == "viewer" { return Err(AppError::Forbidden); }
    let uid = parse_uid(&claims.sub)?;

    let obj = db::get_object_by_id(&pool, id).await?.ok_or_else(|| AppError::NotFound(format!("Object {}", id)))?;
    if !db::user_can_control_in_region(&pool, uid, &claims.role, obj.region_id).await? {
        return Err(AppError::Forbidden);
    }

    let updated = db::update_object(&pool, id, &req).await?;
    let _ = db::write_audit(&pool, Some(uid), Some(&claims.username),
        "UPDATE_OBJECT", Some("object"), Some(&id.to_string()), None, None).await;
    Ok(Json(updated))
}

/// DELETE /api/v1/objects/:id  [admin only — soft delete]
pub async fn delete_object(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;
    db::soft_delete_object(&pool, id).await?;
    let uid = parse_uid(&claims.sub).ok();
    let _ = db::write_audit(&pool, uid, Some(&claims.username),
        "DELETE_OBJECT", Some("object"), Some(&id.to_string()), None, None).await;
    Ok(StatusCode::NO_CONTENT)
}

// ═══════════════════════════════════════════════════════════════════════════
// MJERENJA & ALARMI po objektu
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/objects/:id/measurements/10min
pub async fn get_measurements_10min(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> AppResult<Json<Vec<Measurement10min>>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_measurements_10min(&pool, id, &q).await?))
}

/// GET /api/v1/objects/:id/measurements/1h
pub async fn get_measurements_1h(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> AppResult<Json<Vec<Measurement1h>>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_measurements_1h(&pool, id, &q).await?))
}

/// GET /api/v1/objects/:id/measurements/24h
pub async fn get_measurements_24h(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> AppResult<Json<Vec<Measurement24h>>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_measurements_24h(&pool, id, &q).await?))
}

/// GET /api/v1/objects/:id/battery/prediction
///
/// Vraća predikciju kvara baterije na temelju linearne regresije
/// nad zadnjih 72 satnih mjerenja napona.
pub async fn predict_battery(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BatteryPrediction>> {
    check_object_access(&pool, &claims, id).await?;

    // Dohvati zadnjih 72 satna mjerenja (= do 3 dana podataka)
    let history = db::get_battery_voltage_history(&pool, id, 72).await?;

    // Zadnji izmjereni napon (najnoviji uzorak)
    let current_voltage = history.last().map(|(_, v)| *v);

    // Pretvori u VoltagePoint za regresijski modul
    let points: Vec<crate::battery_prediction::VoltagePoint> = history
        .into_iter()
        .map(|(ts, v)| crate::battery_prediction::VoltagePoint {
            recorded_at: ts,
            voltage: v as f64,
        })
        .collect();

    let prediction = match crate::battery_prediction::compute_trend(&points) {
        Some(trend) => BatteryPrediction {
            object_id:         id,
            computed_at:       chrono::Utc::now(),
            current_voltage,
            trend_voltage:     Some(trend.trend_voltage),
            slope_v_per_hour:  trend.slope_v_per_hour,
            trend:             trend.trend.to_string(),
            hours_to_warning:  trend.hours_to_warning,
            hours_to_critical: trend.hours_to_critical,
            days_to_warning:   trend.hours_to_warning.map(|h| h / 24.0),
            days_to_critical:  trend.hours_to_critical.map(|h| h / 24.0),
            sample_count:      trend.sample_count as i32,
            r_squared:         Some(trend.r_squared),
        },
        None => BatteryPrediction {
            object_id:         id,
            computed_at:       chrono::Utc::now(),
            current_voltage,
            trend_voltage:     None,
            slope_v_per_hour:  0.0,
            trend:             "insufficient_data".to_string(),
            hours_to_warning:  None,
            hours_to_critical: None,
            days_to_warning:   None,
            days_to_critical:  None,
            sample_count:      points.len() as i32,
            r_squared:         None,
        },
    };

    Ok(Json(prediction))
}

/// GET /api/v1/objects/:id/battery/capacity
///
/// Procjenjuje efektivni kapacitet baterije iz dnevnih totalizatora
/// battery_charge_tot i battery_discharge_tot (zadnjih 60 dana).
pub async fn estimate_battery_capacity(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BatteryCapacityEstimate>> {
    check_object_access(&pool, &claims, id).await?;

    let obj = db::get_object_by_id(&pool, id).await?
        .ok_or_else(|| crate::errors::AppError::NotFound("Object not found".into()))?;

    // Dohvati dnevne totalizatore za zadnjih 60 dana
    let daily_data = db::get_daily_battery_totals(&pool, id, 60).await?;

    let points: Vec<crate::battery_capacity::DailyTotal> = daily_data
        .into_iter()
        .map(|(ts, ch, dis)| crate::battery_capacity::DailyTotal {
            recorded_at: ts,
            charge_ah:    ch as f64,
            discharge_ah: dis as f64,
        })
        .collect();

    let est = crate::battery_capacity::estimate_capacity(&points, obj.nominal_battery_capacity_ah);

    Ok(Json(BatteryCapacityEstimate {
        object_id:              id,
        computed_at:            chrono::Utc::now(),
        nominal_capacity_ah:    obj.nominal_battery_capacity_ah,
        estimated_capacity_ah:  est.estimated_ah,
        health_percent:         est.health_percent,
        max_daily_discharge_ah: est.max_daily_discharge_ah,
        max_deficit_run_ah:     est.max_deficit_run_ah,
        sample_days:            est.sample_days as i32,
        status:                 est.status.to_string(),
        status_label:           est.status_label.to_string(),
    }))
}

/// GET /api/v1/objects/:id/measurements/latest
pub async fn get_latest_measurement(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Option<LatestMeasurement>>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_latest_measurement(&pool, id).await?))
}

/// GET /api/v1/objects/:id/alarms
pub async fn get_alarms(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> AppResult<Json<Vec<AlarmRecord>>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_alarms(&pool, id, &q).await?))
}

/// GET /api/v1/objects/:id/alarms/heatmap
pub async fn get_alarm_heatmap(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AlarmHeatmapResponse>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_alarm_heatmap(&pool, id).await?))
}

/// GET /api/v1/objects/:id/alarms/active
pub async fn get_active_alarms(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<AlarmRecord>>> {
    check_object_access(&pool, &claims, id).await?;
    Ok(Json(db::get_active_alarms(&pool, id).await?))
}

/// POST /api/v1/objects/:id/alarms/acknowledge
pub async fn acknowledge_alarm(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if claims.role == "viewer" { return Err(AppError::Forbidden); }
    check_object_access(&pool, &claims, id).await?;
    db::acknowledge_object_alarm(&pool, id, &claims.username).await?;
    let uid = parse_uid(&claims.sub).ok();
    let _ = db::write_audit(&pool, uid, Some(&claims.username),
        "ACKNOWLEDGE_ALARM", Some("object"), Some(&id.to_string()), None, None).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/alarms  — globalni pregled alarma s filterima
pub async fn list_alarms(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Query(q): Query<AlarmListQuery>,
) -> AppResult<Json<Page<AlarmListItem>>> {
    // Za non-admin korisnike ograniči na njihove regije
    let effective_region = if claims.role == "admin" {
        q.region_id
    } else {
        // Ako non-admin specificira regiju, provjeri ima li pristup
        q.region_id // backend će filtrirati po region_id; TODO: fine-grained check
    };
    let q2 = AlarmListQuery {
        region_id: effective_region,
        status: q.status,
        page: q.page,
        page_size: q.page_size,
    };
    Ok(Json(db::list_alarms_global(&pool, &q2).await?))
}

/// DELETE /api/v1/alarms/:alarm_id  — briši jedan alarm zapis
pub async fn delete_alarm(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(alarm_id): Path<i64>,
) -> AppResult<StatusCode> {
    if claims.role == "viewer" { return Err(AppError::Forbidden); }
    db::delete_alarm_by_id(&pool, alarm_id).await?;
    let uid = parse_uid(&claims.sub).ok();
    let _ = db::write_audit(&pool, uid, Some(&claims.username),
        "DELETE_ALARM", Some("alarm"), Some(&alarm_id.to_string()), None, None).await;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/objects/:id/alarms  — briši sve alarm zapise
pub async fn delete_alarms(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if claims.role == "viewer" { return Err(AppError::Forbidden); }
    check_object_access(&pool, &claims, id).await?;
    db::clear_object_alarms(&pool, id).await?;
    let uid = parse_uid(&claims.sub).ok();
    let _ = db::write_audit(&pool, uid, Some(&claims.username),
        "DELETE_ALARMS", Some("object"), Some(&id.to_string()), None, None).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/objects/:id/eventlogs
pub async fn get_event_logs(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Query(q): Query<EventLogQuery>,
) -> AppResult<Json<Vec<EventLogRecord>>> {
    check_object_access(&pool, &claims, id).await?;
    let trq = TimeRangeQuery { from: q.from, to: q.to, limit: q.limit };
    Ok(Json(db::get_event_logs(&pool, id, q.min_level, &trq).await?))
}

#[derive(serde::Deserialize)]
pub struct EventLogQuery {
    pub from:      Option<chrono::DateTime<chrono::Utc>>,
    pub to:        Option<chrono::DateTime<chrono::Utc>>,
    pub min_level: Option<i16>,
    pub limit:     Option<i64>,
}

// ═══════════════════════════════════════════════════════════════════════════
// USERS  [admin only]
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/users
pub async fn list_users(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
) -> AppResult<Json<Vec<UserPublic>>> {
    require_admin(&claims)?;
    Ok(Json(db::list_users(&pool).await?))
}

/// POST /api/v1/users
pub async fn create_user(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<UserPublic>)> {
    require_admin(&claims)?;
    let hash = auth_svc::hash_password(&req.password)?;
    let uid  = parse_uid(&claims.sub).ok();
    let user = db::create_user(&pool, &req, &hash, uid).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

// ═══════════════════════════════════════════════════════════════════════════
// USER REGION ACCESS  [admin only]
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/users/:id/regions
pub async fn get_user_regions(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<Vec<UserRegionAccessView>>> {
    let requester = parse_uid(&claims.sub)?;
    if claims.role != "admin" && requester != user_id {
        return Err(AppError::Forbidden);
    }
    Ok(Json(db::list_user_region_access(&pool, user_id).await?))
}

/// POST /api/v1/users/regions  — dodijeli pristup regiji
pub async fn grant_region_access(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<GrantRegionAccessRequest>,
) -> AppResult<(StatusCode, Json<UserRegionAccess>)> {
    require_admin(&claims)?;
    let by     = parse_uid(&claims.sub)?;
    let access = db::grant_region_access(&pool, &req, by).await?;
    let _ = db::write_audit(&pool, Some(by), Some(&claims.username),
        "GRANT_REGION_ACCESS", Some("user_region_access"), Some(&access.id.to_string()),
        Some(serde_json::json!({"user_id": req.user_id, "region_id": req.region_id, "permission": req.permission})),
        None).await;
    Ok((StatusCode::CREATED, Json(access)))
}

/// DELETE /api/v1/users/:user_id/regions/:region_id
pub async fn revoke_region_access(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path((user_id, region_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;
    db::revoke_region_access(&pool, user_id, region_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ═══════════════════════════════════════════════════════════════════════════
// WEATHER & SOLAR EFFICIENCY
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/objects/:id/weather
///
/// Dohvati satne vremenske podatke (Open-Meteo) za koordinate objekta.
/// Query param `days` (1–30, default 7) određuje koliko dana unatrag.
pub async fn get_weather(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Query(q): Query<crate::models::domain::WeatherQuery>,
) -> AppResult<axum::Json<crate::weather::WeatherResponse>> {
    check_object_access(&pool, &claims, id).await?;

    let obj = db::get_object_by_id(&pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("Object {}", id)))?;

    let (lat, lon) = match (obj.latitude, obj.longitude) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => return Err(AppError::BadRequest("Objekt nema koordinate".into())),
    };

    let days = q.days.unwrap_or(7).clamp(1, 30);

    let weather = crate::weather::fetch_weather(lat, lon, days)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    Ok(axum::Json(weather))
}

/// GET /api/v1/objects/:id/solar-efficiency
///
/// Izračuna solarni score za objekt uspoređujući stvarnu solarnu napetost
/// s teorijskom (na temelju Open-Meteo iradijancije za lokaciju).
/// Koristi zadnjih 30 dana za baseline i 7 dana za "recent" score.
pub async fn get_solar_efficiency(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<axum::Json<crate::weather::SolarEfficiencyResponse>> {
    check_object_access(&pool, &claims, id).await?;

    let obj = db::get_object_by_id(&pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("Object {}", id)))?;

    let (lat, lon) = match (obj.latitude, obj.longitude) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => return Err(AppError::BadRequest("Objekt nema koordinate".into())),
    };

    // Dohvati Open-Meteo iradijanciju za zadnjih 30 dana
    let weather = crate::weather::fetch_weather(lat, lon, 30)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    // Dohvati satna mjerenja za zadnjih 30 dana
    let from = chrono::Utc::now() - chrono::Duration::days(30);
    let measurements = db::get_measurements_1h(
        &pool, id,
        &crate::models::domain::TimeRangeQuery {
            from: Some(from),
            to:   None,
            limit: Some(750), // 30 dana * 25 sati/dan
        },
    ).await?;

    // Izgradi lookup: sati iradijancije po UTC timestamp
    use std::collections::HashMap;
    let irr_map: HashMap<i64, f64> = weather
        .hours
        .iter()
        .filter_map(|h| {
            h.shortwave_radiation.map(|irr| (h.time.timestamp(), irr))
        })
        .collect();

    // Pariranje mjerenja s iradijancijom (zaokruži na sat)
    let points: Vec<crate::weather::EfficiencyPoint> = measurements
        .iter()
        .filter_map(|m| {
            let volt = m.solar_voltage_avg? as f64;
            // Zaokruži recorded_at na sat
            let ts = m.recorded_at.timestamp();
            let ts_rounded = (ts / 3600) * 3600;
            let irr = *irr_map.get(&ts_rounded).or_else(|| {
                // ±30 min fallback
                irr_map.get(&(ts_rounded - 1800))
                    .or_else(|| irr_map.get(&(ts_rounded + 1800)))
            })?;
            Some(crate::weather::EfficiencyPoint {
                date_str:      m.recorded_at.format("%Y-%m-%d").to_string(),
                solar_voltage: volt,
                irradiance:    irr,
            })
        })
        .collect();

    let result = crate::weather::compute_solar_efficiency(&points, &weather.hours);

    Ok(axum::Json(crate::weather::SolarEfficiencyResponse {
        object_id:             id,
        computed_at:           chrono::Utc::now(),
        score:                 result.score,
        status:                result.status,
        status_label:          result.status_label,
        message:               result.message,
        baseline_ratio:        result.baseline_ratio,
        recent_ratio:          result.recent_ratio,
        sample_count_baseline: result.sample_count_baseline,
        sample_count_recent:   result.sample_count_recent,
        daily_scores:          result.daily_scores,
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT LOG  [admin only]
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/admin/audit-log
pub async fn get_audit_log(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Query(q): Query<AuditLogQuery>,
) -> AppResult<Json<Page<AuditLogEntry>>> {
    require_admin(&claims)?;
    Ok(Json(db::list_audit_log(&pool, &q).await?))
}

// ═══════════════════════════════════════════════════════════════════════════
// CHANGE PASSWORD  [any authenticated user]
// ═══════════════════════════════════════════════════════════════════════════

/// POST /api/v1/auth/change-password
pub async fn change_password(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<StatusCode> {
    let uid  = parse_uid(&claims.sub)?;
    let user = db::find_user_full(&pool, uid).await?.ok_or(AppError::Unauthorized)?;

    if !auth_svc::verify_password(&req.current_password, &user.password_hash)? {
        return Err(AppError::BadRequest("Pogrešna trenutna lozinka".into()));
    }
    if req.new_password.len() < 8 {
        return Err(AppError::Validation("Nova lozinka mora imati najmanje 8 znakova".into()));
    }

    let new_hash = auth_svc::hash_password(&req.new_password)?;
    db::update_user_password(&pool, uid, &new_hash).await?;
    let _ = db::write_audit(&pool, Some(uid), Some(&claims.username),
        "CHANGE_PASSWORD", Some("user"), Some(&uid.to_string()), None, None).await;
    Ok(StatusCode::NO_CONTENT)
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn require_admin(claims: &JwtClaims) -> AppResult<()> {
    if claims.role != "admin" { Err(AppError::Forbidden) } else { Ok(()) }
}

fn parse_uid(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::Unauthorized)
}

async fn check_object_access(pool: &PgPool, claims: &JwtClaims, object_id: Uuid) -> AppResult<()> {
    let uid = parse_uid(&claims.sub)?;
    let obj = db::get_object_by_id(pool, object_id).await?
        .ok_or_else(|| AppError::NotFound(format!("Object {}", object_id)))?;
    if !db::user_can_access_region(pool, uid, &claims.role, obj.region_id).await? {
        return Err(AppError::Forbidden);
    }
    Ok(())
}
