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
    /// O čemu se konkretno pita kod action = "objekt":
    /// "svjetlo" | "baterija" | "alarm" | "mjerenje" | "sve".
    pub focus: String,
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
{\"action\": \"<akcija>\", \"object\": \"<ime objekta ili null>\", \"focus\": \"<fokus>\"}

Dozvoljene akcije:
- \"status\"   — opći pregled stanja, koliko je objekata u alarmu, sažetak po regijama.
- \"alarmi\"   — popis trenutno aktivnih alarma.
- \"objekt\"   — pitanje o JEDNOM objektu. U \"object\" stavi ime objekta.
- \"brifing\"  — korisnik traži jutarnji brifing / dnevni izvještaj / pregled zadnja 24 sata.
- \"pomoc\"    — korisnik traži pomoć ili popis mogućnosti.
- \"nepoznato\"— ne možeš razaznati namjeru.

Polje \"focus\" (na što se pitanje odnosi kod action=objekt; inače \"sve\"):
- \"svjetlo\"  — radi li/je li upaljeno svjetlo, lanterna.
- \"baterija\" — napon/stanje baterije.
- \"alarm\"    — je li objekt u alarmu, kakav alarm.
- \"mjerenje\" — kad je zadnje mjerenje, općenito stanje.
- \"sve\"      — opće pitanje o objektu ili nije jasno.

Primjeri:
\"koliki je sad napon baterije na objektu Barbarinac?\" -> {\"action\":\"objekt\",\"object\":\"Barbarinac\",\"focus\":\"baterija\"}
\"radi li svjetlo na objektu Umag\" -> {\"action\":\"objekt\",\"object\":\"Umag\",\"focus\":\"svjetlo\"}
\"je li Galija u alarmu\" -> {\"action\":\"objekt\",\"object\":\"Galija\",\"focus\":\"alarm\"}
\"kad je zadnje mjerenje na Drveniku\" -> {\"action\":\"objekt\",\"object\":\"Drvenik\",\"focus\":\"mjerenje\"}
\"reci mi sve o objektu Galija\" -> {\"action\":\"objekt\",\"object\":\"Galija\",\"focus\":\"sve\"}
\"daj mi stanje sustava\" -> {\"action\":\"status\",\"object\":null,\"focus\":\"sve\"}
\"daj mi jutarnji brifing\" -> {\"action\":\"brifing\",\"object\":null,\"focus\":\"sve\"}
\"što se dogodilo preko noći\" -> {\"action\":\"brifing\",\"object\":null,\"focus\":\"sve\"}
\"koji su aktivni alarmi\" -> {\"action\":\"alarmi\",\"object\":null,\"focus\":\"sve\"}
\"što sve znaš\" -> {\"action\":\"pomoc\",\"object\":null,\"focus\":\"sve\"}";

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
        "status" | "alarmi" | "objekt" | "pomoc" | "brifing" => action,
        // tolerancija na sinonime/varijante
        "object" => "objekt".to_string(),
        "alarms" | "alarm" => "alarmi".to_string(),
        "help" | "pomoć" => "pomoc".to_string(),
        "briefing" => "brifing".to_string(),
        _ => "nepoznato".to_string(),
    };

    let object = v.get("object")
        .and_then(|o| o.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));

    let focus = v.get("focus").and_then(|f| f.as_str()).unwrap_or("sve")
        .trim().to_lowercase();
    let focus = match focus.as_str() {
        "svjetlo" | "baterija" | "alarm" | "mjerenje" | "sve" => focus,
        "light" => "svjetlo".to_string(),
        "battery" => "baterija".to_string(),
        "measurement" => "mjerenje".to_string(),
        _ => "sve".to_string(),
    };

    Ok(BotIntent { action, object, focus })
}

const PHRASE_SYSTEM: &str = "\
Ti si asistent nadzornog sustava beacona. Na temelju zadanih ČINJENICA sastavi
JEDAN kratak, prirodan odgovor na hrvatskom jeziku na korisnikovo pitanje.

Stroga pravila:
- Koristi ISKLJUČIVO vrijednosti iz danih činjenica.
- NE izmišljaj i NE mijenjaj brojeve; prepiši ih točno onako kako su zadani.
- Ne dodaji informacije kojih nema u činjenicama.
- Odgovori izravno na pitanje (npr. na „radi li…\" počni s „Radi…\" ili „Ne radi…\").
- Bez markdowna, bez uvoda i bez objašnjenja — samo jedna rečenica.";

/// Drugi (opcionalni) LLM poziv: od gotovih, autoritativnih činjenica iz baze
/// sastavi prirodnu rečenicu kao odgovor na korisnikovo pitanje. Model NE dobiva
/// pristup ničemu osim već-složenih činjenica, pa ne može izmisliti vrijednosti.
pub async fn phrase_answer(client: &reqwest::Client, question: &str, facts: &str)
    -> anyhow::Result<String>
{
    let (key, url, model) = config();
    if key.is_empty() {
        anyhow::bail!("LLM_API_KEY nije postavljen");
    }

    let payload = json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": PHRASE_SYSTEM },
            { "role": "user",
              "content": format!("Pitanje korisnika:\n{}\n\nČinjenice:\n{}", question, facts) }
        ]
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
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("LLM odgovor bez sadržaja: {}", body))?;

    Ok(content)
}

const BRIEFING_SYSTEM: &str = "\
Ti si asistent nadzornog sustava pomorskih navigacijskih objekata. Na temelju
zadanih ČINJENICA napiši kratak uvodni komentar (2-3 rečenice) za jutarnji
brifing operaterima, na hrvatskom jeziku.

Stroga pravila:
- Koristi ISKLJUČIVO vrijednosti iz danih činjenica; NE izmišljaj i NE
  mijenjaj brojeve ni imena objekata.
- Istakni ono najvažnije: nove alarme, tihe stanice i energetske rizike.
- Ako je sve u redu, reci to kratko i mirno.
- Bez markdowna, bez naslova, bez nabrajanja — samo 2-3 tečne rečenice.";

/// Treći (opcionalni) LLM poziv: iz složenih činjenica za jutarnji brifing
/// napiši kratak uvodni komentar. Kao i drugdje, model ne može izmisliti
/// vrijednosti — dobiva samo već-složene činjenice iz baze.
pub async fn phrase_briefing(client: &reqwest::Client, facts: &str) -> anyhow::Result<String> {
    let (key, url, model) = config();
    if key.is_empty() {
        anyhow::bail!("LLM_API_KEY nije postavljen");
    }

    let payload = json!({
        "model": model,
        "temperature": 0.3,
        "messages": [
            { "role": "system", "content": BRIEFING_SYSTEM },
            { "role": "user",   "content": format!("Činjenice:\n{}", facts) }
        ]
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

    body.pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("LLM odgovor bez sadržaja: {}", body))
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
    fn parses_focus() {
        let i = parse_intent(r#"{"action":"objekt","object":"Umag","focus":"svjetlo"}"#).unwrap();
        assert_eq!(i.focus, "svjetlo");
    }

    #[test]
    fn focus_defaults_to_sve() {
        let i = parse_intent(r#"{"action":"objekt","object":"Umag"}"#).unwrap();
        assert_eq!(i.focus, "sve");
        let i2 = parse_intent(r#"{"action":"objekt","object":"Umag","focus":"bla"}"#).unwrap();
        assert_eq!(i2.focus, "sve");
    }

    #[test]
    fn unknown_on_missing() {
        let i = parse_intent(r#"{"foo":"bar"}"#).unwrap();
        assert_eq!(i.action, "nepoznato");
    }
}
