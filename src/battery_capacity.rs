//! Battery capacity estimation from charge/discharge totalizers.
//!
//! Algoritam:
//!  1. Uzimamo zadnjih N dana dnevnih mjerenja (measurements_24h).
//!  2. Za svaki dan: neto = battery_discharge_tot - battery_charge_tot
//!       - neto > 0 → deficit dan (baterija se praznila)
//!       - neto < 0 → surplus dan (solar je punio više nego što se trošilo)
//!  3. Pronalazimo najdulji neprekidni "deficit run" i zbrajamo neto
//!     pražnjenje. To je donja granica efektivnog kapaciteta baterije.
//!  4. Efektivni kapacitet uspoređujemo s nominalnim i računamo zdravlje (%).
//!
//! Logika:
//!  - Baterija mora biti dovoljno velika da preživi najdulji zabilježeni
//!    period bez sunca → deficit run ≈ efektivni kapacitet
//!  - Ako deficit run nije pronađen (svi dani u surplusu), koristimo
//!    maksimalno dnevno pražnjenje kao konzervativnu donju granicu.

use chrono::{DateTime, Utc};

/// Prag za "dobro zdravlje" baterije (≥ 80% nominalnog kapaciteta)
pub const HEALTH_GOOD_PCT: f64 = 80.0;
/// Prag za preporuku zamjene (< 60% nominalnog kapaciteta)
pub const HEALTH_REPLACE_PCT: f64 = 60.0;
/// Minimalan broj dana za smislenu procjenu
pub const MIN_SAMPLE_DAYS: usize = 7;

/// Jedan dan dnevnih podataka o punjenju/pražnjenju baterije
pub struct DailyTotal {
    pub recorded_at: DateTime<Utc>,
    /// Ukupno napunjeno tog dana (Ah)
    pub charge_ah: f64,
    /// Ukupno ispražnjeno tog dana (Ah)
    pub discharge_ah: f64,
}

/// Rezultat procjene kapaciteta baterije
pub struct CapacityEstimate {
    /// Procijenjeni efektivni kapacitet (Ah) — donja granica
    pub estimated_ah: Option<f64>,
    /// Zdravlje baterije u % (estimated / nominal × 100)
    pub health_percent: Option<f64>,
    /// Maksimalno jednodnevno pražnjenje (Ah) — konzervativna donja granica
    pub max_daily_discharge_ah: Option<f64>,
    /// Kumulativni deficit u najduljem deficit runu (Ah) — bolji estimate
    pub max_deficit_run_ah: Option<f64>,
    /// Broj dana uzetih u analizu
    pub sample_days: usize,
    /// Status: "good" | "degraded" | "replace" | "no_nominal" | "insufficient_data"
    pub status: &'static str,
    /// Opis statusa na hrvatskom
    pub status_label: &'static str,
}

/// Procjenjuje efektivni kapacitet baterije iz dnevnih totaliza.
///
/// `daily_totals` mora biti sortiran uzlazno po datumu.
/// `nominal_ah` je konfiguriran nominalni kapacitet baterije (opcionalno).
pub fn estimate_capacity(daily_totals: &[DailyTotal], nominal_ah: Option<f32>) -> CapacityEstimate {
    if daily_totals.len() < MIN_SAMPLE_DAYS {
        return CapacityEstimate {
            estimated_ah: None,
            health_percent: None,
            max_daily_discharge_ah: None,
            max_deficit_run_ah: None,
            sample_days: daily_totals.len(),
            status: "insufficient_data",
            status_label: "Nedovoljno podataka",
        };
    }

    // Maksimalno jednodnevno pražnjenje (konzervativna donja granica)
    let max_discharge = daily_totals
        .iter()
        .map(|d| d.discharge_ah)
        .fold(0.0_f64, f64::max);

    // Pronađi najveći kumulativni deficit run
    // (uzastopni dani gdje discharge > charge)
    let mut current_run = 0.0_f64;
    let mut max_run = 0.0_f64;

    for total in daily_totals {
        let net = total.discharge_ah - total.charge_ah;
        if net > 0.0 {
            current_run += net;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 0.0;
        }
    }

    // Odabir najboljeg estimata:
    // - Ako postoji deficit run → koristimo taj (bolji estimate)
    // - Inače → max dnevno pražnjenje (konzervativno)
    let estimated_ah = if max_run > 0.5 {
        Some(max_run)
    } else if max_discharge > 0.5 {
        Some(max_discharge)
    } else {
        None
    };

    let health_percent = match (estimated_ah, nominal_ah) {
        (Some(est), Some(nom)) if nom > 0.0 => {
            // Ne dopuštamo > 100% (može se desiti za nova mjerenja s malo podataka)
            Some((est / nom as f64 * 100.0).min(100.0))
        }
        _ => None,
    };

    let (status, status_label) = match (health_percent, nominal_ah) {
        (_, None) => ("no_nominal", "Nominalni kapacitet nije postavljen"),
        (None, _) => ("insufficient_data", "Nedovoljno podataka"),
        (Some(h), _) if h >= HEALTH_GOOD_PCT => ("good", "Baterija dobra"),
        (Some(h), _) if h >= HEALTH_REPLACE_PCT => ("degraded", "Baterija degradirana"),
        _ => ("replace", "Preporučena zamjena baterije"),
    };

    CapacityEstimate {
        estimated_ah,
        health_percent,
        max_daily_discharge_ah: if max_discharge > 0.5 { Some(max_discharge) } else { None },
        max_deficit_run_ah: if max_run > 0.5 { Some(max_run) } else { None },
        sample_days: daily_totals.len(),
        status,
        status_label,
    }
}
