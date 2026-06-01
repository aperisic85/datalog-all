//! Modeli za sustav obavještavanja o alarmima

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ── Kanali ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NotificationChannel {
    pub id:         Uuid,
    pub name:       String,
    pub kind:       String,
    pub config:     Value,
    pub enabled:    bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name:    String,
    pub kind:    String,
    pub config:  Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub name:    Option<String>,
    pub config:  Option<Value>,
    pub enabled: Option<bool>,
}

// ── Pravila ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NotificationRule {
    pub id:                Uuid,
    pub name:              String,
    pub channel_id:        Uuid,
    pub region_id:         Option<Uuid>,
    pub min_severity:      i16,
    pub notify_on_clear:   bool,
    pub quiet_hours_start: Option<i16>,
    pub quiet_hours_end:   Option<i16>,
    pub cooldown_minutes:  i32,
    pub enabled:           bool,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub name:              String,
    pub channel_id:        Uuid,
    pub region_id:         Option<Uuid>,
    #[serde(default = "default_severity")]
    pub min_severity:      i16,
    #[serde(default = "default_true")]
    pub notify_on_clear:   bool,
    pub quiet_hours_start: Option<i16>,
    pub quiet_hours_end:   Option<i16>,
    #[serde(default = "default_cooldown")]
    pub cooldown_minutes:  i32,
    #[serde(default = "default_true")]
    pub enabled:           bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    pub name:              Option<String>,
    pub channel_id:        Option<Uuid>,
    pub region_id:         Option<Uuid>,
    pub clear_region:      Option<bool>,   // true → postavi region_id na NULL (sve regije)
    pub min_severity:      Option<i16>,
    pub notify_on_clear:   Option<bool>,
    pub quiet_hours_start: Option<i16>,
    pub quiet_hours_end:   Option<i16>,
    pub cooldown_minutes:  Option<i32>,
    pub enabled:           Option<bool>,
}

// ── Log ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NotificationLogEntry {
    pub id:           i64,
    pub channel_id:   Option<Uuid>,
    pub channel_name: Option<String>,
    pub object_id:    Option<Uuid>,
    pub object_name:  Option<String>,
    pub alarm_type:   Option<String>,
    pub severity:     Option<i16>,
    pub event:        String,
    pub status:       String,
    pub error:        Option<String>,
    pub message:      Option<String>,
    pub created_at:   DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NotificationLogQuery {
    pub page:      Option<i64>,
    pub page_size: Option<i64>,
}

fn default_true() -> bool { true }
fn default_severity() -> i16 { 3 }
fn default_cooldown() -> i32 { 360 }
