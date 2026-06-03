//! Detekcija degradirane baterije iz ponašanja napona.
//!
//! Procjena kapaciteta iz totalizatora (battery_capacity.rs) daje samo donju
//! granicu i u praksi rijetko može proglasiti bateriju lošom. Ovaj modul gleda
//! **kako se napon ponaša** i hvata tipičan obrazac istrošene olovne baterije:
//! danju se uredno napuni (regulator digne napon), ali noću pod malim teretom
//! (nav-svjetlo) napon brzo padne — baterija ne drži napunjeno.
//!
//! Algoritam:
//!  1. Po danu računamo v_min (noćni low) i v_max (vrhunac punjenja).
//!  2. "Kvalificirani" (napunjeni) dani su oni gdje je v_max ≥ prag punjenja —
//!     znamo da je baterija tog dana dobila pun naboj. Time isključujemo
//!     oblačne dane i lažne pozitive.
//!  3. Nad napunjenim danima gledamo medijan noćnog minimuma i medijan dnevnog
//!     raspona (v_max − v_min):
//!       - nizak noćni minimum unatoč punom punjenju  → baterija ne drži
//!       - velik dnevni raspon                         → mali efektivni kapacitet
//!  4. Pragovi se automatski skaliraju za 12 V ili 24 V sustav.
//!
//! Pragovi su konzervativni i pretpostavljaju olovnu bateriju s laganim
//! (nav-svjetlo) teretom; po potrebi ih se može fino podesiti.

/// Minimalan broj dana ukupno za smislenu procjenu
pub const MIN_SAMPLE_DAYS: usize = 10;
/// Minimalan broj napunjenih (sunčanih) dana potrebnih za zaključak
pub const MIN_CHARGED_DAYS: usize = 5;

// Pragovi za 12 V sustav (skaliraju se ×2 za 24 V)
/// Dnevni vrhunac napona iznad kojeg smatramo da je baterija tog dana napunjena
pub const CHARGED_DAY_MAX_12V: f64 = 13.2;
/// Noćni minimum ispod kojeg (unatoč punjenju) sumnjamo na slabljenje
pub const SAG_WARN_12V: f64 = 12.0;
/// Noćni minimum ispod kojeg (unatoč punjenju) je baterija jasno loša
pub const SAG_CRITICAL_12V: f64 = 11.5;
/// Dnevni raspon napona iznad kojeg sumnjamo na mali efektivni kapacitet
pub const SWING_HIGH_12V: f64 = 1.8;

/// Dnevni napon: minimum i maksimum
pub struct DailyVoltage {
    pub v_min: f64,
    pub v_max: f64,
}

pub struct HealthAssessment {
    /// "good" | "degraded" | "replace" | "insufficient_data"
    pub status: &'static str,
    pub status_label: &'static str,
    /// Ukupno dana u analizi
    pub sample_days: usize,
    /// Broj napunjenih (kvalificiranih) dana
    pub charged_days: usize,
    /// Medijan noćnog minimuma na napunjenim danima (V)
    pub median_charged_night_min: Option<f64>,
    /// Medijan dnevnog raspona napona na napunjenim danima (V)
    pub median_daily_swing: Option<f64>,
    /// Najgori (najniži) noćni minimum na napunjenim danima (V)
    pub worst_charged_night_min: Option<f64>,
    /// Detektirani napon sustava (12 ili 24)
    pub system_voltage: f64,
}

fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn insufficient(sample_days: usize, charged_days: usize, system_voltage: f64, label: &'static str)
    -> HealthAssessment
{
    HealthAssessment {
        status: "insufficient_data",
        status_label: label,
        sample_days,
        charged_days,
        median_charged_night_min: None,
        median_daily_swing: None,
        worst_charged_night_min: None,
        system_voltage,
    }
}

/// Procjenjuje zdravlje baterije iz dnevnih min/max napona.
pub fn assess_health(days: &[DailyVoltage]) -> HealthAssessment {
    if days.len() < MIN_SAMPLE_DAYS {
        return insufficient(days.len(), 0, 12.0, "Nedovoljno podataka");
    }

    // Detekcija napona sustava (12 V ili 24 V) iz tipičnog vrhunca punjenja
    let all_vmax: Vec<f64> = days.iter().map(|d| d.v_max).collect();
    let factor = if median(&all_vmax) > 18.0 { 2.0 } else { 1.0 };
    let system_voltage = 12.0 * factor;

    let charged_thr = CHARGED_DAY_MAX_12V * factor;
    let sag_warn    = SAG_WARN_12V * factor;
    let sag_crit    = SAG_CRITICAL_12V * factor;
    let swing_high  = SWING_HIGH_12V * factor;

    // Napunjeni dani: baterija je tog dana dokazano dobila pun naboj
    let charged: Vec<&DailyVoltage> = days.iter().filter(|d| d.v_max >= charged_thr).collect();
    if charged.len() < MIN_CHARGED_DAYS {
        return insufficient(
            days.len(), charged.len(), system_voltage,
            "Nedovoljno sunčanih dana s punim punjenjem za procjenu",
        );
    }

    let mins:   Vec<f64> = charged.iter().map(|d| d.v_min).collect();
    let swings: Vec<f64> = charged.iter().map(|d| d.v_max - d.v_min).collect();
    let med_min   = median(&mins);
    let med_swing = median(&swings);
    let worst_min = mins.iter().cloned().fold(f64::INFINITY, f64::min);

    let (status, status_label) = if med_min < sag_crit {
        ("replace", "Baterija ne drži napon ni nakon punjenja — preporučena zamjena")
    } else if med_min < sag_warn || med_swing > swing_high {
        ("degraded", "Baterija pokazuje znakove slabljenja")
    } else {
        ("good", "Baterija drži napon")
    };

    HealthAssessment {
        status,
        status_label,
        sample_days: days.len(),
        charged_days: charged.len(),
        median_charged_night_min: Some(med_min),
        median_daily_swing: Some(med_swing),
        worst_charged_night_min: Some(worst_min),
        system_voltage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(n: usize, vmin: f64, vmax: f64) -> Vec<DailyVoltage> {
        (0..n).map(|_| DailyVoltage { v_min: vmin, v_max: vmax }).collect()
    }

    #[test]
    fn healthy_battery_holds_voltage() {
        // Napuni se na 13.6, noću padne tek na 12.6 → zdravo
        let d = days(20, 12.6, 13.6);
        let a = assess_health(&d);
        assert_eq!(a.status, "good");
        assert_eq!(a.charged_days, 20);
        assert_eq!(a.system_voltage, 12.0);
    }

    #[test]
    fn deep_night_sag_reports_replace() {
        // Napuni se na 13.5, ali noću padne na 11.2 unatoč punjenju → zamjena
        let d = days(20, 11.2, 13.5);
        let a = assess_health(&d);
        assert_eq!(a.status, "replace");
    }

    #[test]
    fn moderate_sag_reports_degraded() {
        // Noćni min 11.8 (< 12.0 warn, ≥ 11.5 crit) → degradirana
        let d = days(20, 11.8, 13.5);
        let a = assess_health(&d);
        assert_eq!(a.status, "degraded");
    }

    #[test]
    fn large_swing_reports_degraded() {
        // Noćni min ok (12.1) ali raspon velik (14.0-12.1=1.9 > 1.8) → degradirana
        let d = days(20, 12.1, 14.0);
        let a = assess_health(&d);
        assert_eq!(a.status, "degraded");
    }

    #[test]
    fn cloudy_period_insufficient_charged_days() {
        // Baterija se nikad ne napuni (v_max 12.8 < 13.2) → ne zaključujemo
        let d = days(20, 12.0, 12.8);
        let a = assess_health(&d);
        assert_eq!(a.status, "insufficient_data");
        assert_eq!(a.charged_days, 0);
    }

    #[test]
    fn too_few_days_insufficient() {
        let d = days(5, 12.6, 13.6);
        let a = assess_health(&d);
        assert_eq!(a.status, "insufficient_data");
    }

    #[test]
    fn detects_24v_system() {
        // 24 V sustav: vrhunac 27.2, noć 25.2 → zdravo, pragovi skalirani
        let d = days(20, 25.2, 27.2);
        let a = assess_health(&d);
        assert_eq!(a.system_voltage, 24.0);
        assert_eq!(a.status, "good");
    }
}
