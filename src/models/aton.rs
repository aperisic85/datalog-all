//! Model očitanja AtoN stanice (izvor `aton_csd` — CSD poziv preko snopsy_r-a).
//!
//! Dekodirani [`aton_decode::Aton`] se sprema u `aton_readings` u punom
//! obliku (uključivo sirove registre), a podskup koji postojeći nadzor već
//! servira paralelno ide u `measurements_10min`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Jedno očitanje AtoN stanice — kako se čita iz baze / servira HTTP-om.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AtonReading {
    pub id:          i64,
    pub object_id:   Option<Uuid>,
    pub station_id:  String,
    pub recorded_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    // Temperature [°C]
    pub temp_trenutna_c:         Option<f32>,
    pub temp_0100_c:             Option<f32>,
    pub temp_1300_c:             Option<f32>,
    // Trenutno stanje baterija
    pub gl_svj_napon_v:          Option<f32>,
    pub gl_svj_struja_a:         Option<f32>,
    pub automat_napon_v:         Option<f32>,
    pub automat_struja_a:        Option<f32>,
    // Dnevni prosjeci (potrošnje su negativne)
    pub prosjek_napon_gl_svj_v:  Option<f32>,
    pub prosjek_napon_automat_v: Option<f32>,
    pub punjenje_gl_svj_a:       Option<f32>,
    pub punjenje_automat_a:      Option<f32>,
    pub potrosnja_gl_svj_a:      Option<f32>,
    pub potrosnja_automat_a:     Option<f32>,
    pub potrosnja_izvor_a:       Option<f32>,
    /// Dnevna potrošnja izvora svjetla [Ah] (negativna).
    pub dnevna_potrosnja_a:      Option<f32>,
    // Statusi (reg 26, 12, 29, 30)
    /// Trenutna struja izvora svjetla (LED / Maxi Halo) [A].
    pub struja_led_a:            Option<f32>,
    /// Doba dana: 0 = sumrak/svitanje, 1 = noć, 2 = dan.
    pub doba_dana:               Option<i16>,
    /// Početak noći — minuta od ponoći po satu RTU-a.
    pub pocetak_noci_min:        Option<i16>,
    /// Kraj noći — minuta od ponoći po satu RTU-a.
    pub kraj_noci_min:           Option<i16>,
    /// Podverzija `csd_verzija` programa kojom je zapis dekodiran.
    pub category:                i16,
    /// Svih 31 sirovih registara — mapa alarm/status bitova još nije
    /// razriješena, pa ih čuvamo za naknadno mapiranje.
    pub regs:                    JsonValue,
}

/// Zadnje očitanje po objektu (`v_latest_aton_readings`).
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LatestAtonReading {
    pub object_id:   Option<Uuid>,
    pub station_id:  String,
    pub recorded_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub temp_trenutna_c:         Option<f32>,
    pub temp_0100_c:             Option<f32>,
    pub temp_1300_c:             Option<f32>,
    pub gl_svj_napon_v:          Option<f32>,
    pub gl_svj_struja_a:         Option<f32>,
    pub automat_napon_v:         Option<f32>,
    pub automat_struja_a:        Option<f32>,
    pub prosjek_napon_gl_svj_v:  Option<f32>,
    pub prosjek_napon_automat_v: Option<f32>,
    pub punjenje_gl_svj_a:       Option<f32>,
    pub punjenje_automat_a:      Option<f32>,
    pub potrosnja_gl_svj_a:      Option<f32>,
    pub potrosnja_automat_a:     Option<f32>,
    pub potrosnja_izvor_a:       Option<f32>,
    /// Dnevna potrošnja izvora svjetla [Ah] (negativna).
    pub dnevna_potrosnja_a:      Option<f32>,
    // Statusi (reg 26, 12, 29, 30)
    /// Trenutna struja izvora svjetla (LED / Maxi Halo) [A].
    pub struja_led_a:            Option<f32>,
    /// Doba dana: 0 = sumrak/svitanje, 1 = noć, 2 = dan.
    pub doba_dana:               Option<i16>,
    /// Početak noći — minuta od ponoći po satu RTU-a.
    pub pocetak_noci_min:        Option<i16>,
    /// Kraj noći — minuta od ponoći po satu RTU-a.
    pub kraj_noci_min:           Option<i16>,
    /// Podverzija `csd_verzija` programa kojom je zapis dekodiran.
    pub category:                i16,
    pub regs:                    JsonValue,
}

/// Zapis za upis u `aton_readings`.
#[derive(Debug)]
pub struct AtonReadingInsert {
    pub object_id:   Option<Uuid>,
    pub station_id:  String,
    pub recorded_at: DateTime<Utc>,
    pub temp_trenutna_c:         f32,
    pub temp_0100_c:             f32,
    pub temp_1300_c:             f32,
    pub gl_svj_napon_v:          f32,
    pub gl_svj_struja_a:         f32,
    pub automat_napon_v:         f32,
    pub automat_struja_a:        f32,
    pub prosjek_napon_gl_svj_v:  f32,
    pub prosjek_napon_automat_v: f32,
    pub punjenje_gl_svj_a:       f32,
    pub punjenje_automat_a:      f32,
    pub potrosnja_gl_svj_a:      f32,
    pub potrosnja_automat_a:     f32,
    pub potrosnja_izvor_a:       f32,
    pub dnevna_potrosnja_a:      f32,
    pub struja_led_a:            f32,
    pub doba_dana:               i16,
    pub pocetak_noci_min:        i16,
    pub kraj_noci_min:           i16,
    pub category:                i16,
    pub regs:                    JsonValue,
}

impl AtonReadingInsert {
    /// Preslikaj dekodirano očitanje u zapis za bazu.
    pub fn from_aton(
        object_id: Option<Uuid>,
        station_id: &str,
        recorded_at: DateTime<Utc>,
        category: aton_decode::Category,
        a: &aton_decode::Aton,
    ) -> Self {
        // Minute od ponoći stanu u i16 (0–1439); i16 je najuži tip koji
        // Postgres SMALLINT prima bez konverzije.
        let min_i16 = |m: u16| i16::try_from(m).unwrap_or(-1);
        Self {
            object_id,
            station_id:  station_id.to_string(),
            recorded_at,
            temp_trenutna_c:         a.temp_trenutna_c,
            temp_0100_c:             a.temp_0100_c,
            temp_1300_c:             a.temp_1300_c,
            gl_svj_napon_v:          a.gl_svj.napon_v,
            gl_svj_struja_a:         a.gl_svj.struja_a,
            automat_napon_v:         a.automat.napon_v,
            automat_struja_a:        a.automat.struja_a,
            prosjek_napon_gl_svj_v:  a.prosjek_napon_gl_svj_v,
            prosjek_napon_automat_v: a.prosjek_napon_automat_v,
            punjenje_gl_svj_a:       a.punjenje_gl_svj_a,
            punjenje_automat_a:      a.punjenje_automat_a,
            potrosnja_gl_svj_a:      a.potrosnja_gl_svj_a,
            potrosnja_automat_a:     a.potrosnja_automat_a,
            potrosnja_izvor_a:       a.potrosnja_izvor_a,
            dnevna_potrosnja_a:      a.dnevna_potrosnja_a,
            struja_led_a:            a.struja_led_a,
            doba_dana:               doba_dana_kod(a.doba_dana),
            pocetak_noci_min:        min_i16(a.pocetak_noci_min),
            kraj_noci_min:           min_i16(a.kraj_noci_min),
            category:                i16::from(category.number()),
            regs: JsonValue::Array(
                a.regs.iter().map(|r| JsonValue::from(*r)).collect()
            ),
        }
    }
}

/// Brojčani kod doba dana kakav RTU šalje u registru 12.
fn doba_dana_kod(d: aton_decode::DobaDana) -> i16 {
    use aton_decode::DobaDana::*;
    match d {
        Sumrak => 0,
        Noc => 1,
        Dan => 2,
        Nepoznato(v) => i16::try_from(v).unwrap_or(-1),
    }
}

/// Alarmni zapis iz dekodiranog AtoN očitanja — ide u istu `alarms` tablicu
/// kao i alarmi CR300 stanica, pa AtoN objekti dobivaju obavijesti,
/// potvrđivanje, odlaganje i heatmap bez ijedne iznimke u pipelineu.
pub fn alarm_insert_from_aton(
    object_id: Option<Uuid>,
    station_id: &str,
    recorded_at: DateTime<Utc>,
    a: &aton_decode::Aton,
) -> crate::models::domain::AlarmInsert {
    let f = |b: bool| i16::from(b);
    crate::models::domain::AlarmInsert {
        object_id,
        station_id: station_id.to_string(),
        recorded_at,
        alarm_aton_call_request:      f(a.alarmi.poziv),
        alarm_aton_temperature:       f(a.alarmi.temperatura),
        alarm_aton_voltage_light:     f(a.alarmi.napon_gl_svj),
        alarm_aton_voltage_automat:   f(a.alarmi.napon_automat),
        alarm_aton_door_open:         f(a.alarmi.vrata),
        alarm_aton_flash_code:        f(a.alarmi.bljesak),
        alarm_aton_light_on_automat:  f(a.alarmi.svjetlo_na_automatu),
        alarm_aton_automat_on_light:  f(a.alarmi.automat_na_svjetlu),
        // 2. žarna nit nije u upotrebi na ovom tipu — RTU je uvijek šalje 0,
        // pa je spajamo na isti alarm kao i prvu ako se ikad pojavi.
        alarm_aton_lamp_blown:        f(a.alarmi.pregorena_zarulja || a.alarmi.pregorena_zarulja_2),
        alarm_aton_not_work_at_night: f(a.alarmi.ne_radi_nocu),
        alarm_aton_photocell_error:   f(a.alarmi.greska_fotocelije),
        alarm_aton_work_at_day:       f(a.alarmi.radi_danju),
        ..Default::default()
    }
}

/// Konfiguracija AtoN objekta — interní zapis za poller (čita se iz `objects`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AtonPollConfig {
    pub id:                        Uuid,
    pub station_id:                String,
    pub name:                      String,
    pub aton_snopsy_endpoint:      Option<String>,
    pub aton_number:               Option<String>,
    pub aton_addr:                 Option<i16>,
    pub aton_reg_count:            i16,
    pub aton_sync_clock:           bool,
    pub aton_connect_timeout_sec:  i16,
    pub aton_response_timeout_sec: i16,
    pub aton_category:             i16,
    pub poll_interval_sec:         i32,
}
