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
    pub dnevna_potrosnja_a:      Option<f32>,
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
    pub dnevna_potrosnja_a:      Option<f32>,
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
    pub regs:                    JsonValue,
}

impl AtonReadingInsert {
    /// Preslikaj dekodirano očitanje u zapis za bazu.
    pub fn from_aton(
        object_id: Option<Uuid>,
        station_id: &str,
        recorded_at: DateTime<Utc>,
        a: &aton_decode::Aton,
    ) -> Self {
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
            regs: JsonValue::Array(
                a.regs.iter().map(|r| JsonValue::from(*r)).collect()
            ),
        }
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
    pub poll_interval_sec:         i32,
}
