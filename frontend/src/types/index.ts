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
  description?: string;
  notes?: string;
  is_active: boolean;
  polling_enabled: boolean;
  datalogger_url?: string;
  poll_interval_sec: number;
  commissioned_at?: string;
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
  lantern_current_avg?: number;
  lantern_latitude_avg?: number;
  lantern_longitude_avg?: number;
  lantern_distance_avg?: number;
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
  lantern_light_active_avg?: number;
  lantern_current_avg?: number;
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
  lantern_current_avg?: number;
  lantern_distance_avg?: number;
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
