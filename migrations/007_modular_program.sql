-- ================================================================
-- Migration 007: Podrška za modularni program (novi tip dataloggera)
--
-- Novi program ima kondicionalne module (#IfDef):
--   c_lantern_sealite_installed      → SeaLite fenjer (SL serija)
--   c_lantern_navlite_installed      → NavLite fenjer
--   c_modem_installed                → lokalni modem
--   c_modem_on_other_station         → modem na drugoj stanici (PakBus)
--   c_vaisala_pwd20_installed        → Vaisala PWD20 vidljivost
--   c_visibility_on_other_station    → vidljivost s druge stanice
--   c_fog_signal_sfh_installed       → maglenka SFH
--
-- Razlike u odnosu na stari program (Galija):
--   - Nema Garmin GPS-a
--   - Nema station_out_of_radius alarma
--   - Nema modem_other_error alarma
--   - Novi senzori: Vaisala vidljivost, maglenka
--   - Modem može biti na drugoj stanici
--   - Mjerenja koriste 1-minutne prosjeke (aliasi Battery_voltage_1min itd.)
--   - Dodan Lantern_current_active (je li struja aktivna, 0/1)
-- ================================================================


-- ================================================================
-- Measurements_10min: nova polja za nove senzore
-- ================================================================

-- Vaisala PWD20 vidljivost (ili vidljivost s druge stanice)
ALTER TABLE measurements_10min
    ADD COLUMN IF NOT EXISTS visibility_comm_ok_avg   REAL,      -- Visibility_communication_ok Avg
    ADD COLUMN IF NOT EXISTS visibility_value_avg     REAL,      -- Visibility_value Avg (m)
    ADD COLUMN IF NOT EXISTS visibility_alarm_avg     REAL,      -- Visibility_alarm Avg (0/1)
    ADD COLUMN IF NOT EXISTS visibility_error_smp     SMALLINT;  -- Visibility_error Sample

-- Maglenka SFH
ALTER TABLE measurements_10min
    ADD COLUMN IF NOT EXISTS fog_signal_active_avg    REAL,      -- Fog_signal_current_active Avg (0/1)
    ADD COLUMN IF NOT EXISTS fog_signal_current_avg   REAL;      -- Fog_signal_current Avg (A)

-- Lantern current active (novi signal — je li struja fenjera detektirana)
ALTER TABLE measurements_10min
    ADD COLUMN IF NOT EXISTS lantern_current_active_avg REAL;    -- Lantern_current_active Avg (0/1)


-- ================================================================
-- Measurements_1h: nova polja
-- ================================================================
ALTER TABLE measurements_1h
    ADD COLUMN IF NOT EXISTS visibility_value_avg     REAL,
    ADD COLUMN IF NOT EXISTS visibility_alarm_avg     REAL,
    ADD COLUMN IF NOT EXISTS fog_signal_current_avg   REAL,
    ADD COLUMN IF NOT EXISTS internet_ok_avg          REAL;      -- Internet_connection_ok (novi program ga ima bez modema)


-- ================================================================
-- Measurements_24h: nova polja
-- ================================================================
ALTER TABLE measurements_24h
    ADD COLUMN IF NOT EXISTS visibility_value_avg     REAL,
    ADD COLUMN IF NOT EXISTS fog_signal_current_avg   REAL,
    ADD COLUMN IF NOT EXISTS internet_ok_avg          REAL;


-- ================================================================
-- Alarmi: nova polja za nove senzore
-- ================================================================
ALTER TABLE alarms
    ADD COLUMN IF NOT EXISTS alarm_visibility_comm_failed      SMALLINT NOT NULL DEFAULT 0,  -- Vaisala komunikacija
    ADD COLUMN IF NOT EXISTS alarm_visibility_error            SMALLINT NOT NULL DEFAULT 0,  -- Vaisala greška senzora
    ADD COLUMN IF NOT EXISTS alarm_fog_signal_off_during_fog   SMALLINT NOT NULL DEFAULT 0,  -- Maglenka: isključena u magli
    ADD COLUMN IF NOT EXISTS alarm_fog_signal_on_while_no_fog  SMALLINT NOT NULL DEFAULT 0;  -- Maglenka: uključena bez magle


-- ================================================================
-- Regeneriraj any_alarm_active generated column
-- (treba uključiti nova alarm polja)
-- ================================================================

-- 1. Ukloni objekte koji ovise o kolonni any_alarm_active
DROP INDEX IF EXISTS idx_alarms_active;
DROP INDEX IF EXISTS idx_alarms_ack;
DROP VIEW  IF EXISTS v_latest_alarms;

-- 2. Obrisi stari generated column
ALTER TABLE alarms DROP COLUMN IF EXISTS any_alarm_active;

ALTER TABLE alarms ADD COLUMN any_alarm_active BOOLEAN NOT NULL GENERATED ALWAYS AS (
    alarm_datalogger_high_temp           > 0 OR
    alarm_datalogger_high_voltage        > 0 OR
    alarm_datalogger_other_error         > 0 OR
    alarm_battery_voltage_low            > 0 OR
    alarm_battery_voltage_flat           > 0 OR
    alarm_battery_other_error            > 0 OR
    alarm_garmin_comm_failed             > 0 OR
    alarm_garmin_other_error             > 0 OR
    alarm_station_out_of_radius          > 0 OR
    alarm_lantern_night_light_off        > 0 OR
    alarm_lantern_day_light_on           > 0 OR
    alarm_lantern_comm_failed            > 0 OR
    alarm_lantern_other_error            > 0 OR
    alarm_modem_network_error            > 0 OR
    alarm_modem_other_error              > 0 OR
    alarm_station_other_error            > 0 OR
    alarm_visibility_comm_failed         > 0 OR
    alarm_visibility_error               > 0 OR
    alarm_fog_signal_off_during_fog      > 0 OR
    alarm_fog_signal_on_while_no_fog     > 0
) STORED;

-- Ponovo kreiraj index koji je ovisio o generated columnu
CREATE INDEX IF NOT EXISTS idx_alarms_active ON alarms (object_id, any_alarm_active) WHERE any_alarm_active = TRUE;

-- Ponovo kreiraj index za acknowledged (ovisio o any_alarm_active)
CREATE INDEX IF NOT EXISTS idx_alarms_ack
    ON alarms (acknowledged_at)
    WHERE acknowledged_at IS NULL AND any_alarm_active = TRUE;


-- ================================================================
-- Ažuriraj alarm cache trigger (fn_update_alarm_cache)
-- Dodaje nova alarm polja u summary
-- ================================================================
CREATE OR REPLACE FUNCTION fn_update_alarm_cache()
RETURNS TRIGGER AS $$
DECLARE
    v_count      SMALLINT;
    v_worst      SMALLINT;
    v_summary    TEXT;
    v_any_active BOOLEAN;
BEGIN
    v_any_active := NEW.any_alarm_active;

    SELECT
        COUNT(*) FILTER (WHERE any_alarm_active),
        MAX(
            CASE
                -- FATAL (razina 4)
                WHEN alarm_battery_voltage_flat           > 0 OR
                     alarm_lantern_night_light_off         > 0 OR
                     alarm_fog_signal_off_during_fog       > 0 OR
                     alarm_station_out_of_radius           > 0  THEN 4
                -- ERROR (razina 3)
                WHEN alarm_battery_voltage_low             > 0 OR
                     alarm_garmin_comm_failed              > 0 OR
                     alarm_lantern_comm_failed             > 0 OR
                     alarm_modem_network_error             > 0 OR
                     alarm_visibility_comm_failed          > 0  THEN 3
                -- WARN (razina 2)
                WHEN alarm_datalogger_high_temp            > 0 OR
                     alarm_datalogger_high_voltage         > 0 OR
                     alarm_lantern_day_light_on            > 0 OR
                     alarm_fog_signal_on_while_no_fog      > 0 OR
                     alarm_visibility_error                > 0  THEN 2
                ELSE 1
            END
        )
    INTO v_count, v_worst
    FROM alarms
    WHERE object_id = NEW.object_id
      AND recorded_at >= NOW() - INTERVAL '24 hours'
      AND any_alarm_active = TRUE;

    IF NEW.any_alarm_active THEN
        SELECT string_agg(alarm_name, ', ') INTO v_summary FROM (
            SELECT unnest(ARRAY[
                CASE WHEN NEW.alarm_battery_voltage_flat          > 0 THEN 'Baterija prazna'              END,
                CASE WHEN NEW.alarm_battery_voltage_low           > 0 THEN 'Baterija niska'               END,
                CASE WHEN NEW.alarm_lantern_night_light_off       > 0 THEN 'Fenjer ugašen noću'           END,
                CASE WHEN NEW.alarm_lantern_day_light_on          > 0 THEN 'Fenjer upaljen danju'         END,
                CASE WHEN NEW.alarm_lantern_comm_failed           > 0 THEN 'Fenjer: greška veze'          END,
                CASE WHEN NEW.alarm_garmin_comm_failed            > 0 THEN 'GPS: greška veze'             END,
                CASE WHEN NEW.alarm_station_out_of_radius         > 0 THEN 'Van radijusa'                 END,
                CASE WHEN NEW.alarm_modem_network_error           > 0 THEN 'Modem: nema mreže'            END,
                CASE WHEN NEW.alarm_datalogger_high_temp          > 0 THEN 'Visoka temperatura'           END,
                CASE WHEN NEW.alarm_datalogger_high_voltage       > 0 THEN 'Visoki napon'                 END,
                CASE WHEN NEW.alarm_visibility_comm_failed        > 0 THEN 'Vidljivost: greška veze'      END,
                CASE WHEN NEW.alarm_visibility_error              > 0 THEN 'Vidljivost: greška senzora'   END,
                CASE WHEN NEW.alarm_fog_signal_off_during_fog     > 0 THEN 'Maglenka: nije aktivna u magli' END,
                CASE WHEN NEW.alarm_fog_signal_on_while_no_fog    > 0 THEN 'Maglenka: aktivna bez magle'  END
            ]) AS alarm_name
        ) t WHERE alarm_name IS NOT NULL;
    END IF;

    UPDATE objects SET
        alarm_active       = v_any_active,
        alarm_count        = COALESCE(v_count, 0),
        alarm_worst_level  = v_worst,
        alarm_last_seen_at = CASE WHEN v_any_active THEN NOW() ELSE alarm_last_seen_at END,
        alarm_summary      = CASE WHEN v_any_active THEN v_summary ELSE NULL END
    WHERE id = NEW.object_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


-- ================================================================
-- Objects: podrška za info o programu i instaliranim modulima
-- ================================================================
ALTER TABLE objects
    ADD COLUMN IF NOT EXISTS program_version  VARCHAR(20),
    ADD COLUMN IF NOT EXISTS program_features JSONB;

COMMENT ON COLUMN objects.program_version  IS 'Verzija CR300 programa, npr. "0.05"';
COMMENT ON COLUMN objects.program_features IS
    'Aktivirani moduli na objektu. Primjer za South_Pozicija_2:
     {"sealite": true, "navlite": false, "modem": false,
      "modem_on_other_station": true, "vaisala_pwd20": false,
      "visibility_on_other_station": false, "fog_signal": false}';


-- ================================================================
-- Ažuriraj v_objects view (dodaj nova polja)
-- DROP + CREATE jer se dodaju kolone (CREATE OR REPLACE ne može
-- mijenjati redoslijed postojećih kolona)
-- ================================================================
DROP VIEW IF EXISTS v_objects;
CREATE VIEW v_objects AS
SELECT
    o.id,
    o.station_id,
    o.name,
    o.short_name,
    o.latitude,
    o.longitude,
    o.location_name,
    o.allowed_radius_m,
    o.description,
    o.notes,
    o.is_active,
    o.polling_enabled,
    o.datalogger_url,
    o.poll_interval_sec,
    o.commissioned_at,
    o.program_version,
    o.program_features,
    -- Alarm cache
    o.alarm_active,
    o.alarm_count,
    o.alarm_worst_level,
    o.alarm_last_seen_at,
    o.alarm_summary,
    -- Tip objekta
    st.code AS type_code,
    st.name AS type_name,
    st.icon AS type_icon,
    -- Regija
    r.id    AS region_id,
    r.name  AS region_name,
    r.code  AS region_code,
    r.color AS region_color,
    -- Primarna slika
    (SELECT storage_url FROM object_images
     WHERE object_id = o.id AND is_primary = TRUE LIMIT 1) AS primary_image_url,
    (SELECT COUNT(*) FROM object_images WHERE object_id = o.id) AS image_count
FROM objects o
LEFT JOIN station_types st ON o.station_type_id = st.id
JOIN      regions        r  ON o.region_id = r.id;


-- ================================================================
-- Ažuriraj v_latest_measurements view (dodaj nova polja)
-- CREATE OR REPLACE je ok jedino ako se nove kolone dodaju NA KRAJ
-- (v_region_summary ovisi o ovom viewu pa ga ne smijemo dropati)
-- ================================================================
CREATE OR REPLACE VIEW v_latest_measurements AS
SELECT DISTINCT ON (m.object_id)
    m.object_id,
    m.station_id,
    m.recorded_at,
    m.datalogger_temp_avg,
    m.battery_voltage_avg,
    m.battery_current_avg,
    m.battery_status_smp,
    m.solar_voltage_avg,
    m.solar_daylight_smp,
    m.modem_power_avg,
    m.internet_ok_avg,
    m.garmin_comm_ok_avg,
    m.garmin_satellites_avg,
    m.garmin_latitude_avg,
    m.garmin_longitude_avg,
    m.garmin_distance_avg,
    m.lantern_comm_ok_avg,
    m.lantern_light_active_avg,
    m.lantern_current_avg,
    m.lantern_distance_avg,
    -- Novi senzori (sve na kraj da ne poremetimo redoslijed)
    m.lantern_current_active_avg,
    m.visibility_comm_ok_avg,
    m.visibility_value_avg,
    m.visibility_alarm_avg,
    m.fog_signal_active_avg,
    m.fog_signal_current_avg
FROM measurements_10min m
ORDER BY m.object_id, m.recorded_at DESC;


-- ================================================================
-- Ponovo kreiraj v_latest_alarms (proširena s novim alarmima)
-- ================================================================
CREATE OR REPLACE VIEW v_latest_alarms AS
SELECT DISTINCT ON (a.object_id)
    a.object_id,
    a.station_id,
    a.recorded_at,
    a.any_alarm_active,
    a.alarm_datalogger_high_temp,
    a.alarm_datalogger_high_voltage,
    a.alarm_datalogger_other_error,
    a.alarm_battery_voltage_low,
    a.alarm_battery_voltage_flat,
    a.alarm_battery_other_error,
    a.alarm_garmin_comm_failed,
    a.alarm_garmin_other_error,
    a.alarm_station_out_of_radius,
    a.alarm_lantern_night_light_off,
    a.alarm_lantern_day_light_on,
    a.alarm_lantern_comm_failed,
    a.alarm_lantern_other_error,
    a.alarm_modem_network_error,
    a.alarm_modem_other_error,
    a.alarm_station_other_error,
    -- Novi alarmi (modularni program)
    a.alarm_visibility_comm_failed,
    a.alarm_visibility_error,
    a.alarm_fog_signal_off_during_fog,
    a.alarm_fog_signal_on_while_no_fog
FROM alarms a
ORDER BY a.object_id, a.recorded_at DESC;
