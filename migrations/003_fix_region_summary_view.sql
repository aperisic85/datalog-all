-- Fix v_region_summary: cast avg_battery_voltage to float8 (double precision)
-- so sqlx can map it to f64. ROUND(numeric, 2) returns NUMERIC which sqlx
-- cannot map to f64 without the bigdecimal feature.
-- Must DROP first because PostgreSQL won't change column type via CREATE OR REPLACE.
DROP VIEW IF EXISTS v_region_summary;
CREATE VIEW v_region_summary AS
SELECT
    r.id    AS region_id,
    r.name  AS region_name,
    r.code  AS region_code,
    r.color AS region_color,
    COUNT(o.id)                                                        AS total_objects,
    COUNT(o.id)  FILTER (WHERE o.is_active)                           AS active_objects,
    COUNT(o.id)  FILTER (WHERE o.alarm_active)                        AS objects_in_alarm,
    MAX(o.alarm_worst_level)                                          AS worst_alarm_level,
    ROUND(AVG(lm.battery_voltage_avg)::numeric, 2)::float8            AS avg_battery_voltage,
    COUNT(lm.object_id) FILTER (WHERE lm.battery_status_smp = 1)      AS battery_flat_count,
    COUNT(lm.object_id) FILTER (WHERE lm.battery_status_smp = 2)      AS battery_low_count,
    COUNT(lm.object_id) FILTER (WHERE lm.lantern_light_active_avg > 0.5) AS lanterns_on_count
FROM regions r
LEFT JOIN objects              o  ON o.region_id = r.id
LEFT JOIN v_latest_measurements lm ON lm.object_id = o.id
GROUP BY r.id, r.name, r.code, r.color;
