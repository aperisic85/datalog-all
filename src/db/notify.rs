//! DB sloj za sustav obavještavanja.
//! Koristi runtime upite (bez compile-time provjere) — kao audit log —
//! da nije potreban živi DB / .sqlx cache pri buildu.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppResult;
use crate::models::domain::Page;
use crate::models::notify::*;

// ── Kanali ──────────────────────────────────────────────────────────────────

pub async fn list_channels(pool: &PgPool) -> AppResult<Vec<NotificationChannel>> {
    Ok(sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels ORDER BY created_at DESC")
        .fetch_all(pool).await?)
}

pub async fn get_channel(pool: &PgPool, id: Uuid) -> AppResult<Option<NotificationChannel>> {
    Ok(sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = $1")
        .bind(id).fetch_optional(pool).await?)
}

pub async fn create_channel(pool: &PgPool, req: &CreateChannelRequest, by: Option<Uuid>)
    -> AppResult<NotificationChannel>
{
    Ok(sqlx::query_as::<_, NotificationChannel>(
        "INSERT INTO notification_channels (name, kind, config, enabled, created_by)
         VALUES ($1, $2, $3, $4, $5) RETURNING *")
        .bind(&req.name).bind(&req.kind).bind(&req.config).bind(req.enabled).bind(by)
        .fetch_one(pool).await?)
}

pub async fn update_channel(pool: &PgPool, id: Uuid, req: &UpdateChannelRequest)
    -> AppResult<Option<NotificationChannel>>
{
    Ok(sqlx::query_as::<_, NotificationChannel>(
        "UPDATE notification_channels SET
            name    = COALESCE($2, name),
            config  = COALESCE($3, config),
            enabled = COALESCE($4, enabled),
            updated_at = NOW()
         WHERE id = $1 RETURNING *")
        .bind(id).bind(&req.name).bind(&req.config).bind(req.enabled)
        .fetch_optional(pool).await?)
}

pub async fn delete_channel(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let r = sqlx::query("DELETE FROM notification_channels WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}

// ── Pravila ─────────────────────────────────────────────────────────────────

pub async fn list_rules(pool: &PgPool) -> AppResult<Vec<NotificationRule>> {
    Ok(sqlx::query_as::<_, NotificationRule>(
        "SELECT * FROM notification_rules ORDER BY created_at DESC")
        .fetch_all(pool).await?)
}

pub async fn create_rule(pool: &PgPool, req: &CreateRuleRequest) -> AppResult<NotificationRule> {
    Ok(sqlx::query_as::<_, NotificationRule>(
        "INSERT INTO notification_rules
            (name, channel_id, region_id, min_severity, notify_on_clear,
             quiet_hours_start, quiet_hours_end, cooldown_minutes, enabled)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING *")
        .bind(&req.name).bind(req.channel_id).bind(req.region_id)
        .bind(req.min_severity).bind(req.notify_on_clear)
        .bind(req.quiet_hours_start).bind(req.quiet_hours_end)
        .bind(req.cooldown_minutes).bind(req.enabled)
        .fetch_one(pool).await?)
}

pub async fn update_rule(pool: &PgPool, id: Uuid, req: &UpdateRuleRequest)
    -> AppResult<Option<NotificationRule>>
{
    // clear_region=true → eksplicitno postavi region_id na NULL (sve regije)
    let clear_region = req.clear_region.unwrap_or(false);
    Ok(sqlx::query_as::<_, NotificationRule>(
        "UPDATE notification_rules SET
            name              = COALESCE($2, name),
            channel_id        = COALESCE($3, channel_id),
            region_id         = CASE WHEN $4 THEN NULL ELSE COALESCE($5, region_id) END,
            min_severity      = COALESCE($6, min_severity),
            notify_on_clear   = COALESCE($7, notify_on_clear),
            quiet_hours_start = COALESCE($8, quiet_hours_start),
            quiet_hours_end   = COALESCE($9, quiet_hours_end),
            cooldown_minutes  = COALESCE($10, cooldown_minutes),
            enabled           = COALESCE($11, enabled),
            updated_at        = NOW()
         WHERE id = $1 RETURNING *")
        .bind(id).bind(&req.name).bind(req.channel_id)
        .bind(clear_region).bind(req.region_id)
        .bind(req.min_severity).bind(req.notify_on_clear)
        .bind(req.quiet_hours_start).bind(req.quiet_hours_end)
        .bind(req.cooldown_minutes).bind(req.enabled)
        .fetch_optional(pool).await?)
}

pub async fn delete_rule(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let r = sqlx::query("DELETE FROM notification_rules WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}

// ── Dispatch: dohvat pravila + stanje ────────────────────────────────────────

/// Pravila (sa pripadnim kanalom) koja se podudaraju za danu regiju i ozbiljnost.
pub async fn matching_rules(pool: &PgPool, region_id: Uuid, severity: i16)
    -> AppResult<Vec<(NotificationRule, NotificationChannel)>>
{
    let rules = sqlx::query_as::<_, NotificationRule>(
        "SELECT * FROM notification_rules
         WHERE enabled = TRUE
           AND min_severity <= $1
           AND (region_id IS NULL OR region_id = $2)")
        .bind(severity).bind(region_id)
        .fetch_all(pool).await?;

    let mut out = Vec::new();
    for rule in rules {
        if let Some(ch) = get_channel(pool, rule.channel_id).await? {
            if ch.enabled {
                out.push((rule, ch));
            }
        }
    }
    Ok(out)
}

/// Vraća (active, last_notified_at, since) za (objekt, tip alarma), ako zapis postoji.
pub async fn get_state(pool: &PgPool, object_id: Uuid, alarm_type: &str)
    -> AppResult<Option<(bool, Option<DateTime<Utc>>, Option<DateTime<Utc>>)>>
{
    let row = sqlx::query_as::<_, (bool, Option<DateTime<Utc>>, Option<DateTime<Utc>>)>(
        "SELECT active, last_notified_at, since FROM notification_state
         WHERE object_id = $1 AND alarm_type = $2")
        .bind(object_id).bind(alarm_type)
        .fetch_optional(pool).await?;
    Ok(row)
}

/// Postavi stanje na aktivno (zadrži postojeći `since` ako već postoji).
pub async fn set_state_active(pool: &PgPool, object_id: Uuid, alarm_type: &str, now: DateTime<Utc>)
    -> AppResult<()>
{
    sqlx::query(
        "INSERT INTO notification_state (object_id, alarm_type, active, since)
         VALUES ($1, $2, TRUE, $3)
         ON CONFLICT (object_id, alarm_type)
         DO UPDATE SET active = TRUE,
                       since  = COALESCE(notification_state.since, EXCLUDED.since)")
        .bind(object_id).bind(alarm_type).bind(now)
        .execute(pool).await?;
    Ok(())
}

pub async fn set_state_cleared(pool: &PgPool, object_id: Uuid, alarm_type: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE notification_state SET active = FALSE, since = NULL
         WHERE object_id = $1 AND alarm_type = $2")
        .bind(object_id).bind(alarm_type)
        .execute(pool).await?;
    Ok(())
}

pub async fn mark_notified(pool: &PgPool, object_id: Uuid, alarm_type: &str, now: DateTime<Utc>)
    -> AppResult<()>
{
    sqlx::query(
        "UPDATE notification_state SET last_notified_at = $3
         WHERE object_id = $1 AND alarm_type = $2")
        .bind(object_id).bind(alarm_type).bind(now)
        .execute(pool).await?;
    Ok(())
}

// ── Log ─────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn insert_log(
    pool: &PgPool,
    channel_id: Option<Uuid>, channel_name: Option<&str>,
    object_id: Option<Uuid>, object_name: Option<&str>,
    alarm_type: Option<&str>, severity: Option<i16>,
    event: &str, status: &str, error: Option<&str>, message: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO notification_log
            (channel_id, channel_name, object_id, object_name, alarm_type,
             severity, event, status, error, message)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(channel_id).bind(channel_name).bind(object_id).bind(object_name)
        .bind(alarm_type).bind(severity).bind(event).bind(status).bind(error).bind(message)
        .execute(pool).await?;
    Ok(())
}

pub async fn list_log(pool: &PgPool, q: &NotificationLogQuery)
    -> AppResult<Page<NotificationLogEntry>>
{
    let page      = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(50).clamp(1, 200);
    let offset    = (page - 1) * page_size;

    let rows = sqlx::query_as::<_, NotificationLogEntry>(
        "SELECT * FROM notification_log ORDER BY created_at DESC LIMIT $1 OFFSET $2")
        .bind(page_size).bind(offset)
        .fetch_all(pool).await?;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notification_log")
        .fetch_one(pool).await?;

    Ok(Page::new(rows, total, page, page_size))
}

/// Dohvati objekt (id, ime, regija) po station_id — za rezoluciju u dispatchu.
pub async fn resolve_object(pool: &PgPool, station_id: &str)
    -> AppResult<Option<(Uuid, String, Uuid)>>
{
    Ok(sqlx::query_as::<_, (Uuid, String, Uuid)>(
        "SELECT id, name, region_id FROM objects WHERE station_id = $1")
        .bind(station_id).fetch_optional(pool).await?)
}

// ── Upiti za Telegram bota (dvosmjerna komunikacija) ──────────────────────────

/// Chat ID-evi koji smiju slati upite botu = svi omogućeni Telegram kanali.
pub async fn bot_authorized_chat_ids(pool: &PgPool) -> AppResult<Vec<String>> {
    let rows = sqlx::query_scalar::<_, Option<String>>(
        "SELECT config->>'chat_id' FROM notification_channels
         WHERE kind = 'telegram' AND enabled = TRUE")
        .fetch_all(pool).await?;
    Ok(rows.into_iter().flatten().collect())
}

/// Sažetak po regijama: (naziv, ukupno objekata, u alarmu).
pub async fn bot_region_status(pool: &PgPool) -> AppResult<Vec<(String, i64, i64)>> {
    Ok(sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT r.name,
                COUNT(o.id) AS total,
                COUNT(o.id) FILTER (WHERE o.alarm_active) AS in_alarm
         FROM regions r
         LEFT JOIN objects o ON o.region_id = r.id AND o.is_active
         WHERE r.is_active
         GROUP BY r.name
         ORDER BY r.name")
        .fetch_all(pool).await?)
}

/// Trenutno aktivni alarmi: (objekt, regija, najgori nivo, sažetak).
pub async fn bot_active_alarms(pool: &PgPool)
    -> AppResult<Vec<(String, String, Option<i16>, Option<String>)>>
{
    Ok(sqlx::query_as::<_, (String, String, Option<i16>, Option<String>)>(
        "SELECT o.name, r.name, o.alarm_worst_level, o.alarm_summary
         FROM objects o
         JOIN regions r ON r.id = o.region_id
         WHERE o.alarm_active = TRUE
         ORDER BY o.alarm_worst_level DESC NULLS LAST, o.name
         LIMIT 50")
        .fetch_all(pool).await?)
}

/// Stanje pojedinog objekta po (djelomičnom) imenu.
/// Vraća: (ime, regija, alarm_active, najgori_nivo, sažetak, broj_alarma,
///         napon_baterije, svjetlo_aktivno, vrijeme_zadnjeg_mjerenja).
#[allow(clippy::type_complexity)]
pub async fn bot_find_object(pool: &PgPool, query: &str)
    -> AppResult<Option<(String, String, bool, Option<i16>, Option<String>, i16,
                         Option<f32>, Option<f32>, Option<DateTime<Utc>>)>>
{
    Ok(sqlx::query_as::<_, (String, String, bool, Option<i16>, Option<String>, i16,
                            Option<f32>, Option<f32>, Option<DateTime<Utc>>)>(
        "SELECT o.name, r.name, o.alarm_active, o.alarm_worst_level, o.alarm_summary,
                o.alarm_count, lm.battery_voltage_avg, lm.lantern_light_active_avg, lm.recorded_at
         FROM objects o
         JOIN regions r ON r.id = o.region_id
         LEFT JOIN v_latest_measurements lm ON lm.object_id = o.id
         WHERE o.name ILIKE '%' || $1 || '%' OR o.short_name ILIKE '%' || $1 || '%'
         ORDER BY (o.name ILIKE $1) DESC, o.name
         LIMIT 1")
        .bind(query)
        .fetch_optional(pool).await?)
}

