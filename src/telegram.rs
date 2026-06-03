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
//!   /ai <pitanje>  — eksplicitan upit prirodnim jezikom
//!   /pomoc         — popis komandi
//!
//! Uz to, ako je postavljen `LLM_API_KEY`, bot prima i slobodan tekst (bez "/")
//! te ga preko besplatnog LLM-a (vidi `llm.rs`) pretvori u odgovarajuću komandu.
//! Npr. „koliki je napon baterije na objektu Barbarinac?“ → /objekt Barbarinac.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::db::notify as ndb;
use crate::notify::severity_label;

/// Red koji vrati `bot_find_object`: (ime, regija, alarm_active, najgori_nivo,
/// sažetak, broj_alarma, napon_baterije, svjetlo_aktivno, vrijeme_zadnjeg).
type ObjectRow = (String, String, bool, Option<i16>, Option<String>, i16,
                  Option<f32>, Option<f32>, Option<DateTime<Utc>>);

pub fn start_bot(pool: PgPool) {
    let token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
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

    // Provjeri token
    match get_me(&client, &token).await {
        Ok(name) => tracing::info!(bot = %name, "Telegram bot: token ispravan"),
        Err(e) => {
            tracing::error!(error = %e, "Telegram bot: neispravan TELEGRAM_BOT_TOKEN — bot se gasi");
            return;
        }
    }

    // Obriši eventualni webhook (webhook blokira getUpdates → tišina)
    match delete_webhook(&client, &token).await {
        Ok(true)  => tracing::info!("Telegram bot: postojeći webhook obrisan (sad koristim long-polling)"),
        Ok(false) => {}
        Err(e)    => tracing::warn!(error = %e, "Telegram bot: deleteWebhook nije uspio"),
    }

    tracing::info!("Telegram bot (dvosmjerna komunikacija) pokrenut");

    // Preskoči backlog (poruke poslane prije pokretanja) preko offseta —
    // bez oslanjanja na server-uru. offset=-1 vraća samo zadnji update.
    let mut offset: i64 = 0;
    if let Ok(updates) = get_updates(&client, &token, -1, 0).await {
        if let Some(last) = updates.last()
            .and_then(|u| u.get("update_id")).and_then(|v| v.as_i64())
        {
            offset = last + 1;
            tracing::info!(skipped_to = offset, "Telegram bot: preskočen backlog poruka");
        }
    }

    loop {
        match get_updates(&client, &token, offset, 30).await {
            Ok(updates) => {
                for upd in updates {
                    let update_id = upd.get("update_id").and_then(|v| v.as_i64()).unwrap_or(0);
                    if update_id >= offset { offset = update_id + 1; }
                    if let Err(e) = handle_update(&pool, &client, &token, &upd).await {
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

async fn get_me(client: &reqwest::Client, token: &str) -> anyhow::Result<String> {
    let url = format!("https://api.telegram.org/bot{}/getMe", token);
    let body: Value = client.get(&url).send().await?.json().await?;
    if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        anyhow::bail!("getMe odgovor: {}", body);
    }
    Ok(body.pointer("/result/username").and_then(|v| v.as_str()).unwrap_or("?").to_string())
}

async fn delete_webhook(client: &reqwest::Client, token: &str) -> anyhow::Result<bool> {
    // Prvo provjeri je li webhook uopće postavljen
    let info_url = format!("https://api.telegram.org/bot{}/getWebhookInfo", token);
    let info: Value = client.get(&info_url).send().await?.json().await?;
    let has_webhook = info.pointer("/result/url")
        .and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
    if !has_webhook { return Ok(false); }

    let url = format!("https://api.telegram.org/bot{}/deleteWebhook", token);
    client.get(&url).send().await?;
    Ok(true)
}

async fn get_updates(client: &reqwest::Client, token: &str, offset: i64, timeout: i64)
    -> anyhow::Result<Vec<Value>>
{
    let url = format!("https://api.telegram.org/bot{}/getUpdates", token);
    let resp = client.get(&url)
        .query(&[("offset", offset.to_string()), ("timeout", timeout.to_string())])
        .send().await?;
    let body: Value = resp.json().await?;
    if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        anyhow::bail!("Telegram API odgovor: {}", body);
    }
    Ok(body.get("result").and_then(|v| v.as_array()).cloned().unwrap_or_default())
}

async fn handle_update(
    pool: &PgPool, client: &reqwest::Client, token: &str, upd: &Value,
) -> anyhow::Result<()> {
    let msg = match upd.get("message") { Some(m) => m, None => return Ok(()) };

    let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if text.is_empty() { return Ok(()); }

    let chat_id = match msg.get("chat").and_then(|c| c.get("id")) {
        Some(v) => v.to_string().trim_matches('"').to_string(),
        None => return Ok(()),
    };

    tracing::info!(chat_id = %chat_id, cmd = %text, "Telegram bot: primljena komanda");

    // Autorizacija — samo registrirani Telegram kanali
    let authorized = ndb::bot_authorized_chat_ids(pool).await.unwrap_or_default();
    if !authorized.iter().any(|c| c == &chat_id) {
        tracing::info!(chat_id = %chat_id, "Telegram bot: neovlašten chat — odbijeno");
        let reply = format!(
            "⛔ Nemate pristup ovom botu.\nVaš chat ID: {}\nZatražite od administratora da vas doda kao Telegram kanal za obavijesti.",
            chat_id
        );
        send_message(client, token, &chat_id, &reply).await?;
        return Ok(());
    }

    let reply = handle_command(pool, client, &text).await;
    send_message(client, token, &chat_id, &reply).await?;
    Ok(())
}

// ── Komande ───────────────────────────────────────────────────────────────────

async fn handle_command(pool: &PgPool, client: &reqwest::Client, text: &str) -> String {
    // Slobodan tekst (ne počinje s "/") → natural-language upit preko LLM-a.
    if !text.starts_with('/') {
        return handle_natural_language(pool, client, text).await;
    }

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
        // Eksplicitni AI upit: /ai <pitanje> ili /pitaj <pitanje>
        "/ai" | "/pitaj" | "/ask" => {
            if arg.is_empty() {
                "Koristite: /ai <pitanje>\nNpr. /ai koliki je napon baterije na objektu Barbarinac".to_string()
            } else {
                handle_natural_language(pool, client, arg).await
            }
        }
        _ => format!("Nepoznata komanda: {}\n\n{}", cmd_raw, help_text()),
    }
}

/// Obradi slobodan tekst: LLM-om ga pretvori u namjeru pa izvrši odgovarajuću
/// komandu. Podaci se uvijek dohvaćaju iz baze — LLM samo usmjerava upit.
async fn handle_natural_language(pool: &PgPool, client: &reqwest::Client, text: &str) -> String {
    if !crate::llm::is_enabled() {
        return format!(
            "🤔 Razumijem samo komande (AI tumačenje upita nije uključeno).\n\n{}",
            help_text()
        );
    }

    let intent = match crate::llm::interpret(client, text).await {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(error = %e, "Telegram bot: LLM tumačenje nije uspjelo");
            return format!(
                "⚠️ Trenutno ne mogu protumačiti upit. Pokušajte komandom.\n\n{}",
                help_text()
            );
        }
    };

    tracing::info!(
        action = %intent.action, object = ?intent.object, focus = %intent.focus,
        "Telegram bot: LLM namjera"
    );

    match intent.action.as_str() {
        "status" => cmd_status(pool).await,
        "alarmi" => cmd_alarms(pool).await,
        "pomoc"  => help_text(),
        "objekt" => match intent.object.as_deref() {
            Some(name) => answer_object_nl(pool, client, text, name, &intent.focus).await,
            None => "Na koji objekt mislite? Npr. „napon baterije na objektu Galija“.".to_string(),
        },
        _ => format!(
            "🤔 Nisam siguran što tražite.\n\n{}",
            help_text()
        ),
    }
}

fn help_text() -> String {
    let mut t = String::from(
        "🤖 Beacon nadzor — dostupne komande:\n\n\
         /status — sažetak po regijama\n\
         /alarmi — trenutno aktivni alarmi\n\
         /objekt <ime> — stanje objekta (npr. /objekt Galija)\n\
         /pomoc — ova poruka");
    if crate::llm::is_enabled() {
        t.push_str(
            "\n\n💬 Možete pitati i običnim jezikom, npr.:\n\
             • „koliki je napon baterije na objektu Barbarinac?“\n\
             • „je li Galija u alarmu?“\n\
             • „daj mi pregled stanja“");
    }
    t
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
        Ok(Some(row)) => format_object_card(&row),
        Ok(None) => format!("Objekt \"{}\" nije pronađen.", query),
        Err(e) => { tracing::warn!(error = %e, "bot /objekt"); "Greška pri dohvaćanju objekta.".to_string() }
    }
}

/// Puna kartica objekta (deterministički format, koristi se za /objekt i za
/// opća NL pitanja gdje korisnik traži „sve“).
fn format_object_card(row: &ObjectRow) -> String {
    let (name, region, active, lvl, summary, count, volt, light, ts) = row;
    let mut out = format!("📍 {} ({})\n\n", name, region);
    out.push_str(&format!("Stanje: {}\n", if *active { "🔴 ALARM" } else { "🟢 OK" }));
    if *active {
        let l = match lvl { Some(v) => severity_label(*v), None => "Alarm" };
        out.push_str(&format!("Nivo: {} ({} aktivnih)\n", l, count));
        if let Some(sm) = summary { out.push_str(&format!("Opis: {}\n", sm)); }
    }
    if let Some(v) = volt { out.push_str(&format!("Baterija: {:.2} V\n", v)); }
    if let Some(la) = light { out.push_str(&format!("Svjetlo aktivno: {:.0} %\n", la * 100.0)); }
    if let Some(t) = ts { out.push_str(&format!("Zadnje mjerenje: {}\n", t.format("%d.%m.%Y %H:%M UTC"))); }
    out
}

/// NL odgovor o pojedinom objektu (hibrid): podatke uvijek vučemo iz baze,
/// složimo točne činjenice prema fokusu, a LLM ih samo „uglača“ u prirodnu
/// rečenicu. Ako 2. LLM poziv padne, vraćamo same (točne) činjenice.
async fn answer_object_nl(
    pool: &PgPool, client: &reqwest::Client, question: &str, query: &str, focus: &str,
) -> String {
    let row = match ndb::bot_find_object(pool, query).await {
        Ok(Some(r)) => r,
        Ok(None) => return format!("Objekt \"{}\" nije pronađen.", query),
        Err(e) => {
            tracing::warn!(error = %e, "bot nl objekt");
            return "Greška pri dohvaćanju objekta.".to_string();
        }
    };

    // Opće pitanje → puna kartica, bez dodatnog LLM poziva.
    if focus == "sve" {
        return format_object_card(&row);
    }

    let facts = build_object_facts(&row, focus);

    match crate::llm::phrase_answer(client, question, &facts).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "Telegram bot: phrase_answer pao — vraćam činjenice");
            facts
        }
    }
}

/// Iz reda baze složi precizne, ljudski čitljive činjenice za zadani fokus.
/// Ovo je „izvor istine“ koji LLM smije samo preformulirati, ne mijenjati.
fn build_object_facts(row: &ObjectRow, focus: &str) -> String {
    let (name, region, active, lvl, summary, count, volt, light, ts) = row;
    let mut f = format!("objekt: {}\nregija: {}\n", name, region);
    match focus {
        "svjetlo" => match light {
            Some(la) => {
                let radi = *la > 0.0;
                f.push_str(&format!(
                    "svjetlo (lanterna): {} — aktivno {:.0}% vremena u zadnjem mjerenju\n",
                    if radi { "radi/upaljeno" } else { "ne radi/ugašeno" }, la * 100.0));
            }
            None => f.push_str("svjetlo: nema podatka\n"),
        },
        "baterija" => match volt {
            Some(v) => f.push_str(&format!("napon baterije: {:.2} V\n", v)),
            None => f.push_str("napon baterije: nema podatka\n"),
        },
        "alarm" => {
            if *active {
                let l = match lvl { Some(v) => severity_label(*v), None => "Alarm" };
                f.push_str(&format!("alarm: DA — nivo {}, broj aktivnih: {}\n", l, count));
                if let Some(sm) = summary { f.push_str(&format!("opis alarma: {}\n", sm)); }
            } else {
                f.push_str("alarm: NE (objekt nije u alarmu)\n");
            }
        }
        "mjerenje" => {
            match ts {
                Some(t) => f.push_str(&format!("zadnje mjerenje: {}\n", t.format("%d.%m.%Y %H:%M UTC"))),
                None => f.push_str("zadnje mjerenje: nema podatka\n"),
            }
            if let Some(v) = volt { f.push_str(&format!("napon baterije: {:.2} V\n", v)); }
            f.push_str(&format!("alarm: {}\n", if *active { "DA" } else { "NE" }));
        }
        _ => {}
    }
    f
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
