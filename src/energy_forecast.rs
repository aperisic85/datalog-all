//! Energetska prognoza — predviđanje stanja baterije 7 dana unaprijed.
//!
//! Ideja: sustav već zna tri stvari o svakoj stanici —
//!   1. koliko baterija stvarno prima iz solara po jedinici insolacije
//!      (naučeni omjer Ah / kWh/m² iz povijesti totalizatora + Open-Meteo),
//!   2. koliko stanica dnevno troši (medijan dnevnog pražnjenja),
//!   3. koliki joj je kapacitet (konfigurirani nominalni ili procijenjeni).
//!
//! Open-Meteo besplatno daje prognozu iradijancije 7 dana unaprijed, pa se
//! energetska bilanca može simulirati dan po dan:
//!
//!   SOC(d+1) = SOC(d) + (omjer_punjenja × prognozirana_insolacija(d)
//!                        − dnevna_potrošnja) / kapacitet
//!
//! Iz SOC-a se preko krivulje napona olovne baterije (12 V / 24 V
//! auto-skaliranje, kao u battery_health) procjenjuje napon i uspoređuje s
//! postojećim pragovima (11.5 V upozorenje, 10.5 V kritično). Rezultat:
//! "stanica X past će ispod 11.5 V u četvrtak" — dok se još stigne
//! organizirati obilazak.
//!
//! Model je namjerno konzervativan i transparentan (bez ML magije): svi
//! parametri (omjer, potrošnja, kapacitet, početni SOC) vraćaju se u
//! odgovoru pa se procjena može provjeriti.
//!
//! Pozadinski scheduler periodički računa prognozu za sve aktivne objekte
//! s koordinatama i sprema je u `energy_forecast_cache` — odatle je čitaju
//! dashboard (kartica rizika) i jutarnji brifing, bez dodatnih poziva
//! prema Open-Meteo.

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::AppResult;
use crate::weather::WeatherHour;

/// Prag upozorenja (12 V sustav) — usklađeno s battery_prediction
pub const WARNING_VOLTAGE_12V: f64 = 11.5;
/// Kritični prag (12 V sustav)
pub const CRITICAL_VOLTAGE_12V: f64 = 10.5;
/// Koliko dana povijesti koristimo za učenje parametara
pub const HISTORY_DAYS: u32 = 30;
/// Horizont prognoze (dana unaprijed)
pub const FORECAST_DAYS: u32 = 7;
/// Minimalan broj dana s upotrebljivim omjerom punjenja
const MIN_RATIO_SAMPLES: usize = 5;
/// Minimalan broj dana s podacima o pražnjenju
const MIN_DISCHARGE_SAMPLES: usize = 5;
/// Minimalna dnevna insolacija (kWh/m²) da dan uđe u učenje omjera
const MIN_INSOLATION_KWH: f64 = 0.3;

/// Krivulja napona mirovanja olovne baterije (12 V): (napon, SOC %).
/// Uzlazno po naponu; linearna interpolacija između točaka.
const SOC_CURVE_12V: &[(f64, f64)] = &[
    (10.50, 0.0),
    (11.31, 10.0),
    (11.58, 20.0),
    (11.75, 30.0),
    (11.90, 40.0),
    (12.06, 50.0),
    (12.20, 60.0),
    (12.32, 70.0),
    (12.42, 80.0),
    (12.50, 90.0),
    (12.70, 100.0),
];

/// SOC (%) iz napona mirovanja. `factor` je 1.0 za 12 V, 2.0 za 24 V sustav.
pub fn soc_from_voltage(voltage: f64, factor: f64) -> f64 {
    let v = voltage / factor;
    let first = SOC_CURVE_12V[0];
    let last = SOC_CURVE_12V[SOC_CURVE_12V.len() - 1];
    if v <= first.0 { return first.1; }
    if v >= last.0  { return last.1; }
    for w in SOC_CURVE_12V.windows(2) {
        let (v0, s0) = w[0];
        let (v1, s1) = w[1];
        if v >= v0 && v <= v1 {
            return s0 + (s1 - s0) * (v - v0) / (v1 - v0);
        }
    }
    last.1
}

/// Napon mirovanja iz SOC-a (%) — inverz od [`soc_from_voltage`].
pub fn voltage_from_soc(soc: f64, factor: f64) -> f64 {
    let s = soc.clamp(0.0, 100.0);
    let first = SOC_CURVE_12V[0];
    let last = SOC_CURVE_12V[SOC_CURVE_12V.len() - 1];
    if s <= first.1 { return first.0 * factor; }
    if s >= last.1  { return last.0 * factor; }
    for w in SOC_CURVE_12V.windows(2) {
        let (v0, s0) = w[0];
        let (v1, s1) = w[1];
        if s >= s0 && s <= s1 {
            return (v0 + (v1 - v0) * (s - s0) / (s1 - s0)) * factor;
        }
    }
    last.0 * factor
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() { return None; }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

// ── Strukture ────────────────────────────────────────────────────────────────

/// Jedan dan povijesti totalizatora (ulaz u učenje modela)
pub struct HistoryDay {
    pub date:         NaiveDate,
    pub charge_ah:    f64,
    pub discharge_ah: f64,
}

/// Jedan dan prognoze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDay {
    /// "YYYY-MM-DD"
    pub date:             String,
    /// Prognozirana dnevna insolacija (kWh/m²)
    pub insolation_kwh:   f64,
    /// Procijenjeno punjenje (Ah)
    pub charge_est_ah:    f64,
    /// Procijenjena potrošnja (Ah)
    pub discharge_est_ah: f64,
    /// Neto bilanca (Ah)
    pub net_ah:           f64,
    /// Predviđeni SOC na kraju dana (%)
    pub soc_pct:          f64,
    /// Predviđeni napon mirovanja na kraju dana (V)
    pub voltage_est:      f64,
}

/// Rezultat energetske prognoze za jedan objekt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyForecast {
    pub object_id:               Uuid,
    pub computed_at:             DateTime<Utc>,
    /// "ok" | "warning" | "critical" | "insufficient_data"
    pub status:                  String,
    pub status_label:            String,
    pub message:                 String,
    pub days:                    Vec<ForecastDay>,
    // ── Parametri modela (transparentnost procjene) ──
    /// Kapacitet korišten u simulaciji (Ah)
    pub capacity_ah:             Option<f64>,
    /// Naučeni omjer punjenja (Ah po kWh/m² insolacije)
    pub charge_ratio_ah_per_kwh: Option<f64>,
    /// Medijan dnevne potrošnje (Ah)
    pub daily_discharge_ah:      Option<f64>,
    /// Početni SOC simulacije (%)
    pub start_soc_pct:           Option<f64>,
    /// 12.0 ili 24.0
    pub system_voltage:          f64,
    /// Broj dana povijesti upotrijebljenih za učenje omjera
    pub ratio_sample_days:       usize,
    pub first_warning_date:      Option<String>,
    pub first_critical_date:     Option<String>,
    /// Najniži predviđeni SOC unutar horizonta (%)
    pub min_soc_pct:             Option<f64>,
}

impl EnergyForecast {
    fn insufficient(object_id: Uuid, system_voltage: f64, reason: &str) -> Self {
        EnergyForecast {
            object_id,
            computed_at: Utc::now(),
            status: "insufficient_data".into(),
            status_label: "Nedovoljno podataka".into(),
            message: reason.to_string(),
            days: Vec::new(),
            capacity_ah: None,
            charge_ratio_ah_per_kwh: None,
            daily_discharge_ah: None,
            start_soc_pct: None,
            system_voltage,
            ratio_sample_days: 0,
            first_warning_date: None,
            first_critical_date: None,
            min_soc_pct: None,
        }
    }
}

// ── Izračun (čista funkcija — testabilno bez baze) ───────────────────────────

/// Izračunaj prognozu iz pripremljenih ulaza.
///
/// * `history` — dnevni totalizatori, uzlazno po datumu (zadnjih ~30 dana)
/// * `weather_hours` — satni vremenski podaci: povijest + prognoza
/// * `start_voltage` — referentni napon za početni SOC (najbolje: zadnji
///   noćni minimum, jer eliminira napon punjenja)
/// * `capacity_ah` — kapacitet baterije (nominalni ili procijenjeni)
/// * `today` — današnji datum (parametar radi determinističkih testova)
pub fn compute_forecast(
    object_id: Uuid,
    history: &[HistoryDay],
    weather_hours: &[WeatherHour],
    start_voltage: Option<f64>,
    capacity_ah: Option<f64>,
    today: NaiveDate,
) -> EnergyForecast {
    // Detekcija 12/24 V sustava iz referentnog napona
    let factor = match start_voltage {
        Some(v) if v > 18.0 => 2.0,
        _ => 1.0,
    };
    let system_voltage = 12.0 * factor;

    let start_voltage = match start_voltage {
        Some(v) if v > 1.0 => v,
        _ => return EnergyForecast::insufficient(
            object_id, system_voltage,
            "Nema izmjerenog napona baterije za početnu točku prognoze."),
    };

    let capacity = match capacity_ah {
        Some(c) if c > 1.0 => c,
        _ => return EnergyForecast::insufficient(
            object_id, system_voltage,
            "Nepoznat kapacitet baterije — unesite nominalni kapacitet (Ah) u konfiguraciji objekta."),
    };

    // Dnevna insolacija (kWh/m²) po datumu — iz svih dostupnih sati
    let mut insolation_by_date: HashMap<NaiveDate, f64> = HashMap::new();
    for h in weather_hours {
        if let Some(irr) = h.shortwave_radiation {
            *insolation_by_date.entry(h.time.date_naive()).or_insert(0.0) += irr / 1000.0;
        }
    }

    // Omjer punjenja: Ah primljeno po kWh/m² insolacije (medijan po danima)
    let mut ratios: Vec<f64> = history
        .iter()
        .filter(|d| d.date < today && d.charge_ah > 0.0)
        .filter_map(|d| {
            let ins = *insolation_by_date.get(&d.date)?;
            (ins >= MIN_INSOLATION_KWH).then(|| d.charge_ah / ins)
        })
        .collect();
    let ratio_sample_days = ratios.len();
    let charge_ratio = match median(&mut ratios) {
        Some(r) if ratio_sample_days >= MIN_RATIO_SAMPLES => r,
        _ => return EnergyForecast::insufficient(
            object_id, system_voltage,
            "Premalo dana s podacima o punjenju za učenje solarnog omjera (min. 5)."),
    };

    // Dnevna potrošnja: medijan pražnjenja zadnjih 14 dana (fallback: svi dani)
    let recent_cutoff = today - ChronoDuration::days(14);
    let mut discharges: Vec<f64> = history
        .iter()
        .filter(|d| d.date >= recent_cutoff && d.date < today)
        .map(|d| d.discharge_ah)
        .collect();
    if discharges.len() < MIN_DISCHARGE_SAMPLES {
        discharges = history.iter().filter(|d| d.date < today).map(|d| d.discharge_ah).collect();
    }
    let n_discharge = discharges.len();
    let daily_discharge = match median(&mut discharges) {
        Some(d) if n_discharge >= MIN_DISCHARGE_SAMPLES => d,
        _ => return EnergyForecast::insufficient(
            object_id, system_voltage,
            "Premalo dana s podacima o potrošnji (min. 5)."),
    };

    // Početni SOC iz referentnog napona
    let start_soc = soc_from_voltage(start_voltage, factor);
    let mut soc_ah = capacity * start_soc / 100.0;

    // Simulacija dan po dan
    let warn_v = WARNING_VOLTAGE_12V * factor;
    let crit_v = CRITICAL_VOLTAGE_12V * factor;
    let mut days: Vec<ForecastDay> = Vec::new();
    let mut first_warning: Option<String> = None;
    let mut first_critical: Option<String> = None;
    let mut min_soc = start_soc;

    for offset in 1..=FORECAST_DAYS as i64 {
        let date = today + ChronoDuration::days(offset);
        // Prognoza može biti kraća od horizonta — stani kad nema podataka
        let Some(&insolation) = insolation_by_date.get(&date) else { break; };

        let charge = charge_ratio * insolation;
        let net = charge - daily_discharge;
        soc_ah = (soc_ah + net).clamp(0.0, capacity);
        let soc_pct = soc_ah / capacity * 100.0;
        let voltage_est = voltage_from_soc(soc_pct, factor);

        if soc_pct < min_soc { min_soc = soc_pct; }
        let date_str = date.format("%Y-%m-%d").to_string();
        if voltage_est <= warn_v && first_warning.is_none() {
            first_warning = Some(date_str.clone());
        }
        if voltage_est <= crit_v && first_critical.is_none() {
            first_critical = Some(date_str.clone());
        }

        days.push(ForecastDay {
            date: date_str,
            insolation_kwh: insolation,
            charge_est_ah: charge,
            discharge_est_ah: daily_discharge,
            net_ah: net,
            soc_pct,
            voltage_est,
        });
    }

    if days.is_empty() {
        return EnergyForecast::insufficient(
            object_id, system_voltage, "Nema dostupne vremenske prognoze za lokaciju.");
    }

    let fmt_hr = |iso: &str| {
        NaiveDate::parse_from_str(iso, "%Y-%m-%d")
            .map(|d| d.format("%d.%m.").to_string())
            .unwrap_or_else(|_| iso.to_string())
    };
    let (status, status_label, message) = if let Some(ref d) = first_critical {
        ("critical", "Kritičan pad predviđen".to_string(),
         format!("Uz prognozirano vrijeme, napon pada ispod {:.1} V oko {} — preporučuje se hitna organizacija obilaska.",
                 crit_v, fmt_hr(d)))
    } else if let Some(ref d) = first_warning {
        ("warning", "Pad ispod praga upozorenja predviđen".to_string(),
         format!("Uz prognozirano vrijeme, napon pada ispod {:.1} V oko {} — razmotrite obilazak.",
                 warn_v, fmt_hr(d)))
    } else {
        ("ok", "Energetska bilanca stabilna".to_string(),
         format!("Predviđeni SOC ne pada ispod {:.0}% u sljedećih {} dana.", min_soc, days.len()))
    };

    EnergyForecast {
        object_id,
        computed_at: Utc::now(),
        status: status.into(),
        status_label,
        message,
        days,
        capacity_ah: Some(capacity),
        charge_ratio_ah_per_kwh: Some(charge_ratio),
        daily_discharge_ah: Some(daily_discharge),
        start_soc_pct: Some(start_soc),
        system_voltage,
        ratio_sample_days,
        first_warning_date: first_warning,
        first_critical_date: first_critical,
        min_soc_pct: Some(min_soc),
    }
}

// ── Orkestracija: dohvat ulaza + izračun za jedan objekt ─────────────────────

/// Izračunaj prognozu za objekt: dohvati povijest totalizatora, referentni
/// napon (zadnji noćni minimum), kapacitet i vremensku prognozu, pa simuliraj.
pub async fn forecast_for_object(pool: &PgPool, object_id: Uuid) -> AppResult<EnergyForecast> {
    let obj = crate::db::get_object_by_id(pool, object_id).await?
        .ok_or_else(|| crate::errors::AppError::NotFound(format!("Object {}", object_id)))?;

    let (lat, lon) = match (obj.latitude, obj.longitude) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => return Ok(EnergyForecast::insufficient(
            object_id, 12.0, "Objekt nema koordinate — vremenska prognoza nije moguća.")),
    };

    // Povijest totalizatora (učenje omjera i potrošnje)
    let totals = crate::db::get_daily_battery_totals(pool, object_id, HISTORY_DAYS as i64).await?;
    let history: Vec<HistoryDay> = totals
        .into_iter()
        .map(|(ts, ch, dis)| HistoryDay {
            date: ts.date_naive(),
            charge_ah: ch as f64,
            discharge_ah: dis as f64,
        })
        .collect();

    // Referentni napon: noćni minimum zadnjeg dana s mjerenjima
    let voltage_stats = crate::db::get_daily_voltage_stats(pool, object_id, 3).await?;
    let start_voltage = voltage_stats.last().map(|(_, vmin, _)| *vmin as f64);

    // Kapacitet: konfigurirani nominalni; fallback na procjenu iz totalizatora
    let capacity_ah = match obj.nominal_battery_capacity_ah {
        Some(c) if c > 0.0 => Some(c as f64),
        _ => {
            let points: Vec<crate::battery_capacity::DailyTotal> = history
                .iter()
                .map(|d| crate::battery_capacity::DailyTotal {
                    recorded_at: d.date.and_hms_opt(0, 0, 0)
                        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                        .unwrap_or_else(Utc::now),
                    charge_ah: d.charge_ah,
                    discharge_ah: d.discharge_ah,
                })
                .collect();
            crate::battery_capacity::estimate_capacity(&points, None).estimated_ah
        }
    };

    // Vrijeme: povijest (učenje) + prognoza (simulacija).
    // forecast_days uključuje današnji dan, pa FORECAST_DAYS + 1.
    let weather = crate::weather::fetch_weather_range(lat, lon, HISTORY_DAYS, FORECAST_DAYS + 1)
        .await
        .map_err(|e| crate::errors::AppError::Internal(anyhow::anyhow!(e)))?;

    Ok(compute_forecast(
        object_id,
        &history,
        &weather.hours,
        start_voltage,
        capacity_ah,
        Utc::now().date_naive(),
    ))
}

// ── Cache (energy_forecast_cache) ────────────────────────────────────────────

/// Koliko dugo se cachirana prognoza smatra svježom pri čitanju preko API-ja.
/// Open-Meteo je besplatan servis s dnevnim ograničenjem, pa se prognoza ne
/// preračunava na svako osvježavanje stranice — scheduler je ionako osvježava
/// svakih nekoliko sati.
pub const CACHE_FRESH_MINUTES: i64 = 60;

/// Pročitaj prognozu iz cachea ako je dovoljno svježa.
pub async fn read_fresh_cache(pool: &PgPool, object_id: Uuid) -> Option<EnergyForecast> {
    let row: Option<(DateTime<Utc>, serde_json::Value)> = sqlx::query_as(
        "SELECT computed_at, forecast FROM energy_forecast_cache WHERE object_id = $1")
        .bind(object_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let (computed_at, json) = row?;
    if Utc::now() - computed_at > ChronoDuration::minutes(CACHE_FRESH_MINUTES) {
        return None;
    }
    serde_json::from_value(json).ok()
}

pub async fn store_cache(pool: &PgPool, f: &EnergyForecast) -> AppResult<()> {
    let first_warning = f.first_warning_date.as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let first_critical = f.first_critical_date.as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    sqlx::query(
        "INSERT INTO energy_forecast_cache
            (object_id, computed_at, status, status_label,
             first_warning_date, first_critical_date, min_soc_pct, forecast)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (object_id) DO UPDATE SET
            computed_at = EXCLUDED.computed_at,
            status = EXCLUDED.status,
            status_label = EXCLUDED.status_label,
            first_warning_date = EXCLUDED.first_warning_date,
            first_critical_date = EXCLUDED.first_critical_date,
            min_soc_pct = EXCLUDED.min_soc_pct,
            forecast = EXCLUDED.forecast")
        .bind(f.object_id)
        .bind(f.computed_at)
        .bind(&f.status)
        .bind(&f.status_label)
        .bind(first_warning)
        .bind(first_critical)
        .bind(f.min_soc_pct.map(|v| v as f32))
        .bind(serde_json::to_value(f).unwrap_or_default())
        .execute(pool)
        .await?;
    Ok(())
}

/// Jedan red kartice rizika na dashboardu
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EnergyRiskEntry {
    pub object_id:           Uuid,
    pub object_name:         String,
    pub region_name:         String,
    pub region_color:        Option<String>,
    pub status:              String,
    pub status_label:        String,
    pub message:             String,
    pub first_warning_date:  Option<NaiveDate>,
    pub first_critical_date: Option<NaiveDate>,
    pub min_soc_pct:         Option<f32>,
    pub computed_at:         DateTime<Utc>,
}

/// Stanice pod energetskim rizikom (warning/critical) iz cachea, uz
/// poštivanje regionalnih prava pristupa korisnika.
pub async fn list_risks(pool: &PgPool, user_id: Uuid, role: &str) -> AppResult<Vec<EnergyRiskEntry>> {
    let base = "SELECT c.object_id, o.name AS object_name, r.name AS region_name,
                       r.color AS region_color, c.status, c.status_label,
                       COALESCE(c.forecast->>'message', '') AS message,
                       c.first_warning_date, c.first_critical_date,
                       c.min_soc_pct, c.computed_at
                FROM energy_forecast_cache c
                JOIN objects o ON o.id = c.object_id AND o.is_active
                JOIN regions r ON r.id = o.region_id";
    let order = " ORDER BY (c.status = 'critical') DESC, c.first_critical_date NULLS LAST,
                           c.first_warning_date NULLS LAST, o.name";
    if role == "admin" {
        Ok(sqlx::query_as::<_, EnergyRiskEntry>(
            &format!("{base} WHERE c.status IN ('warning','critical'){order}"))
            .fetch_all(pool).await?)
    } else {
        Ok(sqlx::query_as::<_, EnergyRiskEntry>(
            &format!("{base}
                JOIN user_region_access ura ON ura.region_id = o.region_id AND ura.user_id = $1
                WHERE c.status IN ('warning','critical'){order}"))
            .bind(user_id)
            .fetch_all(pool).await?)
    }
}

/// Sažetak rizika za jutarnji brifing: (ime objekta, regija, status, poruka).
pub async fn briefing_risks(pool: &PgPool) -> AppResult<Vec<(String, String, String, String)>> {
    Ok(sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT o.name, r.name, c.status, COALESCE(c.forecast->>'message', '')
         FROM energy_forecast_cache c
         JOIN objects o ON o.id = c.object_id AND o.is_active
         JOIN regions r ON r.id = o.region_id
         WHERE c.status IN ('warning','critical')
         ORDER BY (c.status = 'critical') DESC, o.name")
        .fetch_all(pool).await?)
}

// ── Pozadinski scheduler ─────────────────────────────────────────────────────

/// Pokreni periodičko računanje prognoze za sve aktivne objekte s
/// koordinatama. Interval preko env `ENERGY_FORECAST_INTERVAL_HOURS`
/// (default 6). `ENERGY_FORECAST_ENABLED=false` isključuje.
pub fn start_scheduler(pool: PgPool) {
    let enabled = std::env::var("ENERGY_FORECAST_ENABLED")
        .map(|v| v.trim() != "false" && v.trim() != "0")
        .unwrap_or(true);
    if !enabled {
        tracing::info!("Energetska prognoza: onemogućena (ENERGY_FORECAST_ENABLED)");
        return;
    }
    let interval_hours: u64 = std::env::var("ENERGY_FORECAST_INTERVAL_HOURS")
        .ok().and_then(|v| v.parse().ok())
        .map(|h: u64| h.clamp(1, 24))
        .unwrap_or(6);

    tokio::spawn(async move {
        // Kratka odgoda nakon starta da se poller/migracije smire
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        loop {
            run_all(&pool).await;
            tokio::time::sleep(std::time::Duration::from_secs(interval_hours * 3600)).await;
        }
    });
    tracing::info!(interval_hours, "Energetska prognoza: scheduler pokrenut");
}

/// Izračunaj i spremi prognozu za sve aktivne objekte s koordinatama.
pub async fn run_all(pool: &PgPool) {
    let objects: Vec<(Uuid, String)> = match sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM objects
         WHERE is_active AND latitude IS NOT NULL AND longitude IS NOT NULL
         ORDER BY name")
        .fetch_all(pool).await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "Energetska prognoza: dohvat objekata nije uspio");
            return;
        }
    };

    let total = objects.len();
    let mut at_risk = 0usize;
    for (id, name) in objects {
        match forecast_for_object(pool, id).await {
            Ok(f) => {
                if f.status == "warning" || f.status == "critical" { at_risk += 1; }
                if let Err(e) = store_cache(pool, &f).await {
                    tracing::warn!(object = %name, error = %e, "Energetska prognoza: spremanje u cache nije uspjelo");
                }
            }
            Err(e) => tracing::warn!(object = %name, error = %e, "Energetska prognoza: izračun nije uspio"),
        }
        // Blagi tempo prema Open-Meteo (besplatni API)
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    tracing::info!(total, at_risk, "Energetska prognoza: ciklus dovršen");
}

// ── Testovi ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn wh(date: NaiveDate, hour: u32, irr: f64) -> WeatherHour {
        WeatherHour {
            time: Utc.from_utc_datetime(&date.and_hms_opt(hour, 0, 0).unwrap()),
            shortwave_radiation: Some(irr),
            cloud_cover: None,
            wind_speed_10m: None,
            precipitation: None,
            temperature_2m: None,
        }
    }

    /// Povijest: 10 dana, svaki dan 5 kWh/m² insolacije, punjenje 20 Ah,
    /// pražnjenje 10 Ah → omjer 4 Ah/kWh, potrošnja 10 Ah/dan.
    fn build_inputs(
        today: NaiveDate,
        forecast_irr_per_hour: f64,
    ) -> (Vec<HistoryDay>, Vec<WeatherHour>) {
        let mut history = Vec::new();
        let mut hours = Vec::new();
        for back in 1..=10i64 {
            let d = today - ChronoDuration::days(back);
            history.push(HistoryDay { date: d, charge_ah: 20.0, discharge_ah: 10.0 });
            for h in 8..18 {
                hours.push(wh(d, h, 500.0)); // 10 h × 500 W/m² = 5 kWh/m²
            }
        }
        for fwd in 1..=7i64 {
            let d = today + ChronoDuration::days(fwd);
            for h in 8..18 {
                hours.push(wh(d, h, forecast_irr_per_hour));
            }
        }
        (history, hours)
    }

    #[test]
    fn soc_voltage_roundtrip() {
        for &(v, s) in SOC_CURVE_12V {
            assert!((soc_from_voltage(v, 1.0) - s).abs() < 0.01);
            assert!((voltage_from_soc(s, 1.0) - v).abs() < 0.01);
        }
        // 24 V skaliranje
        assert!((soc_from_voltage(25.4, 2.0) - 100.0).abs() < 0.01);
        assert!((voltage_from_soc(50.0, 2.0) - 24.12).abs() < 0.01);
    }

    #[test]
    fn sunny_forecast_stays_ok() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        // Prognoza jednaka povijesti → punjenje pokriva potrošnju (+10 Ah/dan)
        let (history, hours) = build_inputs(today, 500.0);
        let f = compute_forecast(
            Uuid::nil(), &history, &hours, Some(12.7), Some(100.0), today);
        assert_eq!(f.status, "ok", "message: {}", f.message);
        assert_eq!(f.days.len(), 7);
        assert!((f.charge_ratio_ah_per_kwh.unwrap() - 4.0).abs() < 0.01);
        assert!((f.daily_discharge_ah.unwrap() - 10.0).abs() < 0.01);
        // SOC ostaje na 100% (punjenje > potrošnja, clamp na kapacitet)
        assert!(f.days.last().unwrap().soc_pct > 99.0);
    }

    #[test]
    fn cloudy_forecast_goes_critical() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        // Prognoza: oblačno (50 W/m² → 0.5 kWh/dan → 2 Ah punjenja, -8 Ah/dan)
        let (history, hours) = build_inputs(today, 50.0);
        // Start: 50% SOC od 50 Ah baterije → 25 Ah rezerve, -8 Ah/dan
        let f = compute_forecast(
            Uuid::nil(), &history, &hours, Some(12.06), Some(50.0), today);
        assert!(f.status == "warning" || f.status == "critical",
                "status: {} ({})", f.status, f.message);
        assert!(f.first_warning_date.is_some());
        // SOC mora monotono padati
        let socs: Vec<f64> = f.days.iter().map(|d| d.soc_pct).collect();
        assert!(socs.windows(2).all(|w| w[1] <= w[0] + 0.001));
    }

    #[test]
    fn insufficient_without_capacity() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let (history, hours) = build_inputs(today, 500.0);
        let f = compute_forecast(Uuid::nil(), &history, &hours, Some(12.7), None, today);
        assert_eq!(f.status, "insufficient_data");
    }

    #[test]
    fn insufficient_without_history() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let (_, hours) = build_inputs(today, 500.0);
        let f = compute_forecast(Uuid::nil(), &[], &hours, Some(12.7), Some(100.0), today);
        assert_eq!(f.status, "insufficient_data");
    }

    #[test]
    fn detects_24v_system() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let (history, hours) = build_inputs(today, 500.0);
        let f = compute_forecast(
            Uuid::nil(), &history, &hours, Some(25.4), Some(100.0), today);
        assert_eq!(f.system_voltage, 24.0);
        assert_eq!(f.status, "ok");
    }
}
