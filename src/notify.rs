//! Sustav obavještavanja o alarmima.
//!
//! Pri svakom novom snapshotu alarma (`dispatch_for_alarm`) detektiramo
//! prijelaze stanja po (objekt, tip alarma):
//!   • neaktivan → aktivan  ⇒ pošalji "ALARM" obavijest
//!   • aktivan i dalje       ⇒ ponovi tek nakon cooldowna pravila
//!   • aktivan → neaktivan   ⇒ pošalji "RIJEŠENO" (ako pravilo to traži)
//!
//! Obavijesti se isporučuju preko kanala (Telegram / Slack / generički webhook)
//! sukladno pravilima koja filtriraju po regiji i minimalnoj ozbiljnosti.

use std::collections::HashSet;
use std::time::Duration;

use chrono::{Timelike, Utc};
use serde_json::json;
use sqlx::PgPool;

use crate::db::notify as ndb;
use crate::models::domain::AlarmInsert;
use crate::models::notify::{NotificationChannel, NotificationRule};

// ── Katalog tipova alarma ────────────────────────────────────────────────────

pub struct AlarmTypeDef {
    pub key:      &'static str,
    pub label:    &'static str,
    pub severity: i16, // 1=INFO, 2=WARN, 3=ERROR, 4=FATAL
}

pub const CATALOG: &[AlarmTypeDef] = &[
    AlarmTypeDef { key: "datalogger_high_temp",       label: "Visoka temperatura dataloggera",            severity: 3 },
    AlarmTypeDef { key: "datalogger_high_voltage",    label: "Visoki napon dataloggera",                  severity: 3 },
    AlarmTypeDef { key: "datalogger_other_error",     label: "Greška dataloggera",                        severity: 2 },
    AlarmTypeDef { key: "battery_voltage_low",        label: "Nizak napon baterije",                      severity: 3 },
    AlarmTypeDef { key: "battery_voltage_flat",       label: "Baterija prazna",                           severity: 4 },
    AlarmTypeDef { key: "battery_other_error",        label: "Greška baterije",                           severity: 2 },
    AlarmTypeDef { key: "garmin_comm_failed",         label: "GPS komunikacija prekinuta",                severity: 3 },
    AlarmTypeDef { key: "garmin_other_error",         label: "GPS greška",                                severity: 2 },
    AlarmTypeDef { key: "station_out_of_radius",      label: "Stanica izvan radijusa (pomak pozicije)",   severity: 4 },
    AlarmTypeDef { key: "lantern_night_light_off",    label: "Fenjer ugašen noću",                        severity: 4 },
    AlarmTypeDef { key: "lantern_day_light_on",       label: "Fenjer upaljen danju",                      severity: 2 },
    AlarmTypeDef { key: "lantern_comm_failed",        label: "Komunikacija s fenjerom prekinuta",         severity: 3 },
    AlarmTypeDef { key: "lantern_other_error",        label: "Greška fenjera",                            severity: 2 },
    AlarmTypeDef { key: "modem_network_error",        label: "Modem bez mreže",                           severity: 3 },
    AlarmTypeDef { key: "modem_other_error",          label: "Greška modema",                             severity: 2 },
    AlarmTypeDef { key: "station_other_error",        label: "Greška stanice",                            severity: 2 },
    AlarmTypeDef { key: "visibility_comm_failed",     label: "Komunikacija sa senzorom vidljivosti prekinuta", severity: 3 },
    AlarmTypeDef { key: "visibility_error",           label: "Greška senzora vidljivosti",                severity: 2 },
    AlarmTypeDef { key: "fog_signal_off_during_fog",  label: "Maglena sirena ugašena za magle",           severity: 4 },
    AlarmTypeDef { key: "fog_signal_on_while_no_fog", label: "Maglena sirena radi bez magle",             severity: 2 },
];

fn flag_value(rec: &AlarmInsert, key: &str) -> i16 {
    match key {
        "datalogger_high_temp"       => rec.alarm_datalogger_high_temp,
        "datalogger_high_voltage"    => rec.alarm_datalogger_high_voltage,
        "datalogger_other_error"     => rec.alarm_datalogger_other_error,
        "battery_voltage_low"        => rec.alarm_battery_voltage_low,
        "battery_voltage_flat"       => rec.alarm_battery_voltage_flat,
        "battery_other_error"        => rec.alarm_battery_other_error,
        "garmin_comm_failed"         => rec.alarm_garmin_comm_failed,
        "garmin_other_error"         => rec.alarm_garmin_other_error,
        "station_out_of_radius"      => rec.alarm_station_out_of_radius,
        "lantern_night_light_off"    => rec.alarm_lantern_night_light_off,
        "lantern_day_light_on"       => rec.alarm_lantern_day_light_on,
        "lantern_comm_failed"        => rec.alarm_lantern_comm_failed,
        "lantern_other_error"        => rec.alarm_lantern_other_error,
        "modem_network_error"        => rec.alarm_modem_network_error,
        "modem_other_error"          => rec.alarm_modem_other_error,
        "station_other_error"        => rec.alarm_station_other_error,
        "visibility_comm_failed"     => rec.alarm_visibility_comm_failed,
        "visibility_error"           => rec.alarm_visibility_error,
        "fog_signal_off_during_fog"  => rec.alarm_fog_signal_off_during_fog,
        "fog_signal_on_while_no_fog" => rec.alarm_fog_signal_on_while_no_fog,
        _ => 0,
    }
}

fn active_alarm_types(rec: &AlarmInsert) -> Vec<&'static AlarmTypeDef> {
    CATALOG.iter().filter(|d| flag_value(rec, d.key) > 0).collect()
}

pub fn severity_label(s: i16) -> &'static str {
    match s {
        4 => "KRITIČNO",
        3 => "Greška",
        2 => "Upozorenje",
        _ => "Info",
    }
}

// ── Tihi sati ────────────────────────────────────────────────────────────────

fn in_quiet_hours(rule: &NotificationRule, hour_utc: i16) -> bool {
    match (rule.quiet_hours_start, rule.quiet_hours_end) {
        (Some(s), Some(e)) if s != e => {
            if s < e { hour_utc >= s && hour_utc < e } else { hour_utc >= s || hour_utc < e }
        }
        _ => false,
    }
}

// ── Glavni dispatch ──────────────────────────────────────────────────────────

/// Obradi jedan snapshot alarma i pošalji obavijesti za prijelaze stanja.
/// Nikad ne vraća grešku koja bi prekinula ingest — interno logira.
pub async fn dispatch_for_alarm(pool: &PgPool, rec: &AlarmInsert) {
    if let Err(e) = dispatch_inner(pool, rec).await {
        tracing::warn!(station = %rec.station_id, error = %e, "Obavještavanje nije uspjelo");
    }
}

async fn dispatch_inner(pool: &PgPool, rec: &AlarmInsert) -> anyhow::Result<()> {
    let now = Utc::now();

    // Ignoriraj povijesne/backfill zapise da ne šaljemo lažne obavijesti
    if rec.recorded_at < now - chrono::Duration::minutes(30) {
        return Ok(());
    }

    let (object_id, object_name, region_id) = match ndb::resolve_object(pool, &rec.station_id).await? {
        Some(o) => o,
        None => return Ok(()), // objekt nije registriran — preskoči
    };

    let active = active_alarm_types(rec);
    let active_keys: HashSet<&str> = active.iter().map(|d| d.key).collect();
    let hour_utc = now.hour() as i16;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    // 1) Aktivni alarmi — novi prijelaz ili ponavljanje nakon cooldowna
    for def in &active {
        let state         = ndb::get_state(pool, object_id, def.key).await?;
        let was_active    = state.as_ref().map(|s| s.0).unwrap_or(false);
        let last_notified = state.as_ref().and_then(|s| s.1);
        let is_new        = !was_active;

        ndb::set_state_active(pool, object_id, def.key, now).await?;

        let rules = ndb::matching_rules(pool, region_id, def.severity).await?;
        if rules.is_empty() { continue; }

        let text = format!(
            "🔴 ALARM • {object_name}\n{} ({})\nVrijeme: {}",
            def.label,
            severity_label(def.severity),
            rec.recorded_at.format("%d.%m.%Y %H:%M UTC"),
        );

        let mut sent_any = false;
        for (rule, ch) in &rules {
            // Tihi sati suspendiraju samo ne-kritične obavijesti
            if def.severity < 4 && in_quiet_hours(rule, hour_utc) { continue; }

            // Cooldown za i dalje aktivan alarm
            if !is_new {
                if let Some(ln) = last_notified {
                    if now - ln < chrono::Duration::minutes(rule.cooldown_minutes as i64) {
                        continue;
                    }
                }
            }

            deliver(pool, &client, ch, Some(object_id), Some(&object_name),
                    Some(def.key), Some(def.severity), "raised", &text).await;
            sent_any = true;
        }

        if sent_any {
            ndb::mark_notified(pool, object_id, def.key, now).await?;
        }
    }

    // 2) Riješeni alarmi — bili aktivni, više nisu
    for def in CATALOG {
        if active_keys.contains(def.key) { continue; }
        let state = ndb::get_state(pool, object_id, def.key).await?;
        let was_active = state.as_ref().map(|s| s.0).unwrap_or(false);
        if !was_active { continue; }

        ndb::set_state_cleared(pool, object_id, def.key).await?;

        let rules = ndb::matching_rules(pool, region_id, def.severity).await?;
        let text = format!(
            "🟢 RIJEŠENO • {object_name}\n{}\nVrijeme: {}",
            def.label,
            now.format("%d.%m.%Y %H:%M UTC"),
        );

        for (rule, ch) in &rules {
            if !rule.notify_on_clear { continue; }
            if def.severity < 4 && in_quiet_hours(rule, hour_utc) { continue; }
            deliver(pool, &client, ch, Some(object_id), Some(&object_name),
                    Some(def.key), Some(def.severity), "cleared", &text).await;
        }
    }

    Ok(())
}

/// Pošalji poruku kanalu i zapiši rezultat u notification_log.
async fn deliver(
    pool: &PgPool, client: &reqwest::Client, ch: &NotificationChannel,
    object_id: Option<uuid::Uuid>, object_name: Option<&str>,
    alarm_type: Option<&str>, severity: Option<i16>, event: &str, text: &str,
) {
    let (status, error) = match send_to_channel(client, ch, text).await {
        Ok(())   => ("sent", None),
        Err(msg) => {
            tracing::warn!(channel = %ch.name, error = %msg, "Slanje obavijesti nije uspjelo");
            ("failed", Some(msg))
        }
    };
    let _ = ndb::insert_log(
        pool, Some(ch.id), Some(&ch.name), object_id, object_name,
        alarm_type, severity, event, status, error.as_deref(), Some(text),
    ).await;
}

/// Probna poruka za testiranje kanala (admin UI).
pub async fn send_test(pool: &PgPool, ch: &NotificationChannel) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let text = format!("✅ Probna obavijest s kanala \"{}\" (Beacon nadzor)", ch.name);
    let result = send_to_channel(&client, ch, &text).await;
    let (status, error) = match &result {
        Ok(())   => ("sent", None),
        Err(msg) => ("failed", Some(msg.as_str())),
    };
    let _ = ndb::insert_log(
        pool, Some(ch.id), Some(&ch.name), None, None,
        None, None, "test", status, error, Some(&text),
    ).await;
    result
}

// ── Isporuka po vrsti kanala ─────────────────────────────────────────────────

async fn send_to_channel(client: &reqwest::Client, ch: &NotificationChannel, text: &str)
    -> Result<(), String>
{
    match ch.kind.as_str() {
        "telegram" => {
            let token = ch.config.get("bot_token").and_then(|v| v.as_str())
                .ok_or("nedostaje 'bot_token' u konfiguraciji")?;
            let chat = ch.config.get("chat_id")
                .ok_or("nedostaje 'chat_id' u konfiguraciji")?;
            let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
            let resp = client.post(&url)
                .json(&json!({ "chat_id": chat, "text": text }))
                .send().await.map_err(|e| e.to_string())?;
            check_response(resp).await
        }
        "slack" | "webhook" => {
            let url = ch.config.get("url").and_then(|v| v.as_str())
                .ok_or("nedostaje 'url' u konfiguraciji")?;
            let resp = client.post(url)
                .json(&json!({ "text": text }))
                .send().await.map_err(|e| e.to_string())?;
            check_response(resp).await
        }
        other => Err(format!("nepoznata vrsta kanala: {}", other)),
    }
}

async fn check_response(resp: reqwest::Response) -> Result<(), String> {
    let status = resp.status();
    if status.is_success() {
        Ok(())
    } else {
        let body: String = resp.text().await.unwrap_or_default().chars().take(200).collect();
        Err(format!("HTTP {} — {}", status.as_u16(), body))
    }
}
