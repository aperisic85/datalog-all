//! Campbell CR300 payload parser
//!
//! CR300 šalje JSON u formatu:
//! {
//!   "head": { "environment": { "station_name": "Galija", "table_name": "Alarms_10min" },
//!             "fields": [{"name": "Alarm_battery_voltage_flat"}, ...] },
//!   "data": [{"time": "2024-01-15T10:00:00", "vals": [0, 1, 0, ...]}, ...]
//! }

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

use crate::errors::{AppError, AppResult};
use crate::models::{DataloggerPayload, FieldDef};
use crate::models::domain::*;

// ── Helpers ──────────────────────────────────────────────────────────────

fn field_map(fields: &[FieldDef]) -> HashMap<String, usize> {
    fields.iter().enumerate()
        .map(|(i, f)| (f.name.to_lowercase(), i))
        .collect()
}

fn get_val<'a>(row: &'a Value, fm: &HashMap<String, usize>, name: &str) -> Option<&'a Value> {
    if let Some(vals) = row.get("vals").and_then(|v| v.as_array()) {
        return fm.get(&name.to_lowercase()).and_then(|i| vals.get(*i));
    }
    row.get(name)
}

fn as_f32(v: &Value) -> Option<f32> {
    match v {
        Value::Number(n) => n.as_f64().map(|f| f as f32),
        Value::String(s) if s == "NAN" || s == "nan" => None,
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) if s == "NAN" || s == "nan" => None,
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn as_i16(v: &Value) -> Option<i16> {
    match v {
        Value::Number(n) => n.as_i64().map(|i| i as i16),
        Value::String(s) if s == "NAN" => None,
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn parse_ts(val: &Value) -> Option<DateTime<Utc>> {
    val.as_str().and_then(|s| {
        DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .ok().map(|ndt| ndt.and_utc())
            })
    })
}

pub fn detect_station_name(payload: &DataloggerPayload) -> Option<String> {
    payload.head.environment.as_ref().and_then(|e| e.station_name.clone())
}

pub fn detect_table_name(payload: &DataloggerPayload) -> Option<String> {
    payload.head.environment.as_ref().and_then(|e| e.table_name.clone())
}

// ── ALARMI ───────────────────────────────────────────────────────────────

pub fn parse_alarms(payload: &DataloggerPayload, station_id: &str) -> AppResult<Vec<AlarmInsert>> {
    let fm = field_map(&payload.head.fields);
    payload.data.iter().map(|row| {
        let recorded_at = row.get("time").and_then(parse_ts)
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in alarm row".into()))?;

        // Mapiranje: stari naziv iz CR300 → novi skraćeni naziv u bazi
        Ok(AlarmInsert {
            object_id:  None, // trigger fn_resolve_object_id popunjava automatski
            station_id: station_id.to_string(),
            recorded_at,
            alarm_datalogger_high_temp:    get_field_i16(row, &fm, &["Alarm_datalogger_high_temp"]),
            alarm_datalogger_high_voltage: get_field_i16(row, &fm, &["Alarm_datalogger_high_voltage"]),
            alarm_datalogger_other_error:  get_field_i16(row, &fm, &["Alarm_datalogger_other_error"]),
            alarm_battery_voltage_low:     get_field_i16(row, &fm, &["Alarm_battery_voltage_low"]),
            alarm_battery_voltage_flat:    get_field_i16(row, &fm, &["Alarm_battery_voltage_flat"]),
            alarm_battery_other_error:     get_field_i16(row, &fm, &["Alarm_battery_other_error"]),
            alarm_garmin_comm_failed:      get_field_i16(row, &fm, &["Alarm_garmin_communication_failed", "Alarm_garmin_comm_failed"]),
            alarm_garmin_other_error:      get_field_i16(row, &fm, &["Alarm_garmin_other_error"]),
            alarm_station_out_of_radius:   get_field_i16(row, &fm, &["Alarm_station_out_of_radius"]),
            alarm_lantern_night_light_off: get_field_i16(row, &fm, &["Alarm_lantern_night_light_off"]),
            alarm_lantern_day_light_on:    get_field_i16(row, &fm, &["Alarm_lantern_day_light_on"]),
            alarm_lantern_comm_failed:     get_field_i16(row, &fm, &["Alarm_lantern_communication_failed", "Alarm_lantern_comm_failed"]),
            alarm_lantern_other_error:     get_field_i16(row, &fm, &["Alarm_lantern_other_error"]),
            alarm_modem_network_error:     get_field_i16(row, &fm, &["Alarm_modem_network_error"]),
            alarm_modem_other_error:       get_field_i16(row, &fm, &["Alarm_modem_other_error"]),
            alarm_station_other_error:     get_field_i16(row, &fm, &["Alarm_station_other_error"]),
            // Novi alarmi — modularni program (Tip 2); stari program šalje 0 jer polje ne postoji
            alarm_visibility_comm_failed:     get_field_i16(row, &fm, &["Alarm_visibility_communication_failed"]),
            alarm_visibility_error:           get_field_i16(row, &fm, &["Alarm_visibility_error"]),
            alarm_fog_signal_off_during_fog:  get_field_i16(row, &fm, &["Alarm_fog_signal_off_during_fog"]),
            alarm_fog_signal_on_while_no_fog: get_field_i16(row, &fm, &["Alarm_fog_signal_on_while_no_fog"]),
        })
    }).collect()
}

// Pokušava više alternativnih naziva polja (stari i novi)
fn get_field_i16(row: &Value, fm: &HashMap<String, usize>, names: &[&str]) -> i16 {
    for name in names {
        if let Some(v) = get_val(row, fm, name) {
            if let Some(n) = as_i16(v) { return n; }
        }
    }
    0
}

// ── MJERENJA 10min ────────────────────────────────────────────────────────

pub fn parse_measurements_10min(payload: &DataloggerPayload, station_id: &str) -> AppResult<Vec<Measurement10minInsert>> {
    let fm = field_map(&payload.head.fields);
    payload.data.iter().map(|row| {
        let recorded_at = row.get("time").and_then(parse_ts)
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in measurements row".into()))?;

        let g = |names: &[&str]| -> Option<f32> {
            for n in names { if let Some(v) = get_val(row, &fm, n) { if let Some(f) = as_f32(v) { return Some(f); } } }
            None
        };
        let gf64 = |names: &[&str]| -> Option<f64> {
            for n in names { if let Some(v) = get_val(row, &fm, n) { if let Some(f) = as_f64(v) { return Some(f); } } }
            None
        };
        let gi16 = |names: &[&str]| -> Option<i16> {
            for n in names { if let Some(v) = get_val(row, &fm, n) { if let Some(i) = as_i16(v) { return Some(i); } } }
            None
        };

        Ok(Measurement10minInsert {
            object_id:   None,
            station_id:  station_id.to_string(),
            recorded_at,
            datalogger_temp_avg:         g(&["Datalogger_temperature_Avg"]),
            battery_voltage_avg:         g(&["Battery_voltage_Avg", "Battery_voltage_1min_Avg"]),
            battery_current_avg:         g(&["Battery_current_Avg", "Battery_current_1min_Avg"]),
            battery_status_smp:          gi16(&["Battery_status"]),
            battery_status_avg:          g(&["Battery_status_Avg"]),
            solar_voltage_avg:           g(&["Solar_panel_voltage_Avg"]),
            solar_daylight_smp:          gi16(&["Solar_panel_day_light"]),
            solar_daylight_avg:          g(&["Solar_panel_day_light_Avg"]),
            modem_power_avg:             g(&["Modem_power_state_Avg"]),
            internet_ok_avg:             g(&["Internet_connection_ok_Avg"]),
            garmin_comm_ok_avg:          g(&["Garmin_communication_ok_Avg"]),
            garmin_satellites_avg:       g(&["Garmin_number_of_sattelites_Avg", "Garmin_satellites_Avg"]),
            garmin_latitude_avg:         gf64(&["Garmin_latitude_Avg"]),
            garmin_longitude_avg:        gf64(&["Garmin_longitude_Avg"]),
            garmin_distance_avg:         g(&["Garmin_distance_Avg"]),
            lantern_comm_ok_avg:         g(&["Lantern_communication_ok_Avg"]),
            lantern_light_active_avg:    g(&["Lantern_light_active_Avg"]),
            lantern_current_active_avg:  g(&["Lantern_current_active_Avg"]),
            lantern_current_avg:         g(&["Lantern_current_Avg", "Lantern_current_1min_Avg"]),
            lantern_latitude_avg:        gf64(&["Lantern_latitude_Avg"]),
            lantern_longitude_avg:       gf64(&["Lantern_longitude_Avg"]),
            lantern_distance_avg:        g(&["Lantern_distance_Avg"]),
            // Novi senzori — modularni program (Tip 2)
            visibility_comm_ok_avg:      g(&["Visibility_communication_ok_Avg"]),
            visibility_value_avg:        g(&["Visibility_value_Avg"]),
            visibility_alarm_avg:        g(&["Visibility_alarm_Avg"]),
            visibility_error_smp:        gi16(&["Visibility_error"]),
            fog_signal_active_avg:       g(&["Fog_signal_current_active_Avg"]),
            fog_signal_current_avg:      g(&["Fog_signal_current_Avg"]),
        })
    }).collect()
}

// ── MJERENJA 1h ───────────────────────────────────────────────────────────

pub fn parse_measurements_1h(payload: &DataloggerPayload, station_id: &str) -> AppResult<Vec<Measurement1hInsert>> {
    let fm = field_map(&payload.head.fields);
    payload.data.iter().map(|row| {
        let recorded_at = row.get("time").and_then(parse_ts)
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in 1h measurements".into()))?;
        let g = |n: &str| get_val(row, &fm, n).and_then(|v| as_f32(v));
        Ok(Measurement1hInsert {
            object_id:   None,
            station_id:  station_id.to_string(),
            recorded_at,
            datalogger_temp_avg:      g("Datalogger_temperature_Avg"),
            battery_voltage_avg:      g("Battery_voltage_Avg"),
            battery_current_avg:      g("Battery_current_Avg"),
            battery_charge_tot:       g("Battery_charge_Tot"),
            battery_discharge_tot:    g("Battery_discharge_Tot"),
            battery_status_avg:       g("Battery_status_Avg"),
            solar_voltage_avg:        g("Solar_panel_voltage_Avg"),
            solar_daylight_avg:       g("Solar_panel_day_light_Avg"),
            modem_power_avg:          g("Modem_power_state_Avg"),
            internet_ok_avg:          g("Internet_connection_ok_Avg"),
            lantern_light_active_avg: g("Lantern_light_active_Avg"),
            lantern_current_avg:      g("Lantern_current_Avg"),
            // Novi senzori — modularni program (Tip 2)
            visibility_value_avg:     g("Visibility_value_Avg"),
            visibility_alarm_avg:     g("Visibility_alarm_Avg"),
            fog_signal_current_avg:   g("Fog_signal_current_Avg"),
        })
    }).collect()
}

// ── MJERENJA 24h ──────────────────────────────────────────────────────────

pub fn parse_measurements_24h(payload: &DataloggerPayload, station_id: &str) -> AppResult<Vec<Measurement24hInsert>> {
    let fm = field_map(&payload.head.fields);
    payload.data.iter().map(|row| {
        let recorded_at = row.get("time").and_then(parse_ts)
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in 24h measurements".into()))?;
        let g = |n: &str| get_val(row, &fm, n).and_then(|v| as_f32(v));
        Ok(Measurement24hInsert {
            object_id:   None,
            station_id:  station_id.to_string(),
            recorded_at,
            datalogger_temp_avg:      g("Datalogger_temperature_Avg"),
            battery_voltage_avg:      g("Battery_voltage_Avg"),
            battery_current_avg:      g("Battery_current_Avg"),
            battery_current_min:      g("Battery_current_Min"),
            battery_current_max:      g("Battery_current_Max"),
            battery_charge_tot:       g("Battery_charge_Tot"),
            battery_discharge_tot:    g("Battery_discharge_Tot"),
            battery_status_avg:       g("Battery_status_Avg"),
            solar_daylight_avg:       g("Solar_panel_day_light_Avg"),
            modem_power_avg:          g("Modem_power_state_Avg"),
            internet_ok_avg:          g("Internet_connection_ok_Avg"),
            lantern_light_active_avg: g("Lantern_light_active_Avg"),
            lantern_current_avg:      g("Lantern_current_Avg"),
            // Novi senzori — modularni program (Tip 2)
            visibility_value_avg:     g("Visibility_value_Avg"),
            fog_signal_current_avg:   g("Fog_signal_current_Avg"),
        })
    }).collect()
}

// ── EVENT LOG ────────────────────────────────────────────────────────────

pub fn parse_event_logs(payload: &DataloggerPayload, station_id: &str) -> AppResult<Vec<EventLogInsert>> {
    let fm = field_map(&payload.head.fields);
    payload.data.iter().map(|row| {
        let recorded_at = row.get("time").and_then(parse_ts)
            .ok_or_else(|| AppError::BadRequest("Missing timestamp in event log".into()))?;
        let log_level   = get_val(row, &fm, "Log_level").and_then(|v| as_i16(v)).unwrap_or(1);
        let log_message = get_val(row, &fm, "Log_message")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        Ok(EventLogInsert { object_id: None, station_id: station_id.to_string(), recorded_at, log_level, log_message })
    }).collect()
}
