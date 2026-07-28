//! Open-Meteo API klijent i izračun solarne efikasnosti.
//!
//! Koristi besplatni Open-Meteo API za dohvat satnih podataka o:
//!   - globalnom solarnom zračenju (shortwave_radiation, W/m²)
//!   - oblačnosti (cloud_cover, %)
//!   - brzini vjetra (wind_speed_10m, km/h)
//!   - padalinama (precipitation, mm)
//!   - temperaturi zraka (temperature_2m, °C)
//!
//! Solarni score:
//!   Za svaki sat kada je iradijancija > MIN_IRRADIANCE_THRESHOLD i postoji mjerenje
//!   solar_voltage_avg, računamo omjer voltage/irradiance (proxy efikasnosti).
//!   Score = (prosjek zadnjih 7 dana / prosjek zadnjih 30 dana) * 100
//!   Pad ispod EFFICIENCY_WARN_THRESHOLD (%) signalizira prljav/oštećen panel.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Minimalna iradijancija (W/m²) za uključivanje u izračun efikasnosti
pub const MIN_IRRADIANCE_THRESHOLD: f64 = 80.0;
/// Score ispod kojeg upozorenje (panel možda prljav/oštećen)
pub const EFFICIENCY_WARN_THRESHOLD: f64 = 75.0;
/// Score ispod kojeg kritično upozorenje
pub const EFFICIENCY_CRIT_THRESHOLD: f64 = 55.0;

// ── Open-Meteo API odgovori ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OpenMeteoResponse {
    pub hourly: OpenMeteoHourly,
    #[serde(default)]
    pub timezone: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenMeteoHourly {
    pub time:                Vec<String>,
    pub shortwave_radiation: Vec<Option<f64>>,
    pub cloud_cover:         Vec<Option<f64>>,
    pub wind_speed_10m:      Vec<Option<f64>>,
    pub precipitation:       Vec<Option<f64>>,
    pub temperature_2m:      Vec<Option<f64>>,
}

// ── Naš output ────────────────────────────────────────────────────────────────

/// Jedan sat vremenskih podataka
#[derive(Debug, Clone, Serialize)]
pub struct WeatherHour {
    pub time:                 DateTime<Utc>,
    pub shortwave_radiation:  Option<f64>,
    pub cloud_cover:          Option<f64>,
    pub wind_speed_10m:       Option<f64>,
    pub precipitation:        Option<f64>,
    pub temperature_2m:       Option<f64>,
}

/// Odgovor za GET /objects/:id/weather
#[derive(Debug, Serialize)]
pub struct WeatherResponse {
    pub latitude:  f64,
    pub longitude: f64,
    pub timezone:  String,
    pub hours:     Vec<WeatherHour>,
}

/// Dnevni solarni score
#[derive(Debug, Serialize)]
pub struct SolarDayScore {
    pub date:              String,
    /// Dnevna insolacija u kWh/m² (zbroj hourly W/m² / 1000)
    pub insolation_kwh:    f64,
    /// Relativni score (0–100) za taj dan (NaN → null)
    pub score:             Option<f64>,
    /// Broj uzoraka upotrijebljenih za taj dan
    pub sample_count:      u32,
}

/// Odgovor za GET /objects/:id/solar-efficiency
#[derive(Debug, Serialize)]
pub struct SolarEfficiencyResponse {
    pub object_id:              uuid::Uuid,
    pub computed_at:            DateTime<Utc>,
    /// Ukupni score 0–100 (recent/baseline * 100)
    pub score:                  Option<f64>,
    /// "good" | "warn" | "critical" | "insufficient_data"
    pub status:                 String,
    pub status_label:           String,
    /// Poruka za korisnike
    pub message:                String,
    /// Prosječni omjer voltage/irradiance za bazno razdoblje (30 dana)
    pub baseline_ratio:         Option<f64>,
    /// Prosječni omjer za zadnjih 7 dana
    pub recent_ratio:           Option<f64>,
    pub sample_count_baseline:  usize,
    pub sample_count_recent:    usize,
    pub daily_scores:           Vec<SolarDayScore>,
}

// ── API poziv ─────────────────────────────────────────────────────────────────

/// Dohvati satne vremenske podatke s Open-Meteo za zadane koordinate.
/// `past_days`: koliko dana unatrag (max 92 za besplatni API).
pub async fn fetch_weather(
    lat: f64,
    lon: f64,
    past_days: u32,
) -> Result<WeatherResponse, String> {
    fetch_weather_range(lat, lon, past_days, 1).await
}

/// Kao [`fetch_weather`], ali s konfiguririvim brojem dana prognoze unaprijed
/// (max 16 za besplatni API). Koristi se za energetsku prognozu, gdje trebamo
/// i povijest (za učenje omjera punjenja) i prognozu iradijancije.
pub async fn fetch_weather_range(
    lat: f64,
    lon: f64,
    past_days: u32,
    forecast_days: u32,
) -> Result<WeatherResponse, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat:.5}&longitude={lon:.5}\
         &hourly=shortwave_radiation,cloud_cover,wind_speed_10m,precipitation,temperature_2m\
         &timezone=auto\
         &past_days={past_days}\
         &forecast_days={forecast_days}"
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp: OpenMeteoResponse = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Open-Meteo request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Open-Meteo JSON parse error: {e}"))?;

    let hours = parse_hours(&resp.hourly);

    Ok(WeatherResponse {
        latitude:  lat,
        longitude: lon,
        timezone:  resp.timezone,
        hours,
    })
}

/// Pretvori Open-Meteo format (ISO8601 bez sekundi) u Vec<WeatherHour>
fn parse_hours(hourly: &OpenMeteoHourly) -> Vec<WeatherHour> {
    hourly
        .time
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            // Format: "2024-01-15T13:00"
            let naive = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M").ok()?;
            let time = Utc.from_utc_datetime(&naive);
            Some(WeatherHour {
                time,
                shortwave_radiation: hourly.shortwave_radiation.get(i).copied().flatten(),
                cloud_cover:         hourly.cloud_cover.get(i).copied().flatten(),
                wind_speed_10m:      hourly.wind_speed_10m.get(i).copied().flatten(),
                precipitation:       hourly.precipitation.get(i).copied().flatten(),
                temperature_2m:      hourly.temperature_2m.get(i).copied().flatten(),
            })
        })
        .collect()
}

// ── Izračun solarnog score-a ──────────────────────────────────────────────────

/// Jedan par (sat mjerenja, iradijancija) za izračun efikasnosti
pub struct EfficiencyPoint {
    pub date_str:           String,        // "YYYY-MM-DD"
    pub solar_voltage:      f64,           // V
    pub irradiance:         f64,           // W/m²
}

/// Izračunaj solarni score iz pariranih točaka.
/// Vraća (score 0–100, status, label, poruka, baseline_ratio, recent_ratio,
///         sample_count_baseline, sample_count_recent, dnevni scores)
pub fn compute_solar_efficiency(
    points: &[EfficiencyPoint],
    weather_hours: &[WeatherHour],
) -> SolarEfficiencyResult {
    // Grupiraj po datumu za dnevni prikaz
    use std::collections::HashMap;

    // Baseline = svi uzorci (do 30 dana)
    let baseline_ratios: Vec<f64> = points
        .iter()
        .filter(|p| p.irradiance >= MIN_IRRADIANCE_THRESHOLD && p.solar_voltage > 0.0)
        .map(|p| p.solar_voltage / p.irradiance)
        .collect();

    // Recent = zadnjih 7 dana
    let cutoff_date = {
        let now = Utc::now();
        (now - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string()
    };
    let recent_ratios: Vec<f64> = points
        .iter()
        .filter(|p| p.date_str >= cutoff_date && p.irradiance >= MIN_IRRADIANCE_THRESHOLD && p.solar_voltage > 0.0)
        .map(|p| p.solar_voltage / p.irradiance)
        .collect();

    let baseline_ratio = if baseline_ratios.is_empty() {
        None
    } else {
        Some(baseline_ratios.iter().sum::<f64>() / baseline_ratios.len() as f64)
    };

    let recent_ratio = if recent_ratios.is_empty() {
        None
    } else {
        Some(recent_ratios.iter().sum::<f64>() / recent_ratios.len() as f64)
    };

    let score = match (baseline_ratio, recent_ratio) {
        (Some(b), Some(r)) if b > 0.0 => Some((r / b * 100.0).min(120.0)),
        _ => None,
    };

    let (status, status_label, message) = classify_score(score, baseline_ratios.len(), recent_ratios.len());

    // Dnevni scores — grupiraj po datumu
    let mut daily_map: HashMap<String, (f64, f64, u32)> = HashMap::new(); // date → (sum_irr, sum_volt, count)

    // Dnevna insolacija iz weather_hours
    let mut daily_insolation: HashMap<String, f64> = HashMap::new();
    for wh in weather_hours {
        let date = wh.time.format("%Y-%m-%d").to_string();
        if let Some(irr) = wh.shortwave_radiation {
            *daily_insolation.entry(date).or_insert(0.0) += irr / 1000.0; // W/m² → kWh/m²
        }
    }

    for p in points {
        if p.irradiance >= MIN_IRRADIANCE_THRESHOLD && p.solar_voltage > 0.0 {
            let e = daily_map.entry(p.date_str.clone()).or_insert((0.0, 0.0, 0));
            e.0 += p.irradiance;
            e.1 += p.solar_voltage;
            e.2 += 1;
        }
    }

    let mut daily_scores: Vec<SolarDayScore> = daily_map
        .into_iter()
        .map(|(date, (sum_irr, sum_volt, cnt))| {
            let avg_irr = sum_irr / cnt as f64;
            let avg_volt = sum_volt / cnt as f64;
            let day_ratio = avg_volt / avg_irr;
            let day_score = baseline_ratio
                .filter(|&b| b > 0.0)
                .map(|b| (day_ratio / b * 100.0).min(120.0));
            let insolation = daily_insolation.get(&date).copied().unwrap_or(0.0);
            SolarDayScore {
                date,
                insolation_kwh: insolation,
                score: day_score,
                sample_count: cnt,
            }
        })
        .collect();

    daily_scores.sort_by(|a, b| a.date.cmp(&b.date));

    SolarEfficiencyResult {
        score,
        status,
        status_label,
        message,
        baseline_ratio,
        recent_ratio,
        sample_count_baseline: baseline_ratios.len(),
        sample_count_recent: recent_ratios.len(),
        daily_scores,
    }
}

pub struct SolarEfficiencyResult {
    pub score:                 Option<f64>,
    pub status:                String,
    pub status_label:          String,
    pub message:               String,
    pub baseline_ratio:        Option<f64>,
    pub recent_ratio:          Option<f64>,
    pub sample_count_baseline: usize,
    pub sample_count_recent:   usize,
    pub daily_scores:          Vec<SolarDayScore>,
}

fn classify_score(
    score: Option<f64>,
    n_baseline: usize,
    n_recent: usize,
) -> (String, String, String) {
    const MIN_SAMPLES: usize = 12;
    if score.is_none() || n_baseline < MIN_SAMPLES || n_recent < 3 {
        return (
            "insufficient_data".into(),
            "Nedovoljno podataka".into(),
            "Potrebno najmanje 12 sat. mjerenja (baseline) i 3 u zadnjih 7 dana.".into(),
        );
    }
    let s = score.unwrap();
    if s >= 95.0 {
        ("good".into(), "Normalna efikasnost".into(), "Panel radi unutar normalnih parametara.".into())
    } else if s >= EFFICIENCY_WARN_THRESHOLD {
        ("warn".into(), "Blagi pad efikasnosti".into(),
         format!("Score {:.0}% — moguće prljanje panela ili sezonske promjene.", s))
    } else if s >= EFFICIENCY_CRIT_THRESHOLD {
        ("warn".into(), "Pad efikasnosti".into(),
         format!("Score {:.0}% — panel vjerojatno prljav ili djelomično zasjenjn. Preporučuje se čišćenje.", s))
    } else {
        ("critical".into(), "Kritičan pad efikasnosti".into(),
         format!("Score {:.0}% — panel možda oštećen ili jako prljav. Preporučuje se inspekcija.", s))
    }
}
