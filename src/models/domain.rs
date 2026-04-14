use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

// ================================================================
// REGION
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Region {
    pub id:          Uuid,
    pub name:        String,
    pub code:        String,
    pub description: Option<String>,
    pub color:       String,
    pub is_active:   bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRegionRequest {
    pub name:        String,
    pub code:        String,
    pub description: Option<String>,
    pub color:       Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateRegionRequest {
    pub name:        Option<String>,
    pub description: Option<String>,
    pub color:       Option<String>,
    pub is_active:   Option<bool>,
}

// ================================================================
// STATION TYPE
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StationType {
    pub id:   i16,
    pub code: String,
    pub name: String,
    pub icon: Option<String>,
}

// ================================================================
// OBJECT (v_objects view)
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ObjectView {
    pub id:                  Uuid,
    pub station_id:          String,
    pub name:                String,
    pub short_name:          Option<String>,
    pub latitude:            Option<f64>,
    pub longitude:           Option<f64>,
    pub location_name:       Option<String>,
    pub allowed_radius_m:    i32,
    pub description:         Option<String>,
    pub notes:               Option<String>,
    pub is_active:           bool,
    pub polling_enabled:     bool,
    pub datalogger_url:      Option<String>,
    pub poll_interval_sec:   i32,
    pub commissioned_at:     Option<NaiveDate>,
    // Program tip (modularni CR300)
    pub program_version:     Option<String>,
    pub program_features:    Option<JsonValue>,
    // Alarm cache
    pub alarm_active:        bool,
    pub alarm_count:         i16,
    pub alarm_worst_level:   Option<i16>,
    pub alarm_last_seen_at:  Option<DateTime<Utc>>,
    pub alarm_summary:       Option<String>,
    // Tip objekta
    pub type_code:           Option<String>,
    pub type_name:           Option<String>,
    pub type_icon:           Option<String>,
    // Regija
    pub region_id:           Uuid,
    pub region_name:         String,
    pub region_code:         String,
    pub region_color:        String,
    // Slike
    pub primary_image_url:   Option<String>,
    pub image_count:         Option<i64>,
    // Battery capacity estimator
    pub nominal_battery_capacity_ah: Option<f32>,
    // Silent station detection
    pub silence_timeout_minutes: i32,
    pub last_measurement_at:     Option<DateTime<Utc>>,
    pub is_silent:               bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateObjectRequest {
    pub station_id:        String,
    pub name:              String,
    pub short_name:        Option<String>,
    pub region_id:         Uuid,
    pub station_type_id:   Option<i16>,
    pub latitude:          Option<f64>,
    pub longitude:         Option<f64>,
    pub location_name:     Option<String>,
    pub allowed_radius_m:  Option<i32>,
    pub description:       Option<String>,
    pub notes:             Option<String>,
    pub datalogger_url:    Option<String>,
    pub datalogger_user:   Option<String>,
    pub datalogger_pass:   Option<String>,
    pub poll_interval_sec: Option<i32>,
    pub polling_enabled:   Option<bool>,
    pub commissioned_at:   Option<NaiveDate>,
    pub program_version:   Option<String>,
    pub program_features:  Option<JsonValue>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateObjectRequest {
    pub name:              Option<String>,
    pub short_name:        Option<String>,
    pub region_id:         Option<Uuid>,
    pub station_type_id:   Option<i16>,
    pub latitude:          Option<f64>,
    pub longitude:         Option<f64>,
    pub location_name:     Option<String>,
    pub allowed_radius_m:  Option<i32>,
    pub description:       Option<String>,
    pub notes:             Option<String>,
    pub datalogger_url:    Option<String>,
    pub datalogger_user:   Option<String>,
    pub datalogger_pass:   Option<String>,
    pub poll_interval_sec: Option<i32>,
    pub polling_enabled:   Option<bool>,
    pub is_active:         Option<bool>,
    pub program_version:   Option<String>,
    pub program_features:  Option<JsonValue>,
    pub nominal_battery_capacity_ah: Option<f32>,
    pub silence_timeout_minutes:     Option<i32>,
}

// ================================================================
// OBJECT IMAGE
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ObjectImage {
    pub id:              Uuid,
    pub object_id:       Uuid,
    pub filename:        String,
    pub original_name:   Option<String>,
    pub mime_type:       String,
    pub file_size_bytes: Option<i32>,
    pub storage_path:    String,
    pub storage_url:     Option<String>,
    pub is_primary:      bool,
    pub caption:         Option<String>,
    pub taken_at:        Option<NaiveDate>,
    pub uploaded_by:     Option<Uuid>,
    pub uploaded_at:     DateTime<Utc>,
}

// ================================================================
// MEASUREMENTS 10min
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Measurement10min {
    pub id:                      i64,
    pub object_id:               Option<Uuid>,
    pub station_id:              String,
    pub recorded_at:             DateTime<Utc>,
    pub received_at:             DateTime<Utc>,
    pub datalogger_temp_avg:     Option<f32>,
    pub battery_voltage_avg:     Option<f32>,
    pub battery_current_avg:     Option<f32>,
    pub battery_status_smp:      Option<i16>,
    pub battery_status_avg:      Option<f32>,
    pub solar_voltage_avg:       Option<f32>,
    pub solar_daylight_smp:      Option<i16>,
    pub solar_daylight_avg:      Option<f32>,
    pub modem_power_avg:         Option<f32>,
    pub internet_ok_avg:         Option<f32>,
    pub garmin_comm_ok_avg:      Option<f32>,
    pub garmin_satellites_avg:   Option<f32>,
    pub garmin_latitude_avg:     Option<f64>,
    pub garmin_longitude_avg:    Option<f64>,
    pub garmin_distance_avg:     Option<f32>,
    pub lantern_comm_ok_avg:        Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_active_avg: Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    pub lantern_latitude_avg:       Option<f64>,
    pub lantern_longitude_avg:      Option<f64>,
    pub lantern_distance_avg:       Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_comm_ok_avg:     Option<f32>,
    pub visibility_value_avg:       Option<f32>,
    pub visibility_alarm_avg:       Option<f32>,
    pub visibility_error_smp:       Option<i16>,
    pub fog_signal_active_avg:      Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

#[derive(Debug, Default)]
pub struct Measurement10minInsert {
    pub object_id:                  Option<Uuid>,
    pub station_id:                 String,
    pub recorded_at:                DateTime<Utc>,
    pub datalogger_temp_avg:        Option<f32>,
    pub battery_voltage_avg:        Option<f32>,
    pub battery_current_avg:        Option<f32>,
    pub battery_status_smp:         Option<i16>,
    pub battery_status_avg:         Option<f32>,
    pub solar_voltage_avg:          Option<f32>,
    pub solar_daylight_smp:         Option<i16>,
    pub solar_daylight_avg:         Option<f32>,
    pub modem_power_avg:            Option<f32>,
    pub internet_ok_avg:            Option<f32>,
    pub garmin_comm_ok_avg:         Option<f32>,
    pub garmin_satellites_avg:      Option<f32>,
    pub garmin_latitude_avg:        Option<f64>,
    pub garmin_longitude_avg:       Option<f64>,
    pub garmin_distance_avg:        Option<f32>,
    pub lantern_comm_ok_avg:        Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_active_avg: Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    pub lantern_latitude_avg:       Option<f64>,
    pub lantern_longitude_avg:      Option<f64>,
    pub lantern_distance_avg:       Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_comm_ok_avg:     Option<f32>,
    pub visibility_value_avg:       Option<f32>,
    pub visibility_alarm_avg:       Option<f32>,
    pub visibility_error_smp:       Option<i16>,
    pub fog_signal_active_avg:      Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

// ================================================================
// MEASUREMENTS 1h
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Measurement1h {
    pub id:                      i64,
    pub object_id:               Option<Uuid>,
    pub station_id:              String,
    pub recorded_at:             DateTime<Utc>,
    pub received_at:             DateTime<Utc>,
    pub datalogger_temp_avg:     Option<f32>,
    pub battery_voltage_avg:     Option<f32>,
    pub battery_current_avg:     Option<f32>,
    pub battery_charge_tot:      Option<f32>,
    pub battery_discharge_tot:   Option<f32>,
    pub battery_status_avg:      Option<f32>,
    pub solar_voltage_avg:       Option<f32>,
    pub solar_daylight_avg:      Option<f32>,
    pub modem_power_avg:            Option<f32>,
    pub internet_ok_avg:            Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_value_avg:       Option<f32>,
    pub visibility_alarm_avg:       Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

#[derive(Debug, Default)]
pub struct Measurement1hInsert {
    pub object_id:                  Option<Uuid>,
    pub station_id:                 String,
    pub recorded_at:                DateTime<Utc>,
    pub datalogger_temp_avg:        Option<f32>,
    pub battery_voltage_avg:        Option<f32>,
    pub battery_current_avg:        Option<f32>,
    pub battery_charge_tot:         Option<f32>,
    pub battery_discharge_tot:      Option<f32>,
    pub battery_status_avg:         Option<f32>,
    pub solar_voltage_avg:          Option<f32>,
    pub solar_daylight_avg:         Option<f32>,
    pub modem_power_avg:            Option<f32>,
    pub internet_ok_avg:            Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_value_avg:       Option<f32>,
    pub visibility_alarm_avg:       Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

// ================================================================
// MEASUREMENTS 24h
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Measurement24h {
    pub id:                      i64,
    pub object_id:               Option<Uuid>,
    pub station_id:              String,
    pub recorded_at:             DateTime<Utc>,
    pub received_at:             DateTime<Utc>,
    pub datalogger_temp_avg:     Option<f32>,
    pub battery_voltage_avg:     Option<f32>,
    pub battery_current_avg:     Option<f32>,
    pub battery_current_min:     Option<f32>,
    pub battery_current_max:     Option<f32>,
    pub battery_charge_tot:      Option<f32>,
    pub battery_discharge_tot:   Option<f32>,
    pub battery_status_avg:      Option<f32>,
    pub solar_daylight_avg:      Option<f32>,
    pub modem_power_avg:            Option<f32>,
    pub internet_ok_avg:            Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_value_avg:       Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

#[derive(Debug, Default)]
pub struct Measurement24hInsert {
    pub object_id:                  Option<Uuid>,
    pub station_id:                 String,
    pub recorded_at:                DateTime<Utc>,
    pub datalogger_temp_avg:        Option<f32>,
    pub battery_voltage_avg:        Option<f32>,
    pub battery_current_avg:        Option<f32>,
    pub battery_current_min:        Option<f32>,
    pub battery_current_max:        Option<f32>,
    pub battery_charge_tot:         Option<f32>,
    pub battery_discharge_tot:      Option<f32>,
    pub battery_status_avg:         Option<f32>,
    pub solar_daylight_avg:         Option<f32>,
    pub modem_power_avg:            Option<f32>,
    pub internet_ok_avg:            Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_value_avg:       Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

// ================================================================
// ALARMS
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlarmRecord {
    pub id:                           i64,
    pub object_id:                    Option<Uuid>,
    pub station_id:                   String,
    pub recorded_at:                  DateTime<Utc>,
    pub received_at:                  DateTime<Utc>,
    pub alarm_datalogger_high_temp:   i16,
    pub alarm_datalogger_high_voltage: i16,
    pub alarm_datalogger_other_error: i16,
    pub alarm_battery_voltage_low:    i16,
    pub alarm_battery_voltage_flat:   i16,
    pub alarm_battery_other_error:    i16,
    pub alarm_garmin_comm_failed:     i16,
    pub alarm_garmin_other_error:     i16,
    pub alarm_station_out_of_radius:  i16,
    pub alarm_lantern_night_light_off: i16,
    pub alarm_lantern_day_light_on:   i16,
    pub alarm_lantern_comm_failed:    i16,
    pub alarm_lantern_other_error:    i16,
    pub alarm_modem_network_error:    i16,
    pub alarm_modem_other_error:               i16,
    pub alarm_station_other_error:             i16,
    // Novi alarmi (modularni program)
    pub alarm_visibility_comm_failed:          i16,
    pub alarm_visibility_error:                i16,
    pub alarm_fog_signal_off_during_fog:       i16,
    pub alarm_fog_signal_on_while_no_fog:      i16,
    pub any_alarm_active:                      bool,
}

/// Prošireni alarm zapis s info o objektu i regiji — za globalni pregled
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlarmListItem {
    pub id:                           i64,
    pub object_id:                    Uuid,
    pub object_name:                  String,
    pub station_id:                   String,
    pub region_id:                    Uuid,
    pub region_name:                  String,
    pub region_code:                  String,
    pub region_color:                 String,
    pub location_name:                Option<String>,
    pub recorded_at:                  DateTime<Utc>,
    pub acknowledged_at:              Option<DateTime<Utc>>,
    pub acknowledged_by:              Option<String>,
    pub any_alarm_active:             bool,
    pub alarm_datalogger_high_temp:   i16,
    pub alarm_datalogger_high_voltage: i16,
    pub alarm_datalogger_other_error: i16,
    pub alarm_battery_voltage_low:    i16,
    pub alarm_battery_voltage_flat:   i16,
    pub alarm_battery_other_error:    i16,
    pub alarm_garmin_comm_failed:     i16,
    pub alarm_garmin_other_error:     i16,
    pub alarm_station_out_of_radius:  i16,
    pub alarm_lantern_night_light_off: i16,
    pub alarm_lantern_day_light_on:   i16,
    pub alarm_lantern_comm_failed:    i16,
    pub alarm_lantern_other_error:    i16,
    pub alarm_modem_network_error:             i16,
    pub alarm_modem_other_error:               i16,
    pub alarm_station_other_error:             i16,
    // Novi alarmi (modularni program)
    pub alarm_visibility_comm_failed:          i16,
    pub alarm_visibility_error:                i16,
    pub alarm_fog_signal_off_during_fog:       i16,
    pub alarm_fog_signal_on_while_no_fog:      i16,
}

#[derive(Debug, Default, Deserialize)]
pub struct AlarmListQuery {
    pub region_id:  Option<Uuid>,
    /// "active" | "acknowledged" | "all"  (default: "active")
    pub status:     Option<String>,
    pub page:       Option<i64>,
    pub page_size:  Option<i64>,
}

#[derive(Debug, Default)]
pub struct AlarmInsert {
    pub object_id:                    Option<Uuid>,
    pub station_id:                   String,
    pub recorded_at:                  DateTime<Utc>,
    pub alarm_datalogger_high_temp:   i16,
    pub alarm_datalogger_high_voltage: i16,
    pub alarm_datalogger_other_error: i16,
    pub alarm_battery_voltage_low:    i16,
    pub alarm_battery_voltage_flat:   i16,
    pub alarm_battery_other_error:    i16,
    pub alarm_garmin_comm_failed:     i16,
    pub alarm_garmin_other_error:     i16,
    pub alarm_station_out_of_radius:  i16,
    pub alarm_lantern_night_light_off: i16,
    pub alarm_lantern_day_light_on:   i16,
    pub alarm_lantern_comm_failed:    i16,
    pub alarm_lantern_other_error:    i16,
    pub alarm_modem_network_error:             i16,
    pub alarm_modem_other_error:               i16,
    pub alarm_station_other_error:             i16,
    // Novi alarmi (modularni program)
    pub alarm_visibility_comm_failed:          i16,
    pub alarm_visibility_error:                i16,
    pub alarm_fog_signal_off_during_fog:       i16,
    pub alarm_fog_signal_on_while_no_fog:      i16,
}

// ================================================================
// EVENT LOG
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventLogRecord {
    pub id:          i64,
    pub object_id:   Option<Uuid>,
    pub station_id:  String,
    pub recorded_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub log_level:   i16,
    pub log_message: String,
}

#[derive(Debug)]
pub struct EventLogInsert {
    pub object_id:   Option<Uuid>,
    pub station_id:  String,
    pub recorded_at: DateTime<Utc>,
    pub log_level:   i16,
    pub log_message: String,
}

// ================================================================
// USER
// ================================================================
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id:            Uuid,
    pub username:      String,
    pub email:         String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name:     Option<String>,
    pub role:          String,
    pub is_active:     bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
    pub created_by:    Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserPublic {
    pub id:            Uuid,
    pub username:      String,
    pub email:         String,
    pub full_name:     Option<String>,
    pub role:          String,
    pub is_active:     bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at:    DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username:  String,
    pub email:     String,
    pub password:  String,
    pub full_name: Option<String>,
    pub role:      String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email:     Option<String>,
    pub full_name: Option<String>,
    pub role:      Option<String>,
    pub is_active: Option<bool>,
    pub password:  Option<String>,
}

// ================================================================
// USER REGION ACCESS
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRegionAccess {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub region_id:  Uuid,
    pub permission: String,
    pub granted_at: DateTime<Utc>,
    pub granted_by: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserRegionAccessView {
    pub id:           Uuid,
    pub user_id:      Uuid,
    pub region_id:    Uuid,
    pub region_name:  String,
    pub region_code:  String,
    pub region_color: String,
    pub permission:   String,
    pub granted_at:   DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GrantRegionAccessRequest {
    pub user_id:    Uuid,
    pub region_id:  Uuid,
    pub permission: String,
}

// ================================================================
// AUTH
// ================================================================
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token:  String,
    pub refresh_token: String,
    pub token_type:    &'static str,
    pub expires_in:    u64,
    pub user:          UserPublic,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub:      String,
    pub username: String,
    pub role:     String,
    pub exp:      u64,
    pub iat:      u64,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type:   &'static str,
    pub expires_in:   u64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

// ================================================================
// REGION SUMMARY (dashboard)
// ================================================================
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RegionSummary {
    pub region_id:           Option<Uuid>,
    pub region_name:         Option<String>,
    pub region_code:         Option<String>,
    pub region_color:        Option<String>,
    pub total_objects:       Option<i64>,
    pub active_objects:      Option<i64>,
    pub objects_in_alarm:    Option<i64>,
    pub worst_alarm_level:   Option<i16>,
    pub avg_battery_voltage: Option<f64>,
    pub battery_flat_count:  Option<i64>,
    pub battery_low_count:   Option<i64>,
    pub lanterns_on_count:   Option<i64>,
}

// ================================================================
// LATEST MEASUREMENT (v_latest_measurements view)
// ================================================================
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LatestMeasurement {
    pub object_id:               Option<Uuid>,
    pub station_id:              Option<String>,
    pub recorded_at:             Option<DateTime<Utc>>,
    pub datalogger_temp_avg:     Option<f32>,
    pub battery_voltage_avg:     Option<f32>,
    pub battery_current_avg:     Option<f32>,
    pub battery_status_smp:      Option<i16>,
    pub solar_voltage_avg:       Option<f32>,
    pub solar_daylight_smp:      Option<i16>,
    pub modem_power_avg:         Option<f32>,
    pub internet_ok_avg:         Option<f32>,
    pub garmin_comm_ok_avg:      Option<f32>,
    pub garmin_satellites_avg:   Option<f32>,
    pub garmin_latitude_avg:     Option<f64>,
    pub garmin_longitude_avg:    Option<f64>,
    pub garmin_distance_avg:     Option<f32>,
    pub lantern_comm_ok_avg:        Option<f32>,
    pub lantern_light_active_avg:   Option<f32>,
    pub lantern_current_active_avg: Option<f32>,
    pub lantern_current_avg:        Option<f32>,
    pub lantern_distance_avg:       Option<f32>,
    // Novi senzori (modularni program)
    pub visibility_comm_ok_avg:     Option<f32>,
    pub visibility_value_avg:       Option<f32>,
    pub visibility_alarm_avg:       Option<f32>,
    pub fog_signal_active_avg:      Option<f32>,
    pub fog_signal_current_avg:     Option<f32>,
}

// ================================================================
// OBJECT POLL CONFIG (interný — za poller, bez izlaganja lozinke u API-u)
// ================================================================
#[derive(Debug, sqlx::FromRow)]
pub struct ObjectPollConfig {
    pub id:                Uuid,
    pub station_id:        String,
    pub datalogger_url:    Option<String>,
    pub datalogger_user:   Option<String>,
    pub datalogger_pass:   Option<String>,
    pub poll_interval_sec: i32,
    pub polling_enabled:   bool,
}

// ================================================================
// WEATHER & SOLAR EFFICIENCY QUERY PARAMS
// ================================================================

/// Query params za GET /objects/:id/weather
#[derive(Debug, Deserialize, Default)]
pub struct WeatherQuery {
    /// Broj dana unatrag (1–30, default 7)
    pub days: Option<u32>,
}

// ================================================================
// QUERY PARAMS
// ================================================================
#[derive(Debug, Deserialize, Default)]
pub struct ObjectsQuery {
    pub page:      Option<i64>,
    pub page_size: Option<i64>,
    pub search:    Option<String>,
    pub region_id: Option<Uuid>,
    pub active:    Option<bool>,
    pub in_alarm:  Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    pub from:      Option<DateTime<Utc>>,
    pub to:        Option<DateTime<Utc>>,
    pub limit:     Option<i64>,
}

// ================================================================
// BATTERY PREDICTION
// ================================================================

/// Rezultat predikcije kvara baterije za jedan objekt.
/// Vraćen od GET /api/v1/objects/:id/battery/prediction
#[derive(Debug, Serialize)]
pub struct BatteryPrediction {
    pub object_id:         Uuid,
    pub computed_at:       DateTime<Utc>,
    /// Zadnji izmjereni napon iz measurements_1h (V)
    pub current_voltage:   Option<f32>,
    /// Linearno ekstrapolirani napon u trenutku izračuna (V)
    pub trend_voltage:     Option<f64>,
    /// Nagib trenda (V/h); negativan = baterija se prazni
    pub slope_v_per_hour:  f64,
    /// Klasifikacija: "stable" | "charging" | "degrading" | "warning" | "critical" | "insufficient_data"
    pub trend:             String,
    /// Za koliko sati se očekuje pad ispod 11.5 V (upozorenje)
    pub hours_to_warning:  Option<f64>,
    /// Za koliko sati se očekuje pad ispod 10.5 V (kritično)
    pub hours_to_critical: Option<f64>,
    /// Isto u danima (radi lakšeg prikaza)
    pub days_to_warning:   Option<f64>,
    pub days_to_critical:  Option<f64>,
    /// Broj satnih uzoraka korištenih u regresiji
    pub sample_count:      i32,
    /// Koeficijent determinacije R² (0–1)
    pub r_squared:         Option<f64>,
}

// ================================================================
// BATTERY CAPACITY ESTIMATE
// ================================================================

/// Procjena efektivnog kapaciteta baterije iz dnevnih totalizatora.
/// Vraćen od GET /api/v1/objects/:id/battery/capacity
#[derive(Debug, Serialize)]
pub struct BatteryCapacityEstimate {
    pub object_id:               Uuid,
    pub computed_at:             DateTime<Utc>,
    /// Nominalni kapacitet (Ah) — iz konfiguracije objekta
    pub nominal_capacity_ah:     Option<f32>,
    /// Procijenjeni efektivni kapacitet (Ah) — iz analize totalizatora
    pub estimated_capacity_ah:   Option<f64>,
    /// Zdravlje baterije (estimated / nominal × 100), max 100
    pub health_percent:          Option<f64>,
    /// Maksimalno jednodnevno pražnjenje (Ah) — konzervativna donja granica
    pub max_daily_discharge_ah:  Option<f64>,
    /// Kumulativni deficit u najduljem deficit runu (Ah) — bolji estimat
    pub max_deficit_run_ah:      Option<f64>,
    /// Broj dana uzetih u analizu
    pub sample_days:             i32,
    /// "good" | "degraded" | "replace" | "no_nominal" | "insufficient_data"
    pub status:                  String,
    /// Opis statusa na hrvatskom
    pub status_label:            String,
}

// ================================================================
// ALARM HEATMAP
// ================================================================

/// Dnevni sažetak alarma — za godišnji kalendarski heatmap
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AlarmHeatmapDay {
    /// Datum (YYYY-MM-DD)
    pub date:  chrono::NaiveDate,
    /// Broj 10-minutnih perioda s aktivnim alarmom taj dan
    pub count: i64,
}

/// Prosječna učestalost alarma po satu i danu u tjednu — za hour-of-day heatmap
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AlarmHeatmapHour {
    pub hour:  i32,   // 0–23
    pub dow:   i32,   // 0=Mon, 6=Sun (ISO tjedan − 1)
    pub count: f64,   // prosjek (0.0–1.0) — udio perioda s aktivnim alarmom
}

/// Kompletan odgovor za heatmap endpoint
#[derive(Debug, Serialize)]
pub struct AlarmHeatmapResponse {
    pub daily:  Vec<AlarmHeatmapDay>,
    pub hourly: Vec<AlarmHeatmapHour>,
}

// ================================================================
// PAGINATION
// ================================================================
#[derive(Debug, Serialize)]
pub struct Page<T: Serialize> {
    pub data:        Vec<T>,
    pub total:       i64,
    pub page:        i64,
    pub page_size:   i64,
    pub total_pages: i64,
}

impl<T: Serialize> Page<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 { (total + page_size - 1) / page_size } else { 0 };
        Self { data, total, page, page_size, total_pages }
    }
}
