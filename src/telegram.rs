//! Telegram bot — dvosmjerna komunikacija (upiti → odgovori).
//!
//! Radi preko long-pollinga (getUpdates), pa ne treba javni HTTPS URL.
//! Pokreće se samo ako je postavljena env varijabla `TELEGRAM_BOT_TOKEN`.
//!
//! Odgovara isključivo chat ID-evima koji su registrirani kao omogućeni
//! Telegram kanali za obavijesti (vidi notification_channels).
//!
//! Podržane komande:
//!   /status        — sažetak po regijama
//!   /alarmi        — trenutno aktivni alarmi
//!   /objekt <ime>  — stanje pojedinog objekta
//!   /pomoc         — popis komandi

use std::time::Duration;

use serde_json::{json, Value};
use sqlx::PgPool;

use crate::db::notify as ndb;
use crate::notify::severity_label;

pub fn start_bot(pool: PgPool) {
    let token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            tracing::info!("TELEGRAM_BOT_TOKEN nije postavljen — Telegram bot (upiti) onemogućen");
            return;
        }
    };
    tokio::spawn(async move { run(pool, token).await; });
}

async fn run(pool: PgPool, token: String) {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(60)).build() {
        Ok(c) => c,
        Err(e) => { tracing::error!(error = %e, "Telegram bot: ne mogu kreirati HTTP klijent"); return; }
    };

    tracing::info!("Telegram bot (dvosmjerna komunikacija) pokrenut");

    // Ignoriraj poruke poslane prije pokretanja (backlog nakon restarta)
    let start_ts = chrono::Utc::now().timestamp();
    let mut offset: i64 = 0;

    loop {
        match get_updates(&client, &token, offset).await {
            Ok(updates) => {
                for upd in updates {
                    let update_id = upd.get("update_id").and_then(|v| v.as_i64()).unwrap_or(0);
                    if update_id >= offset { offset = update_id + 1; }
                    if let Err(e) = handle_update(&pool, &client, &token, &upd, start_ts).await {
                        tracing::warn!(error = %e, "Telegram bot: greška pri obradi poruke");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Telegram bot: getUpdates nije uspio — pauza 5s");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn get_updates(client: &reqwest::Client, token: &str, offset: i64) -> anyhow::Result<Vec<Value>> {
    let url = format!("https://api.telegram.org/bot{}/getUpdates", token);
    let resp = client.get(&url)
        .query(&[("offset", offset.to_string()), ("timeout", "30".to_string())])
        .send().await?;
    let body: Value = resp.json().await?;
    if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        anyhow::bail!("Telegram API odgovor: {}", body);
    }
    Ok(body.get("result").and_then(|v| v.as_array()).cloned().unwrap_or_default())
}

async fn handle_update(
    pool: &PgPool, client: &reqwest::Client, token: &str, upd: &Value, start_ts: i64,
) -> anyhow::Result<()> {
    let msg = match upd.get("message") { Some(m) => m, None => return Ok(()) };

    // Preskoči stare poruke (otprije pokretanja servisa)
    let date = msg.get("date").and_then(|v| v.as_i64()).unwrap_or(0);
    if date < start_ts { return Ok(()); }

    let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if text.is_empty() { return Ok(()); }

    let chat_id = match msg.get("chat").and_then(|c| c.get("id")) {
        Some(v) => v.to_string().trim_matches('"').to_string(),
        None => return Ok(()),
    };

    // Autorizacija — samo registrirani Telegram kanali
    let authorized = ndb::bot_authorized_chat_ids(pool).await.unwrap_or_default();
    if !authorized.iter().any(|c| c == &chat_id) {
        let reply = format!(
            "⛔ Nemate pristup ovom botu.\nVaš chat ID: {}\nZatražite od administratora da vas doda kao Telegram kanal za obavijesti.",
            chat_id
        );
        send_message(client, token, &chat_id, &reply).await?;
        return Ok(());
    }

    let reply = handle_command(pool, &text).await;
    send_message(client, token, &chat_id, &reply).await?;
    Ok(())
}

// ── Komande ───────────────────────────────────────────────────────────────────

async fn handle_command(pool: &PgPool, text: &str) -> String {
    let mut parts = text.splitn(2, char::is_whitespace);
    let cmd_raw = parts.next().unwrap_or("");
    let arg = parts.next().unwrap_or("").trim();
    // skini eventualni @ime_bota sufiks (u grupama)
    let cmd = cmd_raw.split('@').next().unwrap_or("").to_lowercase();

    match cmd.as_str() {
        "/start" | "/pomoc" | "/help" => help_text(),
        "/status" => cmd_status(pool).await,
        "/alarmi" | "/alarms" => cmd_alarms(pool).await,
        "/objekt" | "/object" => {
            if arg.is_empty() {
                "Koristite: /objekt <ime>\nNpr. /objekt Galija".to_string()
            } else {
                cmd_object(pool, arg).await
            }
        }
        _ => format!("Nepoznata komanda: {}\n\n{}", cmd_raw, help_text()),
    }
}

fn help_text() -> String {
    "🤖 Beacon nadzor — dostupne komande:\n\n\
     /status — sažetak po regijama\n\
     /alarmi — trenutno aktivni alarmi\n\
     /objekt <ime> — stanje objekta (npr. /objekt Galija)\n\
     /pomoc — ova poruka".to_string()
}

async fn cmd_status(pool: &PgPool) -> String {
    match ndb::bot_region_status(pool).await {
        Ok(rows) if !rows.is_empty() => {
            let mut total = 0i64;
            let mut alarm = 0i64;
            let mut out = String::from("📊 Status sustava\n\n");
            for (name, t, a) in &rows {
                total += *t;
                alarm += *a;
                let mark = if *a > 0 { "🔴" } else { "🟢" };
                out.push_str(&format!("{} {} — {} obj., {} u alarmu\n", mark, name, t, a));
            }
            out.push_str(&format!("\nUkupno: {} objekata, {} u alarmu", total, alarm));
            out
        }
        Ok(_) => "Nema registriranih regija.".to_string(),
        Err(e) => { tracing::warn!(error = %e, "bot /status"); "Greška pri dohvaćanju statusa.".to_string() }
    }
}

async fn cmd_alarms(pool: &PgPool) -> String {
    match ndb::bot_active_alarms(pool).await {
        Ok(rows) if !rows.is_empty() => {
            let mut out = format!("🚨 Aktivni alarmi ({})\n\n", rows.len());
            for (name, region, lvl, summary) in &rows {
                let l = match lvl { Some(v) => severity_label(*v), None => "Alarm" };
                out.push_str(&format!("• {} ({}) — {}\n   {}\n", name, region, l, summary.as_deref().unwrap_or("—")));
            }
            out
        }
        Ok(_) => "✅ Nema aktivnih alarma.".to_string(),
        Err(e) => { tracing::warn!(error = %e, "bot /alarmi"); "Greška pri dohvaćanju alarma.".to_string() }
    }
}

async fn cmd_object(pool: &PgPool, query: &str) -> String {
    match ndb::bot_find_object(pool, query).await {
        Ok(Some((name, region, active, lvl, summary, count, volt, light, ts))) => {
            let mut out = format!("📍 {} ({})\n\n", name, region);
            out.push_str(&format!("Stanje: {}\n", if active { "🔴 ALARM" } else { "🟢 OK" }));
            if active {
                let l = match lvl { Some(v) => severity_label(v), None => "Alarm" };
                out.push_str(&format!("Nivo: {} ({} aktivnih)\n", l, count));
                if let Some(sm) = summary { out.push_str(&format!("Opis: {}\n", sm)); }
            }
            if let Some(v) = volt { out.push_str(&format!("Baterija: {:.2} V\n", v)); }
            if let Some(la) = light { out.push_str(&format!("Svjetlo aktivno: {:.0} %\n", la * 100.0)); }
            if let Some(t) = ts { out.push_str(&format!("Zadnje mjerenje: {}\n", t.format("%d.%m.%Y %H:%M UTC"))); }
            out
        }
        Ok(None) => format!("Objekt \"{}\" nije pronađen.", query),
        Err(e) => { tracing::warn!(error = %e, "bot /objekt"); "Greška pri dohvaćanju objekta.".to_string() }
    }
}

// ── Slanje ────────────────────────────────────────────────────────────────────

async fn send_message(client: &reqwest::Client, token: &str, chat_id: &str, text: &str) -> anyhow::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let resp = client.post(&url)
        .json(&json!({ "chat_id": chat_id, "text": text }))
        .send().await?;
    if !resp.status().is_success() {
        let body: String = resp.text().await.unwrap_or_default().chars().take(200).collect();
        anyhow::bail!("sendMessage HTTP greška: {}", body);
    }
    Ok(())
}
