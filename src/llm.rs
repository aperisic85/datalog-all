//! LLM sloj — pretvara slobodan (natural-language) tekst u strukturiranu
//! bot-namjeru (intent), koju Telegram bot zatim mapira na postojeće komande.
//!
//! Koristi OpenAI-kompatibilan `chat/completions` endpoint, pa radi s više
//! pružatelja koji nude **besplatni** tier, npr.:
//!   • Groq        — https://api.groq.com/openai/v1/chat/completions
//!                   (besplatan API ključ, brzo; npr. model `llama-3.3-70b-versatile`)
//!   • OpenRouter  — https://openrouter.ai/api/v1/chat/completions
//!                   (ima besplatne `:free` modele)
//!   • Google Gemini (OpenAI-compat) — .../v1beta/openai/chat/completions
//!
//! Uključuje se SAMO ako je postavljena env varijabla `LLM_API_KEY`.
//!
//! VAŽNO: model nikad ne izmišlja podatke. On samo prepoznaje ŠTO korisnik
//! pita (akciju) i o KOJEM objektu — sve stvarne vrijednosti (napon baterije,
//! alarmi…) dohvaćaju se iz baze. Tako nema halucinacija mjernih vrijednosti.

use serde_json::{json, Value};

const DEFAULT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

/// Strukturirana namjera koju vrati LLM.
#[derive(Debug, Clone)]
pub struct BotIntent {
    /// "status" | "alarmi" | "objekt" | "pomoc" | "nepoznato"
    pub action: String,
    /// Ime (ili dio imena) objekta — relevantno samo za action = "objekt".
    pub object: Option<String>,
}

/// Je li NL sloj omogućen (postoji li API ključ).
pub fn is_enabled() -> bool {
    std::env::var("LLM_API_KEY").map(|k| !k.trim().is_empty()).unwrap_or(false)
}

fn config() -> (String, String, String) {
    let key = std::env::var("LLM_API_KEY").unwrap_or_default().trim().to_string();
    let url = std::env::var("LLM_API_URL")
        .ok().filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_URL.to_string());
    let model = std::env::var("LLM_MODEL")
        .ok().filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    (key, url, model)
}

const SYSTEM_PROMPT: &str = "\
Ti si usmjerivač upita (router) za nadzorni sustav beacona/objekata.
Korisnik na hrvatskom jeziku postavlja pitanja preko Telegrama. Tvoj jedini
zadatak je prepoznati namjeru i, ako je riječ o pojedinom objektu, izvući
njegovo ime. NE izmišljaj i NE navodi nikakve mjerne vrijednosti — podatke
dohvaća sustav iz baze.

Odgovori ISKLJUČIVO jednim JSON objektom (bez markdowna, bez objašnjenja):
{\"action\": \"<akcija>\", \"object\": \"<ime objekta ili null>\"}

Dozvoljene akcije:
- \"status\"   — opći pregled stanja, koliko je objekata u alarmu, sažetak po regijama.
- \"alarmi\"   — popis trenutno aktivnih alarma.
- \"objekt\"   — pitanje o JEDNOM objektu (napon baterije, je li u alarmu,
                svjetlo, zadnje mjerenje itd.). U \"object\" stavi ime objekta.
- \"pomoc\"    — korisnik traži pomoć ili popis mogućnosti.
- \"nepoznato\"— ne možeš razaznati namjeru.

Primjeri:
\"koliki je sad napon baterije na objektu Barbarinac?\" -> {\"action\":\"objekt\",\"object\":\"Barbarinac\"}
\"je li Galija u alarmu\" -> {\"action\":\"objekt\",\"object\":\"Galija\"}
\"daj mi stanje sustava\" -> {\"action\":\"status\",\"object\":null}
\"koji su aktivni alarmi\" -> {\"action\":\"alarmi\",\"object\":null}
\"što sve znaš\" -> {\"action\":\"pomoc\",\"object\":null}";

/// Pretvori slobodan tekst u `BotIntent`. Vraća Err kod mrežne/API greške.
pub async fn interpret(client: &reqwest::Client, text: &str) -> anyhow::Result<BotIntent> {
    let (key, url, model) = config();
    if key.is_empty() {
        anyhow::bail!("LLM_API_KEY nije postavljen");
    }

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user",   "content": text }
        ],
        // Većina OpenAI-kompatibilnih providera (Groq, OpenAI) podržava ovo.
        // Ako provider ignorira polje, i dalje radimo zahvaljujući robustnom
        // izvlačenju JSON-a iz odgovora.
        "response_format": { "type": "json_object" }
    });

    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", key))
        .json(&payload)
        .send().await?;

    let status = resp.status();
    let body: Value = resp.json().await?;
    if !status.is_success() {
        anyhow::bail!("LLM API greška ({}): {}", status, body);
    }

    let content = body
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("LLM odgovor bez sadržaja: {}", body))?;

    parse_intent(content)
}

/// Iz teksta odgovora izvuci JSON i mapiraj na `BotIntent`. Tolerantno na
/// ```json ... ``` ogradu i na suvišni tekst oko JSON-a.
fn parse_intent(content: &str) -> anyhow::Result<BotIntent> {
    let json_str = extract_json(content)
        .ok_or_else(|| anyhow::anyhow!("LLM nije vratio JSON: {}", content))?;
    let v: Value = serde_json::from_str(&json_str)
        .map_err(|e| anyhow::anyhow!("neispravan JSON iz LLM-a: {} ({})", e, json_str))?;

    let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("nepoznato")
        .trim().to_lowercase();
    let action = match action.as_str() {
        "status" | "alarmi" | "objekt" | "pomoc" => action,
        // tolerancija na sinonime/varijante
        "object" => "objekt".to_string(),
        "alarms" | "alarm" => "alarmi".to_string(),
        "help" | "pomoć" => "pomoc".to_string(),
        _ => "nepoznato".to_string(),
    };

    let object = v.get("object")
        .and_then(|o| o.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));

    Ok(BotIntent { action, object })
}

/// Vrati prvi izbalansirani `{ ... }` blok iz teksta.
fn extract_json(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, ch) in s[start..].char_indices() {
        if in_str {
            if esc { esc = false; }
            else if ch == '\\' { esc = true; }
            else if ch == '"' { in_str = false; }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..start + i + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let i = parse_intent(r#"{"action":"objekt","object":"Barbarinac"}"#).unwrap();
        assert_eq!(i.action, "objekt");
        assert_eq!(i.object.as_deref(), Some("Barbarinac"));
    }

    #[test]
    fn parses_fenced_json() {
        let i = parse_intent("```json\n{\"action\":\"status\",\"object\":null}\n```").unwrap();
        assert_eq!(i.action, "status");
        assert!(i.object.is_none());
    }

    #[test]
    fn parses_with_surrounding_text() {
        let i = parse_intent("Evo: {\"action\":\"alarmi\", \"object\": \"\"} hvala").unwrap();
        assert_eq!(i.action, "alarmi");
        assert!(i.object.is_none());
    }

    #[test]
    fn maps_synonyms() {
        let i = parse_intent(r#"{"action":"object","object":"Galija"}"#).unwrap();
        assert_eq!(i.action, "objekt");
    }

    #[test]
    fn unknown_on_missing() {
        let i = parse_intent(r#"{"foo":"bar"}"#).unwrap();
        assert_eq!(i.action, "nepoznato");
    }
}
