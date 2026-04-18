//! Battery capacity estimation from charge/discharge totalizers.
//!
//! Algoritam:
//!  1. Uzimamo zadnjih N dana dnevnih mjerenja (measurements_24h).
//!  2. Za svaki dan: neto = battery_discharge_tot - battery_charge_tot
//!       - neto > 0 → deficit dan (baterija se praznila)
//!       - neto < 0 → surplus dan (solar je punio više nego što se trošilo)
//!  3. Pronalazimo najdulji neprekidni "deficit run" i zbrajamo neto
//!     pražnjenje. To je **donja granica** efektivnog kapaciteta baterije —
//!     tj. baterija je dokazano mogla isporučiti barem toliko Ah.
//!
//! Važno ograničenje:
//!  Maksimalni zabilježeni deficit NIJE procjena ukupnog kapaciteta, nego
//!  isključivo donja granica. Ako solarni sustav pokriva potrošnju (baterija
//!  se rijetko ili nikad duboko ne prazni), deficit run bit će puno manji od
//!  stvarnog kapaciteta — to ne znači da je baterija loša. Zato zaključke o
//!  zdravlju donosimo samo kad je deficit run dovoljno dubok
//!  (MIN_TEST_DEPTH_PCT nominalnog). Ispod tog praga status je
//!  "insufficient_discharge" i ne preporučuje se zamjena.
//!
//!  Detekcija stvarno degradiranih baterija (status "degraded"/"replace") iz
//!  samih totalizatora nije moguća bez dodatnog signala (npr. napon ispod
//!  low-voltage cutoffa nakon plitkog pražnjenja).

use chrono::{DateTime, Utc};

/// Prag za "dobro zdravlje" baterije (≥ 80% nominalnog kapaciteta)
pub const HEALTH_GOOD_PCT: f64 = 80.0;
/// Prag za preporuku zamjene (< 60% nominalnog kapaciteta).
///
/// Trenutno se ne dostiže iz dnevnih totalizatora jer oni daju samo donju
/// granicu kapaciteta (vidi MIN_TEST_DEPTH_PCT). Zadržano za buduću
/// integraciju s naponskim signalima (low-voltage cutoff).
pub const HEALTH_REPLACE_PCT: f64 = 60.0;
/// Minimalan broj dana za smislenu procjenu
pub const MIN_SAMPLE_DAYS: usize = 7;
/// Minimalni udio nominalnog kapaciteta koji zabilježeni deficit run mora
/// doseći da bi se uopće moglo zaključivati o zdravlju baterije. Ispod ovog
/// praga baterija jednostavno nije bila dovoljno stresirana — najčešće zato
/// što solarni sustav u potpunosti pokriva potrošnju.
pub const MIN_TEST_DEPTH_PCT: f64 = 80.0;
/// Minimalni iznos pražnjenja koji smatramo "stvarnim" (da odbacimo šum).
pub const MIN_OBSERVED_DISCHARGE_AH: f64 = 0.5;

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
    /// Procijenjena **donja granica** efektivnog kapaciteta (Ah).
    /// Baterija je dokazano sposobna isporučiti barem toliko — stvarni
    /// kapacitet može biti veći.
    pub estimated_ah: Option<f64>,
    /// Zdravlje baterije u % (estimated / nominal × 100). Računa se samo kad
    /// je deficit run dovoljno dubok (≥ MIN_TEST_DEPTH_PCT nominalnog).
    pub health_percent: Option<f64>,
    /// Maksimalno jednodnevno pražnjenje (Ah)
    pub max_daily_discharge_ah: Option<f64>,
    /// Kumulativni deficit u najduljem deficit runu (Ah)
    pub max_deficit_run_ah: Option<f64>,
    /// Broj dana uzetih u analizu
    pub sample_days: usize,
    /// Status: "good" | "degraded" | "replace" | "no_nominal"
    ///       | "insufficient_data" | "insufficient_discharge"
    pub status: &'static str,
    /// Opis statusa na hrvatskom
    pub status_label: &'static str,
}

/// Procjenjuje efektivni kapacitet baterije iz dnevnih totalizacija.
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

    // Maksimalno jednodnevno pražnjenje
    let max_discharge = daily_totals
        .iter()
        .map(|d| d.discharge_ah)
        .fold(0.0_f64, f64::max);

    // Najveći kumulativni deficit run (uzastopni dani gdje discharge > charge)
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

    // Najbolja donja granica koju smo promatrali.
    let observed_lower_bound = max_run.max(max_discharge);

    let estimated_ah = if observed_lower_bound > MIN_OBSERVED_DISCHARGE_AH {
        Some(observed_lower_bound)
    } else {
        None
    };

    let max_discharge_opt = if max_discharge > MIN_OBSERVED_DISCHARGE_AH {
        Some(max_discharge)
    } else {
        None
    };
    let max_run_opt = if max_run > MIN_OBSERVED_DISCHARGE_AH {
        Some(max_run)
    } else {
        None
    };

    // Zaključak o zdravlju donosimo samo kad imamo nominalni kapacitet i kad
    // je promatrani deficit dovoljno dubok u odnosu na nominal. U suprotnom
    // (npr. solar u potpunosti pokriva potrošnju) donja granica je premala
    // da bi se ičega moglo zaključiti → "insufficient_discharge".
    let (status, status_label, health_percent) = match (nominal_ah, estimated_ah) {
        (None, _) => (
            "no_nominal",
            "Nominalni kapacitet nije postavljen",
            None,
        ),
        (Some(_), None) => (
            "insufficient_discharge",
            "Nema zabilježenog pražnjenja baterije",
            None,
        ),
        (Some(nom), Some(obs)) if nom > 0.0 => {
            let test_depth_pct = obs / nom as f64 * 100.0;
            if test_depth_pct < MIN_TEST_DEPTH_PCT {
                // Baterija nije bila dovoljno pražnjena — stvarni kapacitet se ne može
                // procijeniti. Ne donosimo zaključak o zdravlju.
                (
                    "insufficient_discharge",
                    "Baterija nije bila dovoljno pražnjena za procjenu kapaciteta",
                    None,
                )
            } else if test_depth_pct >= HEALTH_GOOD_PCT {
                // Deficit run ≥ 80% nominalnog → baterija je dokazano dobra
                // (kapacitet je barem toliki koliko smo vidjeli).
                (
                    "good",
                    "Baterija dobra",
                    Some(test_depth_pct.min(100.0)),
                )
            } else if test_depth_pct >= HEALTH_REPLACE_PCT {
                (
                    "degraded",
                    "Baterija degradirana",
                    Some(test_depth_pct),
                )
            } else {
                (
                    "replace",
                    "Preporučena zamjena baterije",
                    Some(test_depth_pct),
                )
            }
        }
        // nominal ≤ 0 — tretiraj kao nepostavljen
        (Some(_), _) => (
            "no_nominal",
            "Nominalni kapacitet nije postavljen",
            None,
        ),
    };

    CapacityEstimate {
        estimated_ah,
        health_percent,
        max_daily_discharge_ah: max_discharge_opt,
        max_deficit_run_ah: max_run_opt,
        sample_days: daily_totals.len(),
        status,
        status_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(i: i64, charge: f64, discharge: f64) -> DailyTotal {
        DailyTotal {
            recorded_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
                + chrono::Duration::days(i),
            charge_ah: charge,
            discharge_ah: discharge,
        }
    }

    /// Sunčan dobro-dimenzioniran sustav: svaki dan solar pokriva potrošnju,
    /// baterija se jedva koristi. Prije popravka algoritam je prijavljivao
    /// ~0% zdravlja i "replace" — sada mora biti "insufficient_discharge".
    #[test]
    fn sunny_system_reports_insufficient_discharge() {
        // 58 dana, svaki dan charge 10 Ah, discharge 0.6 Ah → nikad deficit
        let data: Vec<_> = (0..58).map(|i| day(i, 10.0, 0.6)).collect();
        let est = estimate_capacity(&data, Some(220.0));

        assert_eq!(est.status, "insufficient_discharge");
        assert!(est.health_percent.is_none());
        // Donja granica (max dnevno pražnjenje) se i dalje prikazuje kao info
        assert!(est.estimated_ah.unwrap() > 0.0);
        assert!(est.estimated_ah.unwrap() < 1.0);
    }

    #[test]
    fn insufficient_samples_reports_insufficient_data() {
        let data: Vec<_> = (0..5).map(|i| day(i, 5.0, 20.0)).collect();
        let est = estimate_capacity(&data, Some(100.0));
        assert_eq!(est.status, "insufficient_data");
    }

    #[test]
    fn no_nominal_reports_no_nominal() {
        let data: Vec<_> = (0..10).map(|i| day(i, 5.0, 20.0)).collect();
        let est = estimate_capacity(&data, None);
        assert_eq!(est.status, "no_nominal");
        assert!(est.health_percent.is_none());
    }

    #[test]
    fn deep_deficit_close_to_nominal_reports_good() {
        // Nominal 100 Ah. Uzastopnih 5 deficit dana po 20 Ah → run = 100 Ah → 100%
        let mut data = vec![];
        for i in 0..5 {
            data.push(day(i, 0.0, 20.0));
        }
        // par neutralnih dana
        for i in 5..10 {
            data.push(day(i, 10.0, 10.0));
        }
        let est = estimate_capacity(&data, Some(100.0));
        assert_eq!(est.status, "good");
        assert!(est.health_percent.unwrap() >= 80.0);
    }

    #[test]
    fn moderate_deficit_reports_insufficient_discharge() {
        // Run = 70 Ah, nominal = 100 → 70% test depth. To je ISPOD
        // MIN_TEST_DEPTH_PCT (80%) jer 70 Ah je donja granica — baterija je
        // mogla imati i 100 Ah, samo nismo vidjeli dublji ciklus. Ne smijemo
        // proglasiti "degraded" na temelju same donje granice.
        let mut data = vec![];
        for _ in 0..7 {
            data.push(day(data.len() as i64, 0.0, 10.0));
        }
        for _ in 0..3 {
            data.push(day(data.len() as i64, 10.0, 10.0));
        }
        let est = estimate_capacity(&data, Some(100.0));
        assert_eq!(est.status, "insufficient_discharge");
        assert!(est.health_percent.is_none());
        // Ali donja granica je zabilježena kao info
        assert_eq!(est.max_deficit_run_ah.unwrap(), 70.0);
    }

    #[test]
    fn no_discharge_reports_insufficient_discharge() {
        // Svi dani: 0 pražnjenja
        let data: Vec<_> = (0..10).map(|i| day(i, 5.0, 0.0)).collect();
        let est = estimate_capacity(&data, Some(100.0));
        assert_eq!(est.status, "insufficient_discharge");
        assert!(est.health_percent.is_none());
    }
}
