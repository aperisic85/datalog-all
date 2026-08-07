// Aktivirani moduli na objektu (novi modularni CR300 program)
export interface ProgramFeatures {
  sealite?: boolean;
  navlite?: boolean;
  modem?: boolean;
  modem_on_other_station?: boolean;
  vaisala_pwd20?: boolean;
  visibility_on_other_station?: boolean;
  fog_signal?: boolean;
}

export interface Region {
  id: string;
  name: string;
  code: string;
  description?: string;
  color: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface StationType {
  id: number;
  code: string;
  name: string;
  icon?: string;
}

export interface ObjectView {
  id: string;
  station_id: string;
  name: string;
  short_name?: string;
  latitude?: number;
  longitude?: number;
  location_name?: string;
  allowed_radius_m?: number;
  description?: string;
  notes?: string;
  is_active: boolean;
  polling_enabled: boolean;
  datalogger_url?: string;
  poll_interval_sec: number;
  commissioned_at?: string;
  // Program tip — null = Tip 1 (Galija), postavljen = Tip 2 (Modularni)
  program_version?: string;
  program_features?: ProgramFeatures;
  alarm_active: boolean;
  alarm_count: number;
  alarm_worst_level?: number;
  alarm_last_seen_at?: string;
  alarm_summary?: string;
  type_code?: string;
  type_name?: string;
  type_icon?: string;
  region_id: string;
  region_name: string;
  region_code: string;
  region_color: string;
  primary_image_url?: string;
  image_count?: number;
  // Battery capacity estimator
  nominal_battery_capacity_ah?: number;
  // Silent station detection
  silence_timeout_minutes: number;
  last_measurement_at?: string;
  is_silent: boolean;
  // Kategorija izvora podataka
  source_kind: SourceKind;
  // AtoN (CSD preko snopsy_r proxyja) — samo za source_kind === 'aton_csd'
  aton_snopsy_endpoint?: string;
  aton_number?: string;
  aton_addr?: number;
  aton_reg_count: number;
  aton_sync_clock: boolean;
}

/** Kategorija izvora: CR300 datalogger preko HTTP-a ili AtoN RTU preko CSD-a. */
export type SourceKind = 'cr300_http' | 'aton_csd';

// ── AtoN (izvor `aton_csd`) ────────────────────────────────────────────────

export interface AtonReading {
  id?: number;
  object_id?: string;
  station_id: string;
  recorded_at: string;
  received_at: string;
  temp_trenutna_c?: number;
  temp_0100_c?: number;
  temp_1300_c?: number;
  gl_svj_napon_v?: number;
  gl_svj_struja_a?: number;
  automat_napon_v?: number;
  automat_struja_a?: number;
  prosjek_napon_gl_svj_v?: number;
  prosjek_napon_automat_v?: number;
  punjenje_gl_svj_a?: number;
  punjenje_automat_a?: number;
  potrosnja_gl_svj_a?: number;
  potrosnja_automat_a?: number;
  potrosnja_izvor_a?: number;
  dnevna_potrosnja_a?: number;
  /** Svih 31 sirovih registara — alarm/status bitovi još nisu mapirani. */
  regs: number[];
}

export interface AtonPollResult {
  station_id: string;
  success: boolean;
  error?: string;
  temperatura_c?: number;
  gl_svj_napon_v?: number;
  gl_svj_struja_a?: number;
  automat_napon_v?: number;
  automat_struja_a?: number;
}

export interface Measurement10min {
  id: number;
  object_id?: string;
  station_id: string;
  recorded_at: string;
  received_at: string;
  datalogger_temp_avg?: number;
  battery_voltage_avg?: number;
  battery_current_avg?: number;
  battery_status_smp?: number;
  battery_status_avg?: number;
  solar_voltage_avg?: number;
  solar_daylight_smp?: number;
  solar_daylight_avg?: number;
  modem_power_avg?: number;
  internet_ok_avg?: number;
  garmin_comm_ok_avg?: number;
  garmin_satellites_avg?: number;
  garmin_latitude_avg?: number;
  garmin_longitude_avg?: number;
  garmin_distance_avg?: number;
  lantern_comm_ok_avg?: number;
  lantern_light_active_avg?: number;
  lantern_current_active_avg?: number;
  lantern_current_avg?: number;
  lantern_latitude_avg?: number;
  lantern_longitude_avg?: number;
  lantern_distance_avg?: number;
  // Novi senzori (modularni program)
  visibility_comm_ok_avg?: number;
  visibility_value_avg?: number;
  visibility_alarm_avg?: number;
  visibility_error_smp?: number;
  fog_signal_active_avg?: number;
  fog_signal_current_avg?: number;
}

export interface Measurement1h {
  id: number;
  station_id: string;
  recorded_at: string;
  battery_voltage_avg?: number;
  battery_current_avg?: number;
  battery_charge_tot?: number;
  battery_discharge_tot?: number;
  battery_status_avg?: number;
  solar_voltage_avg?: number;
  solar_daylight_avg?: number;
  datalogger_temp_avg?: number;
  internet_ok_avg?: number;
  lantern_light_active_avg?: number;
  lantern_current_avg?: number;
  // Novi senzori (modularni program)
  visibility_value_avg?: number;
  visibility_alarm_avg?: number;
  fog_signal_current_avg?: number;
}

export interface Measurement24h {
  id: number;
  object_id?: string;
  station_id: string;
  recorded_at: string;
  received_at: string;
  datalogger_temp_avg?: number;
  battery_voltage_avg?: number;
  battery_current_avg?: number;
  battery_current_min?: number;
  battery_current_max?: number;
  battery_charge_tot?: number;
  battery_discharge_tot?: number;
  battery_status_avg?: number;
  solar_daylight_avg?: number;
  modem_power_avg?: number;
  internet_ok_avg?: number;
  lantern_light_active_avg?: number;
  lantern_current_avg?: number;
  // Novi senzori (modularni program)
  visibility_value_avg?: number;
  fog_signal_current_avg?: number;
}

export interface AlarmListItem {
  id: number;
  object_id: string;
  object_name: string;
  station_id: string;
  region_id: string;
  region_name: string;
  region_code: string;
  region_color: string;
  location_name?: string;
  recorded_at: string;
  acknowledged_at?: string;
  acknowledged_by?: string;
  any_alarm_active: boolean;
  alarm_datalogger_high_temp: number;
  alarm_datalogger_high_voltage: number;
  alarm_datalogger_other_error: number;
  alarm_battery_voltage_low: number;
  alarm_battery_voltage_flat: number;
  alarm_battery_other_error: number;
  alarm_garmin_comm_failed: number;
  alarm_garmin_other_error: number;
  alarm_station_out_of_radius: number;
  alarm_lantern_night_light_off: number;
  alarm_lantern_day_light_on: number;
  alarm_lantern_comm_failed: number;
  alarm_lantern_other_error: number;
  alarm_modem_network_error: number;
  alarm_modem_other_error: number;
  alarm_station_other_error: number;
  // Novi alarmi (modularni program)
  alarm_visibility_comm_failed: number;
  alarm_visibility_error: number;
  alarm_fog_signal_off_during_fog: number;
  alarm_fog_signal_on_while_no_fog: number;
}

// Alarm shelving — privremeno odloženi alarmi
export interface AlarmShelf {
  id: string;
  object_id: string;
  object_name: string;
  station_id: string;
  region_name: string;
  /** null = shelvani svi alarmi objekta */
  alarm_type: string | null;
  reason?: string;
  shelved_by: string;
  shelved_at: string;
  expires_at: string;
}

export interface AlarmRecord {
  id: number;
  station_id: string;
  recorded_at: string;
  alarm_datalogger_high_temp: number;
  alarm_datalogger_high_voltage: number;
  alarm_datalogger_other_error: number;
  alarm_battery_voltage_low: number;
  alarm_battery_voltage_flat: number;
  alarm_battery_other_error: number;
  alarm_garmin_comm_failed: number;
  alarm_garmin_other_error: number;
  alarm_station_out_of_radius: number;
  alarm_lantern_night_light_off: number;
  alarm_lantern_day_light_on: number;
  alarm_lantern_comm_failed: number;
  alarm_lantern_other_error: number;
  alarm_modem_network_error: number;
  alarm_modem_other_error: number;
  alarm_station_other_error: number;
  // Novi alarmi (modularni program)
  alarm_visibility_comm_failed: number;
  alarm_visibility_error: number;
  alarm_fog_signal_off_during_fog: number;
  alarm_fog_signal_on_while_no_fog: number;
  any_alarm_active: boolean;
}

export interface EventLogRecord {
  id: number;
  station_id: string;
  recorded_at: string;
  log_level: number;
  log_message: string;
}

export interface LatestMeasurement {
  object_id?: string;
  station_id?: string;
  recorded_at?: string;
  datalogger_temp_avg?: number;
  battery_voltage_avg?: number;
  battery_current_avg?: number;
  battery_status_smp?: number;
  solar_voltage_avg?: number;
  solar_daylight_smp?: number;
  modem_power_avg?: number;
  internet_ok_avg?: number;
  garmin_comm_ok_avg?: number;
  garmin_satellites_avg?: number;
  garmin_latitude_avg?: number;
  garmin_longitude_avg?: number;
  garmin_distance_avg?: number;
  lantern_comm_ok_avg?: number;
  lantern_light_active_avg?: number;
  lantern_current_active_avg?: number;
  lantern_current_avg?: number;
  lantern_distance_avg?: number;
  // Novi senzori (modularni program)
  visibility_comm_ok_avg?: number;
  visibility_value_avg?: number;
  visibility_alarm_avg?: number;
  fog_signal_active_avg?: number;
  fog_signal_current_avg?: number;
}

export interface RegionSummary {
  region_id?: string;
  region_name?: string;
  region_code?: string;
  region_color?: string;
  total_objects?: number;
  active_objects?: number;
  objects_in_alarm?: number;
  worst_alarm_level?: number;
  avg_battery_voltage?: number;
  battery_flat_count?: number;
  battery_low_count?: number;
  lanterns_on_count?: number;
}

export interface UserPublic {
  id: string;
  username: string;
  email: string;
  full_name?: string;
  role: string;
  is_active: boolean;
  last_login_at?: string;
  created_at: string;
}

export interface UserRegionAccessView {
  id: string;
  user_id: string;
  region_id: string;
  region_name: string;
  region_code: string;
  region_color: string;
  permission: string;
  granted_at: string;
}

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: UserPublic;
}

export interface Page<T> {
  data: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

// ── Alarm heatmap ─────────────────────────────────────────────────────────────

export interface AlarmHeatmapDay {
  /** ISO datum "YYYY-MM-DD" */
  date: string;
  /** Broj 10-min perioda s aktivnim alarmom taj dan */
  count: number;
}

export interface AlarmHeatmapHour {
  hour: number;   // 0–23
  dow: number;    // 0=pon, 6=ned
  count: number;  // udio perioda s aktivnim alarmom (0.0–1.0)
}

export interface AlarmHeatmapData {
  daily: AlarmHeatmapDay[];
  hourly: AlarmHeatmapHour[];
}

// ── Vremenski uvjeti (Open-Meteo) ─────────────────────────────────────────────

export interface WeatherHour {
  time: string;                   // ISO8601 UTC
  shortwave_radiation?: number;  // W/m²
  cloud_cover?: number;          // %
  wind_speed_10m?: number;       // km/h
  precipitation?: number;        // mm
  temperature_2m?: number;       // °C
}

export interface WeatherResponse {
  latitude: number;
  longitude: number;
  timezone: string;
  hours: WeatherHour[];
}

// ── Solarni efikasnost score ───────────────────────────────────────────────────

export interface SolarDayScore {
  date: string;             // "YYYY-MM-DD"
  insolation_kwh: number;  // kWh/m²
  score?: number;           // 0–120 (100 = nominalno, >100 moguće uz povoljan kut)
  sample_count: number;
}

export interface SolarEfficiency {
  object_id: string;
  computed_at: string;
  /** Ukupni score 0–120, 100 = nominalna efikasnost */
  score?: number;
  /** "good" | "warn" | "critical" | "insufficient_data" */
  status: string;
  status_label: string;
  message: string;
  baseline_ratio?: number;
  recent_ratio?: number;
  sample_count_baseline: number;
  sample_count_recent: number;
  daily_scores: SolarDayScore[];
}

/**
 * Procjena efektivnog kapaciteta baterije iz dnevnih totalizatora.
 * Vraćen od GET /api/v1/objects/:id/battery/capacity
 */
export interface BatteryCapacityEstimate {
  object_id: string;
  computed_at: string;
  /** Nominalni kapacitet (Ah) iz konfiguracije objekta */
  nominal_capacity_ah?: number;
  /** Procijenjeni efektivni kapacitet (Ah) iz analize totalizatora */
  estimated_capacity_ah?: number;
  /** Zdravlje baterije (%) — estimated / nominal × 100, max 100 */
  health_percent?: number;
  /** Maksimalno jednodnevno pražnjenje (Ah) */
  max_daily_discharge_ah?: number;
  /** Kumulativni deficit u najduljem deficit runu (Ah) */
  max_deficit_run_ah?: number;
  /** Broj dana analiziranih */
  sample_days: number;
  /** "good" | "degraded" | "replace" | "no_nominal" | "insufficient_data" | "insufficient_discharge" */
  status: string;
  /** Opis statusa na hrvatskom */
  status_label: string;
}

/**
 * Procjena zdravlja baterije iz ponašanja napona (danju vs noću).
 * Vraćen od GET /api/v1/objects/:id/battery/health
 */
export interface BatteryHealthAssessment {
  object_id: string;
  computed_at: string;
  /** "good" | "degraded" | "replace" | "insufficient_data" */
  status: string;
  status_label: string;
  sample_days: number;
  /** Broj napunjenih (sunčanih) dana korištenih za zaključak */
  charged_days: number;
  /** Medijan noćnog minimuma na napunjenim danima (V) */
  median_charged_night_min?: number;
  /** Medijan dnevnog raspona napona na napunjenim danima (V) */
  median_daily_swing?: number;
  /** Najniži noćni minimum na napunjenim danima (V) */
  worst_charged_night_min?: number;
  /** Detektirani napon sustava (12 ili 24) */
  system_voltage: number;
}

// ── Audit log ─────────────────────────────────────────────────────────────────

export interface AuditLogEntry {
  id: number;
  user_id?: string;
  username?: string;
  action: string;
  entity_type?: string;
  entity_id?: string;
  details?: Record<string, unknown>;
  ip_address?: string;
  created_at: string;
}

// ── Obavještavanje (notifikacije) ───────────────────────────────────────────

export type NotificationKind = 'telegram' | 'webhook' | 'slack';

export interface NotificationChannel {
  id: string;
  name: string;
  kind: NotificationKind;
  config: Record<string, unknown>;
  enabled: boolean;
  created_by?: string;
  created_at: string;
  updated_at: string;
}

export interface NotificationRule {
  id: string;
  name: string;
  channel_id: string;
  region_id?: string | null;
  min_severity: number;        // 1=Info, 2=Upozorenje, 3=Greška, 4=Kritično
  notify_on_clear: boolean;
  quiet_hours_start?: number | null;
  quiet_hours_end?: number | null;
  cooldown_minutes: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

// ── Kontrola uređaja ──────────────────────────────────────────────────────────

export interface SetValueResponse {
  success: boolean;
  message: string;
}

export interface NotificationLogEntry {
  id: number;
  channel_id?: string;
  channel_name?: string;
  object_id?: string;
  object_name?: string;
  alarm_type?: string;
  severity?: number;
  event: string;               // raised | cleared | test
  status: string;              // sent | failed
  error?: string;
  message?: string;
  created_at: string;
}

/**
 * Predikcija kvara baterije — linearni trend nad satnim mjerenjima napona.
 * Vraćen od GET /api/v1/objects/:id/battery/prediction
 */
export interface BatteryPrediction {
  object_id: string;
  computed_at: string;
  /** Zadnji izmjereni napon (V) */
  current_voltage?: number;
  /** Ekstrapolirani napon u trenutku izračuna (V) */
  trend_voltage?: number;
  /** Nagib trenda V/h — negativan = pražnjenje */
  slope_v_per_hour: number;
  /** "stable" | "charging" | "degrading" | "warning" | "critical" | "insufficient_data" */
  trend: string;
  hours_to_warning?: number;
  hours_to_critical?: number;
  days_to_warning?: number;
  days_to_critical?: number;
  sample_count: number;
  r_squared?: number;
}
