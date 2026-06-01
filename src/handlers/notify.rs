//! HTTP handleri za sustav obavještavanja (admin only).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::notify as ndb;
use crate::errors::{AppError, AppResult};
use crate::models::domain::{JwtClaims, Page};
use crate::models::notify::*;
use crate::notify as notifier;

fn require_admin(claims: &JwtClaims) -> AppResult<()> {
    if claims.role != "admin" { Err(AppError::Forbidden) } else { Ok(()) }
}

const VALID_KINDS: [&str; 3] = ["telegram", "webhook", "slack"];

// ── Kanali ──────────────────────────────────────────────────────────────────

/// GET /api/v1/notifications/channels
pub async fn list_channels(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
) -> AppResult<Json<Vec<NotificationChannel>>> {
    require_admin(&claims)?;
    Ok(Json(ndb::list_channels(&pool).await?))
}

/// POST /api/v1/notifications/channels
pub async fn create_channel(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<CreateChannelRequest>,
) -> AppResult<(StatusCode, Json<NotificationChannel>)> {
    require_admin(&claims)?;
    if !VALID_KINDS.contains(&req.kind.as_str()) {
        return Err(AppError::Validation(format!("Nepoznata vrsta kanala: {}", req.kind)));
    }
    let by = Uuid::parse_str(&claims.sub).ok();
    let ch = ndb::create_channel(&pool, &req, by).await?;
    Ok((StatusCode::CREATED, Json(ch)))
}

/// PATCH /api/v1/notifications/channels/:id
pub async fn update_channel(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateChannelRequest>,
) -> AppResult<Json<NotificationChannel>> {
    require_admin(&claims)?;
    ndb::update_channel(&pool, id, &req).await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Kanal {}", id)))
}

/// DELETE /api/v1/notifications/channels/:id
pub async fn delete_channel(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;
    if ndb::delete_channel(&pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Kanal {}", id)))
    }
}

/// POST /api/v1/notifications/channels/:id/test
pub async fn test_channel(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&claims)?;
    let ch = ndb::get_channel(&pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("Kanal {}", id)))?;
    match notifier::send_test(&pool, &ch).await {
        Ok(())   => Ok(Json(serde_json::json!({ "status": "sent" }))),
        Err(msg) => Ok(Json(serde_json::json!({ "status": "failed", "error": msg }))),
    }
}

// ── Pravila ─────────────────────────────────────────────────────────────────

/// GET /api/v1/notifications/rules
pub async fn list_rules(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
) -> AppResult<Json<Vec<NotificationRule>>> {
    require_admin(&claims)?;
    Ok(Json(ndb::list_rules(&pool).await?))
}

/// POST /api/v1/notifications/rules
pub async fn create_rule(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Json(req): Json<CreateRuleRequest>,
) -> AppResult<(StatusCode, Json<NotificationRule>)> {
    require_admin(&claims)?;
    if !(1..=4).contains(&req.min_severity) {
        return Err(AppError::Validation("min_severity mora biti 1–4".into()));
    }
    let rule = ndb::create_rule(&pool, &req).await?;
    Ok((StatusCode::CREATED, Json(rule)))
}

/// PATCH /api/v1/notifications/rules/:id
pub async fn update_rule(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRuleRequest>,
) -> AppResult<Json<NotificationRule>> {
    require_admin(&claims)?;
    ndb::update_rule(&pool, id, &req).await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Pravilo {}", id)))
}

/// DELETE /api/v1/notifications/rules/:id
pub async fn delete_rule(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;
    if ndb::delete_rule(&pool, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Pravilo {}", id)))
    }
}

// ── Log ─────────────────────────────────────────────────────────────────────

/// GET /api/v1/notifications/log
pub async fn list_log(
    State(pool): State<PgPool>,
    Extension(claims): Extension<JwtClaims>,
    Query(q): Query<NotificationLogQuery>,
) -> AppResult<Json<Page<NotificationLogEntry>>> {
    require_admin(&claims)?;
    Ok(Json(ndb::list_log(&pool, &q).await?))
}
