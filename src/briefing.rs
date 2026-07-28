//! AI jutarnji brifing — dnevni sažetak stanja flote na Telegram.
//!
//! Svako jutro (env `BRIEFING_HOUR_UTC`, default 5 → 07:00 ljetnog CET-a)
//! sustav sastavi pregled: što se dogodilo u zadnja 24 sata, koji su alarmi
//! aktivni, koje su stanice tihe i što energetska prognoza kaže za sljedeće
//! dane — i pošalje ga svim omogućenim Telegram kanalima.
//!
//! Brifing je dostupan i na zahtjev: komanda `/brifing` u botu.
//!
//! Kao i ostatak LLM integracije, brojke NIKAD ne dolaze iz modela: izvještaj
//! se sastavlja deterministički iz baze, a LLM (ako je uključen) dodaje samo
//! kratki uvodni komentar na temelju već složenih činjenica. Ako LLM poziv
//! padne, brifing se šalje bez uvoda — sadržaj je isti.

use chrono::{Timelike, Utc};
use sqlx::PgPool;

use crate::db::notify as ndb;
use crate::notify::severity_label;

/// Sastavi kompletan tekst brifinga (činjenice iz baze + opcijski AI uvod).
pub async fn generate(pool: &PgPool, client: &reqwest::Client) -> String {
    let facts = match gather_facts(pool).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, "Brifing: prikupljanje činjenica nije uspjelo");
            return "⚠️ Brifing trenutno nije dostupan (greška pri dohvaćanju podataka).".to_string();
        }
    };

    let body = format_report(&facts);

    // Opcijski AI uvod — 2-3 rečenice o najvažnijem, iz istih činjenica
    let intro = if crate::llm::is_enabled() {
        match crate::llm::phrase_briefing(client, &facts_for_llm(&facts)).await {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!(error = %e, "Brifing: LLM uvod nije uspio — šaljem bez njega");
                None
            }
        }
    } else {
        None
    };

    let date = Utc::now().format("%d.%m.%Y");
    match intro {
        Some(i) => format!("🌅 Jutarnji brifing — {}\n\n{}\n\n{}", date, i, body),
        None => format!("🌅 Jutarnji brifing — {}\n\n{}", date, body),
    }
}

// ── Činjenice ────────────────────────────────────────────────────────────────

pub struct BriefingFacts {
    /// (regija, ukupno objekata, u alarmu)
    pub regions: Vec<(String, i64, i64)>,
    /// (objekt, regija, broj novih alarm zapisa u 24h)
    pub new_alarms_24h: Vec<(String, String, i64)>,
    /// (objekt, regija, najgori nivo, sažetak)
    pub active_alarms: Vec<(String, String, Option<i16>, Option<String>)>,
    /// (objekt, regija) — stanice koje ne šalju podatke
    pub silent: Vec<(String, String)>,
    /// (objekt, regija, status, poruka) — energetski rizici iz prognoze
    pub energy_risks: Vec<(String, String, String, String)>,
}

async fn gather_facts(pool: &PgPool) -> crate::errors::AppResult<BriefingFacts> {
    let regions = ndb::bot_region_status(pool).await?;
    let active_alarms = ndb::bot_active_alarms(pool).await?;

    let new_alarms_24h: Vec<(String, String, i64)> =
        sqlx::query_as::<_, (String, String, i64)>(
            "SELECT o.name, r.name, COUNT(*)
             FROM alarms a
             JOIN objects o ON o.id = a.object_id
             JOIN regions r ON r.id = o.region_id
             WHERE a.received_at >= NOW() - INTERVAL '24 hours'
               AND a.any_alarm_active = TRUE
             GROUP BY o.name, r.name
             ORDER BY COUNT(*) DESC, o.name
             LIMIT 15")
            .fetch_all(pool).await?;

    let silent: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT o.name, r.name
         FROM v_objects o
         JOIN regions r ON r.id = o.region_id
         WHERE o.is_active AND o.is_silent
         ORDER BY o.name
         LIMIT 20")
        .fetch_all(pool).await?;

    let energy_risks = crate::energy_forecast::briefing_risks(pool).await
        .unwrap_or_default();

    Ok(BriefingFacts { regions, new_alarms_24h, active_alarms, silent, energy_risks })
}

// ── Formatiranje ─────────────────────────────────────────────────────────────

fn format_report(f: &BriefingFacts) -> String {
    let mut out = String::new();

    // Pregled po regijama
    let total: i64 = f.regions.iter().map(|(_, t, _)| t).sum();
    let in_alarm: i64 = f.regions.iter().map(|(_, _, a)| a).sum();
    out.push_str(&format!("📊 Flota: {} objekata, {} u alarmu\n", total, in_alarm));
    for (name, t, a) in &f.regions {
        let mark = if *a > 0 { "🔴" } else { "🟢" };
        out.push_str(&format!("  {} {} — {}/{}\n", mark, name, a, t));
    }

    // Novi alarmi u 24h
    if f.new_alarms_24h.is_empty() {
        out.push_str("\n✅ Nema novih alarma u zadnja 24 sata.\n");
    } else {
        let total_new: i64 = f.new_alarms_24h.iter().map(|(_, _, n)| n).sum();
        out.push_str(&format!("\n🚨 Novi alarmi (24h): {} zapisa\n", total_new));
        for (obj, region, n) in f.new_alarms_24h.iter().take(8) {
            out.push_str(&format!("  • {} ({}) — {}×\n", obj, region, n));
        }
        if f.new_alarms_24h.len() > 8 {
            out.push_str(&format!("  … i još {} objekata\n", f.new_alarms_24h.len() - 8));
        }
    }

    // Trenutno aktivni alarmi
    if !f.active_alarms.is_empty() {
        out.push_str(&format!("\n🔴 Trenutno u alarmu ({}):\n", f.active_alarms.len()));
        for (obj, region, lvl, summary) in f.active_alarms.iter().take(8) {
            let l = match lvl { Some(v) => severity_label(*v), None => "Alarm" };
            out.push_str(&format!("  • {} ({}) — {}", obj, region, l));
            if let Some(s) = summary {
                out.push_str(&format!(": {}", s));
            }
            out.push('\n');
        }
        if f.active_alarms.len() > 8 {
            out.push_str(&format!("  … i još {}\n", f.active_alarms.len() - 8));
        }
    }

    // Tihe stanice
    if !f.silent.is_empty() {
        out.push_str(&format!("\n🔇 Tihe stanice ({}):\n", f.silent.len()));
        for (obj, region) in f.silent.iter().take(8) {
            out.push_str(&format!("  • {} ({})\n", obj, region));
        }
        if f.silent.len() > 8 {
            out.push_str(&format!("  … i još {}\n", f.silent.len() - 8));
        }
    }

    // Energetska prognoza
    if f.energy_risks.is_empty() {
        out.push_str("\n🔋 Energetska prognoza: nijedna stanica pod rizikom u sljedećih 7 dana.\n");
    } else {
        out.push_str(&format!("\n🔋 Energetski rizik ({} stanica):\n", f.energy_risks.len()));
        for (obj, region, status, msg) in &f.energy_risks {
            let mark = if status == "critical" { "🟥" } else { "🟧" };
            out.push_str(&format!("  {} {} ({}) — {}\n", mark, obj, region, msg));
        }
    }

    out
}

/// Kompaktna verzija činjenica za LLM uvod (bez emojija i formatiranja).
fn facts_for_llm(f: &BriefingFacts) -> String {
    let total: i64 = f.regions.iter().map(|(_, t, _)| t).sum();
    let in_alarm: i64 = f.regions.iter().map(|(_, _, a)| a).sum();
    let new_total: i64 = f.new_alarms_24h.iter().map(|(_, _, n)| n).sum();

    let mut s = format!(
        "ukupno objekata: {}\nobjekata u alarmu: {}\nnovih alarm zapisa u 24h: {}\ntihih stanica: {}\n",
        total, in_alarm, new_total, f.silent.len());

    if !f.active_alarms.is_empty() {
        let names: Vec<&str> = f.active_alarms.iter().take(5).map(|(n, ..)| n.as_str()).collect();
        s.push_str(&format!("objekti u alarmu: {}\n", names.join(", ")));
    }
    if !f.silent.is_empty() {
        let names: Vec<&str> = f.silent.iter().take(5).map(|(n, _)| n.as_str()).collect();
        s.push_str(&format!("tihe stanice: {}\n", names.join(", ")));
    }
    if f.energy_risks.is_empty() {
        s.push_str("energetska prognoza: sve stanice stabilne sljedećih 7 dana\n");
    } else {
        for (obj, _, status, msg) in &f.energy_risks {
            s.push_str(&format!("energetski rizik ({}): {} — {}\n", status, obj, msg));
        }
    }
    s
}

// ── Scheduler ────────────────────────────────────────────────────────────────

/// Pokreni dnevno slanje brifinga svim omogućenim Telegram kanalima.
/// `BRIEFING_HOUR_UTC` (default 5) određuje sat slanja; `BRIEFING_ENABLED=false`
/// isključuje. Radi samo ako je postavljen `TELEGRAM_BOT_TOKEN`.
pub fn start_scheduler(pool: PgPool) {
    let token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            tracing::info!("Brifing: TELEGRAM_BOT_TOKEN nije postavljen — dnevni brifing onemogućen");
            return;
        }
    };
    let enabled = std::env::var("BRIEFING_ENABLED")
        .map(|v| v.trim() != "false" && v.trim() != "0")
        .unwrap_or(true);
    if !enabled {
        tracing::info!("Brifing: onemogućen (BRIEFING_ENABLED)");
        return;
    }
    let hour_utc: u32 = std::env::var("BRIEFING_HOUR_UTC")
        .ok().and_then(|v| v.parse().ok())
        .map(|h: u32| h.min(23))
        .unwrap_or(5);

    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30)).build()
        {
            Ok(c) => c,
            Err(e) => { tracing::error!(error = %e, "Brifing: HTTP klijent"); return; }
        };
        tracing::info!(hour_utc, "Brifing: scheduler pokrenut");
        loop {
            tokio::time::sleep(seconds_until_hour(hour_utc)).await;
            send_to_all(&pool, &client, &token).await;
            // Preskoči preostali dio istog sata da se ne pošalje dvaput
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
}

/// Vrijeme do sljedećeg punog sata `hour_utc` (UTC).
fn seconds_until_hour(hour_utc: u32) -> std::time::Duration {
    let now = Utc::now();
    let today_target = now.date_naive().and_hms_opt(hour_utc, 0, 0).unwrap();
    let now_naive = now.naive_utc();
    let target = if now_naive < today_target {
        today_target
    } else {
        today_target + chrono::Duration::days(1)
    };
    let secs = (target - now_naive).num_seconds().max(1) as u64;
    // Sanity: nikad više od 24h + 1min
    std::time::Duration::from_secs(secs.min(86_460))
}

/// Generiraj brifing i pošalji ga svim omogućenim Telegram kanalima.
pub async fn send_to_all(pool: &PgPool, client: &reqwest::Client, token: &str) {
    let chat_ids = match ndb::bot_authorized_chat_ids(pool).await {
        Ok(ids) if !ids.is_empty() => ids,
        Ok(_) => {
            tracing::info!("Brifing: nema omogućenih Telegram kanala — preskačem");
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, "Brifing: dohvat kanala nije uspio");
            return;
        }
    };

    let text = generate(pool, client).await;
    for chat_id in &chat_ids {
        match crate::telegram::send_message(client, token, chat_id, &text).await {
            Ok(()) => {
                let _ = ndb::insert_log(
                    pool, None, Some("telegram"), None, None, None, None,
                    "briefing", "sent", None, Some(&text)).await;
            }
            Err(e) => {
                tracing::warn!(chat_id = %chat_id, error = %e, "Brifing: slanje nije uspjelo");
                let _ = ndb::insert_log(
                    pool, None, Some("telegram"), None, None, None, None,
                    "briefing", "error", Some(&e.to_string()), None).await;
            }
        }
    }
    tracing::info!(channels = chat_ids.len(), "Brifing: poslan");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_formats_empty_state() {
        let f = BriefingFacts {
            regions: vec![("Sjever".into(), 10, 0)],
            new_alarms_24h: vec![],
            active_alarms: vec![],
            silent: vec![],
            energy_risks: vec![],
        };
        let r = format_report(&f);
        assert!(r.contains("10 objekata, 0 u alarmu"));
        assert!(r.contains("Nema novih alarma"));
        assert!(r.contains("nijedna stanica pod rizikom"));
    }

    #[test]
    fn report_lists_risks_and_silent() {
        let f = BriefingFacts {
            regions: vec![("Jug".into(), 5, 1)],
            new_alarms_24h: vec![("Galija".into(), "Jug".into(), 3)],
            active_alarms: vec![("Galija".into(), "Jug".into(), Some(3), Some("Baterija".into()))],
            silent: vec![("Umag".into(), "Jug".into())],
            energy_risks: vec![("Sv. Andrija".into(), "Jug".into(), "warning".into(),
                                "Napon pada ispod 11.5 V oko 31.07.".into())],
        };
        let r = format_report(&f);
        assert!(r.contains("Galija (Jug) — 3×"));
        assert!(r.contains("🔇 Tihe stanice (1)"));
        assert!(r.contains("Sv. Andrija"));
        assert!(r.contains("11.5 V"));
    }
}
