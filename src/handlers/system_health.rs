use std::collections::{HashMap, HashSet};

use axum::{extract::State, Json};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::poller::SharedPollerStatus;

#[derive(Clone)]
pub struct SystemHealthState {
    pub pool: PgPool,
    pub poller_status: SharedPollerStatus,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ServiceHealth {
    pub status: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PollerHealth {
    pub total: usize,
    pub online: usize,
    pub offline: usize,
    pub late: usize,
    pub last_successful_poll: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DataHealth {
    pub active_objects: i64,
    pub stale_objects: i64,
}

#[derive(Debug, Serialize)]
pub struct NotificationHealth {
    pub status: &'static str,
    pub enabled_channels: i64,
    pub failed_last_24h: i64,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SystemHealthResponse {
    pub status: &'static str,
    pub checked_at: DateTime<Utc>,
    pub uptime_seconds: i64,
    pub api: ServiceHealth,
    pub database: ServiceHealth,
    pub pollers: PollerHealth,
    pub data: DataHealth,
    pub notifications: NotificationHealth,
}

pub async fn system_health(
    State(state): State<SystemHealthState>,
) -> Json<SystemHealthResponse> {
    let now = Utc::now();
    let uptime_seconds = (now - state.started_at).num_seconds().max(0);

    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();

    if !db_ok {
        return Json(SystemHealthResponse {
            status: "critical",
            checked_at: now,
            uptime_seconds,
            api: ServiceHealth {
                status: "ok",
                message: "Backend API odgovara".into(),
            },
            database: ServiceHealth {
                status: "error",
                message: "PostgreSQL nije dostupan".into(),
            },
            pollers: PollerHealth {
                total: 0,
                online: 0,
                offline: 0,
                late: 0,
                last_successful_poll: None,
            },
            data: DataHealth {
                active_objects: 0,
                stale_objects: 0,
            },
            notifications: NotificationHealth {
                status: "unknown",
                enabled_channels: 0,
                failed_last_24h: 0,
                last_success_at: None,
                last_failure_at: None,
            },
        });
    }

    let poll_configs = sqlx::query_as::<_, (String, i32, String)>(
        "SELECT station_id, poll_interval_sec, source_kind
         FROM objects
         WHERE is_active = TRUE AND polling_enabled = TRUE"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let status = state.poller_status.read().await;

    let mut expected: HashMap<String, i64> = HashMap::new();
    for (station_id, interval, source_kind) in poll_configs {
        let effective_interval = if source_kind == "aton_csd" {
            i64::from(interval.max(300))
        } else {
            i64::from(interval.max(10))
        };
        expected.insert(station_id, effective_interval);
    }

    let mut station_names: HashSet<String> = expected.keys().cloned().collect();
    station_names.extend(status.online.keys().cloned());
    station_names.extend(status.last_poll.keys().cloned());

    let mut online = 0usize;
    let mut offline = 0usize;
    let mut late = 0usize;
    let mut last_successful_poll: Option<DateTime<Utc>> = None;

    for name in &station_names {
        if status.online.get(name).copied().unwrap_or(false) {
            online += 1;
        } else {
            offline += 1;
        }

        let interval = expected.get(name).copied().unwrap_or(60);
        let late_after = (interval * 3).max(120);

        match status.last_poll.get(name) {
            Some(last) => {
                if now.signed_duration_since(*last).num_seconds() > late_after {
                    late += 1;
                }
                if last_successful_poll.map(|current| *last > current).unwrap_or(true) {
                    last_successful_poll = Some(*last);
                }
            }
            None if uptime_seconds > late_after => {
                late += 1;
            }
            None => {}
        }
    }
    drop(status);

    let active_objects = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM v_objects WHERE is_active = TRUE"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let stale_objects = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM v_objects WHERE is_active = TRUE AND is_silent = TRUE"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let enabled_channels = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_channels WHERE enabled = TRUE"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let last_success_at = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT MAX(created_at) FROM notification_log WHERE status = 'sent'"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(None);

    let last_failure_at = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT MAX(created_at) FROM notification_log WHERE status = 'failed'"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(None);

    let failed_last_24h = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_log
         WHERE status = 'failed' AND created_at >= NOW() - INTERVAL '24 hours'"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let notification_status = if enabled_channels == 0 {
        "not_configured"
    } else if last_failure_at
        .map(|failed| last_success_at.map(|sent| failed > sent).unwrap_or(true))
        .unwrap_or(false)
    {
        "degraded"
    } else if last_success_at.is_some() {
        "ok"
    } else {
        "unknown"
    };

    let poller_health = PollerHealth {
        total: station_names.len(),
        online,
        offline,
        late,
        last_successful_poll,
    };

    let notifications = NotificationHealth {
        status: notification_status,
        enabled_channels,
        failed_last_24h,
        last_success_at,
        last_failure_at,
    };

    let overall = if poller_health.late > 0
        || poller_health.offline > 0
        || stale_objects > 0
        || matches!(notification_status, "degraded" | "unknown")
    {
        "degraded"
    } else {
        "ok"
    };

    Json(SystemHealthResponse {
        status: overall,
        checked_at: now,
        uptime_seconds,
        api: ServiceHealth {
            status: "ok",
            message: "Backend API odgovara".into(),
        },
        database: ServiceHealth {
            status: "ok",
            message: "PostgreSQL je dostupan".into(),
        },
        pollers: poller_health,
        data: DataHealth {
            active_objects,
            stale_objects,
        },
        notifications,
    })
}
