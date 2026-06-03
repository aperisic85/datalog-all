//! Predikcija kvara baterije na temelju linearne regresije nad trendom napona.
//!
//! Algoritam:
//!  1. Uzimamo DNEVNI MINIMUM napona (noćni low) zadnjih N dana. Dnevni
//!     minimum uklanja dnevni ciklus punjenja/pražnjenja (solar danju diže
//!     napon na 14 V, noću pada na ~12 V), pa je trend stvarni pokazatelj
//!     zdravlja/SoC — za razliku od sirovog satnog napona čiji nagib ovisi
//!     o fazi dana na rubovima prozora.
//!  2. OLS linearna regresija: napon = slope * t + intercept  (t u satima)
//!  3. Procjenjujemo kada će trend pasti ispod pragova (mjereno od zadnjeg
//!     mjerenja, ne od now() — vidi t_ref):
//!       - UPOZORENJE : < 11.5 V
//!       - KRITIČNO   : < 10.5 V
//!  4. Vraćamo strukturiranu predikciju s trendom, satima/danima do praga i R².

use chrono::{DateTime, Utc};

/// Minimalni napon za upozorenje (V)
pub const VOLTAGE_WARNING: f64 = 11.5;
/// Minimalni napon za kritično stanje (V)
pub const VOLTAGE_CRITICAL: f64 = 10.5;
/// Minimalan broj uzoraka za pouzdanu predikciju
pub const MIN_SAMPLES: usize = 6;

/// Jedan vremenski punkt napona za regresiski ulaz
pub struct VoltagePoint {
    pub recorded_at: DateTime<Utc>,
    pub voltage: f64,
}

/// Rezultat linearne regresije i predikcije
pub struct BatteryTrend {
    /// Nagib trenda u V/h (negativan = pražnjenje)
    pub slope_v_per_hour: f64,
    /// Linearno procijenjen napon u trenutku zadnjeg mjerenja
    pub trend_voltage: f64,
    /// Za koliko sati se očekuje pad ispod praga upozorenja (None = nema pada)
    pub hours_to_warning: Option<f64>,
    /// Za koliko sati se očekuje pad ispod kritičnog praga (None = nema pada)
    pub hours_to_critical: Option<f64>,
    /// Stanje: "stable" | "degrading" | "warning" | "critical" | "charging"
    pub trend: &'static str,
    /// Broj uzoraka korištenih u regresiji
    pub sample_count: usize,
    /// Koeficijent determinacije (koliko dobro pravac odgovara podacima)
    pub r_squared: f64,
}

/// Izračunava linearni trend i predikciju pada napona.
///
/// `points` mora biti sortiran uzlazno po `recorded_at`.
/// Vraća `None` ako ima premalo uzoraka ili nema varijanse u vremenu.
pub fn compute_trend(points: &[VoltagePoint]) -> Option<BatteryTrend> {
    if points.len() < MIN_SAMPLES {
        return None;
    }

    // Normaliziramo t na sate od prvog mjerenja radi numeričke stabilnosti
    let t0_sec = points[0].recorded_at.timestamp() as f64;
    let xs: Vec<f64> = points
        .iter()
        .map(|p| (p.recorded_at.timestamp() as f64 - t0_sec) / 3600.0)
        .collect();
    let ys: Vec<f64> = points.iter().map(|p| p.voltage).collect();

    let n = xs.len() as f64;
    let x_mean = xs.iter().sum::<f64>() / n;
    let y_mean = ys.iter().sum::<f64>() / n;

    let cov_xy: f64 = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum();
    let var_x: f64 = xs.iter().map(|x| (x - x_mean).powi(2)).sum();

    if var_x < 1e-9 {
        return None;
    }

    let slope = cov_xy / var_x;
    let intercept = y_mean - slope * x_mean;

    // Referentno vrijeme = ZADNJI uzorak (ne Utc::now()). Time izbjegavamo
    // ekstrapolaciju u budućnost za tihe stanice (zadnji podatak star više dana)
    // koja bi lažno proglasila pad napona ispod praga.
    let t_ref = *xs.last().unwrap();
    let trend_voltage = slope * t_ref + intercept;

    // R² — mjera dobrote regresije
    let ss_res: f64 = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| (y - (slope * x + intercept)).powi(2))
        .sum();
    let ss_tot: f64 = ys.iter().map(|y| (y - y_mean).powi(2)).sum();
    let r_squared = if ss_tot < 1e-12 { 1.0 } else { 1.0 - ss_res / ss_tot }.max(0.0);

    // Predviđamo kada će trend prijeći pragove (samo ako napon pada)
    let (hours_to_warning, hours_to_critical) = if slope < 0.0 {
        let h_warn = threshold_hours(trend_voltage, VOLTAGE_WARNING, slope, intercept, t_ref);
        let h_crit = threshold_hours(trend_voltage, VOLTAGE_CRITICAL, slope, intercept, t_ref);
        (h_warn, h_crit)
    } else {
        (None, None)
    };

    // Klasifikacija stanja
    let trend = if trend_voltage <= VOLTAGE_CRITICAL {
        "critical"
    } else if trend_voltage <= VOLTAGE_WARNING {
        "warning"
    } else if slope > 0.005 {
        "charging"
    } else if slope < -0.01 {
        // Pada više od 0.01 V/h — definitivno se prazni
        "degrading"
    } else {
        "stable"
    };

    Some(BatteryTrend {
        slope_v_per_hour: slope,
        trend_voltage,
        hours_to_warning,
        hours_to_critical,
        trend,
        sample_count: points.len(),
        r_squared,
    })
}

/// Vraća za koliko sati (od zadnjeg mjerenja) trend prelazi `threshold`.
/// Vraća `None` ako je napon već ispod praga ili je prijelaz u prošlosti.
fn threshold_hours(
    current_v: f64,
    threshold: f64,
    slope: f64,
    intercept: f64,
    t_ref: f64,
) -> Option<f64> {
    if current_v <= threshold {
        return None; // Već ispod praga
    }
    // threshold = slope * t_cross + intercept  →  t_cross = (threshold - intercept) / slope
    let t_cross = (threshold - intercept) / slope;
    let hours_remaining = t_cross - t_ref;
    if hours_remaining > 0.0 {
        Some(hours_remaining)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn pt(day: i64, v: f64) -> VoltagePoint {
        VoltagePoint {
            recorded_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap() + Duration::days(day),
            voltage: v,
        }
    }

    #[test]
    fn too_few_samples_returns_none() {
        let pts: Vec<_> = (0..3).map(|i| pt(i, 12.0)).collect();
        assert!(compute_trend(&pts).is_none());
    }

    #[test]
    fn declining_series_predicts_warning() {
        // 10 dana, pad 0.1 V/dan: 12.6 → 11.7 (iznad praga upozorenja 11.5)
        let pts: Vec<_> = (0..10).map(|i| pt(i, 12.6 - 0.1 * i as f64)).collect();
        let t = compute_trend(&pts).unwrap();
        assert!(t.slope_v_per_hour < 0.0);
        assert!(t.r_squared > 0.99);
        assert!(t.hours_to_warning.is_some());
    }

    #[test]
    fn stable_series_has_no_eta() {
        let pts: Vec<_> = (0..10).map(|i| pt(i, 12.8)).collect();
        let t = compute_trend(&pts).unwrap();
        assert!(t.hours_to_warning.is_none());
        assert!(t.hours_to_critical.is_none());
    }

    /// Ključni regresijski test za popravak tihih stanica (#3): podaci su iz
    /// 2024., a danas je puno kasnije. trend_voltage mora odgovarati fitu na
    /// ZADNJEM uzorku (~12.65 V), a NE ekstrapolaciji na danas (koja bi dala
    /// besmisleno nizak/negativan napon i lažni alarm).
    #[test]
    fn stale_station_reference_is_last_sample_not_now() {
        let pts: Vec<_> = (0..8).map(|i| pt(i, 13.0 - 0.05 * i as f64)).collect();
        let last_v = 13.0 - 0.05 * 7.0; // 12.65
        let t = compute_trend(&pts).unwrap();
        assert!((t.trend_voltage - last_v).abs() < 0.05, "trend_voltage = {}", t.trend_voltage);
    }
}
