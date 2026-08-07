-- ================================================================
-- Migration 015: csd_verzija — kategorije, ID oznaka, alarmi i statusi
--
-- Nakon uvida u izvorni kod RTU-a (funkcija CreateReturnStringToCenter)
-- registarska mapa je potpuna: registri 5–9, 13–18 i 25 su alarmi, 12/29/30
-- statusi, a 26 je struja izvora svjetla. Dosad su bili samo sirovi brojevi
-- u aton_readings.regs.
--
-- Ova migracija:
--   1. imenuje AtoN program `csd_verzija` i uvodi kategoriju (podverziju) 1–7,
--   2. dodaje statusna polja u aton_readings,
--   3. uvodi AtoN alarme u postojeću alarms tablicu, pa AtoN objekti dobivaju
--      alarme, obavijesti, potvrđivanje, odlaganje i heatmap bez iznimaka.
-- ================================================================

-- ────────────────────────────────────────────────────────────────
-- 1. Kategorija (podverzija) programa csd_verzija
-- ────────────────────────────────────────────────────────────────
ALTER TABLE objects
    ADD COLUMN IF NOT EXISTS aton_category SMALLINT NOT NULL DEFAULT 7;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'objects_aton_category_check') THEN
        ALTER TABLE objects
            ADD CONSTRAINT objects_aton_category_check
            CHECK (aton_category BETWEEN 1 AND 7);
    END IF;
END $$;

COMMENT ON COLUMN objects.aton_category IS
    'Podverzija programa csd_verzija (1-7). Određuje registarsku mapu odgovora. Implementirana je kategorija 7 (puni set, 31 registar).';

COMMENT ON COLUMN objects.aton_addr IS
    'ID oznaka objekta = Modbus adresa. RTU je pakira na početak svakog okvira, po njoj centar prepoznaje tko javlja.';

-- Postojeći AtoN objekti voze program csd_verzija
UPDATE objects
   SET program_version = 'csd_verzija'
 WHERE source_kind = 'aton_csd'
   AND program_version IS DISTINCT FROM 'csd_verzija';

-- Naziv programa slijedi kategoriju izvora — jedno mjesto istine, pa ga ne
-- treba pamtiti ni pri kreiranju ni pri uređivanju objekta.
CREATE OR REPLACE FUNCTION fn_sync_aton_program_version()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.source_kind = 'aton_csd' THEN
        NEW.program_version := 'csd_verzija';
    ELSIF NEW.program_version = 'csd_verzija' THEN
        -- objekt je prebačen natrag na CR300 → naziv AtoN programa više ne vrijedi
        NEW.program_version := NULL;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_aton_program_version ON objects;
CREATE TRIGGER trg_aton_program_version
    BEFORE INSERT OR UPDATE OF source_kind, program_version ON objects
    FOR EACH ROW EXECUTE FUNCTION fn_sync_aton_program_version();

-- ────────────────────────────────────────────────────────────────
-- 2. Statusi u aton_readings
-- ────────────────────────────────────────────────────────────────
ALTER TABLE aton_readings
    -- reg 26: trenutna struja izvora svjetla (Maxi Halo / LED), negativna
    ADD COLUMN IF NOT EXISTS struja_led_a      REAL,
    -- reg 12: 0 = sumrak/svitanje, 1 = noć, 2 = dan
    ADD COLUMN IF NOT EXISTS doba_dana         SMALLINT,
    -- reg 29/30: minuta od ponoći po satu RTU-a (NIJE analogna vrijednost)
    ADD COLUMN IF NOT EXISTS pocetak_noci_min  SMALLINT,
    ADD COLUMN IF NOT EXISTS kraj_noci_min     SMALLINT,
    -- kategorija kojom je zapis dekodiran (mapa se po kategoriji razlikuje)
    ADD COLUMN IF NOT EXISTS category          SMALLINT NOT NULL DEFAULT 7;

COMMENT ON COLUMN aton_readings.dnevna_potrosnja_a IS
    'sumMaxiDischargeEnergy — dnevna potrošnja izvora svjetla u Ah (ne A), negativna.';

-- ────────────────────────────────────────────────────────────────
-- 3. AtoN alarmi u postojećoj alarms tablici
--    Isti obrazac kao migracija 007 (senzori modularnog programa).
-- ────────────────────────────────────────────────────────────────
ALTER TABLE alarms
    ADD COLUMN IF NOT EXISTS alarm_aton_call_request      SMALLINT NOT NULL DEFAULT 0, -- reg 5
    ADD COLUMN IF NOT EXISTS alarm_aton_temperature       SMALLINT NOT NULL DEFAULT 0, -- reg 6
    ADD COLUMN IF NOT EXISTS alarm_aton_voltage_light     SMALLINT NOT NULL DEFAULT 0, -- reg 7
    ADD COLUMN IF NOT EXISTS alarm_aton_voltage_automat   SMALLINT NOT NULL DEFAULT 0, -- reg 8
    ADD COLUMN IF NOT EXISTS alarm_aton_door_open         SMALLINT NOT NULL DEFAULT 0, -- reg 9
    ADD COLUMN IF NOT EXISTS alarm_aton_flash_code        SMALLINT NOT NULL DEFAULT 0, -- reg 13
    ADD COLUMN IF NOT EXISTS alarm_aton_light_on_automat  SMALLINT NOT NULL DEFAULT 0, -- reg 15
    ADD COLUMN IF NOT EXISTS alarm_aton_automat_on_light  SMALLINT NOT NULL DEFAULT 0, -- reg 16
    ADD COLUMN IF NOT EXISTS alarm_aton_lamp_blown        SMALLINT NOT NULL DEFAULT 0, -- reg 17 bit 0
    ADD COLUMN IF NOT EXISTS alarm_aton_not_work_at_night SMALLINT NOT NULL DEFAULT 0, -- reg 17 bit 1
    ADD COLUMN IF NOT EXISTS alarm_aton_photocell_error   SMALLINT NOT NULL DEFAULT 0, -- reg 17 bit 2
    ADD COLUMN IF NOT EXISTS alarm_aton_work_at_day       SMALLINT NOT NULL DEFAULT 0; -- reg 25

-- Regeneriraj any_alarm_active (generated column mora obuhvatiti nova polja)
DROP INDEX IF EXISTS idx_alarms_active;
DROP INDEX IF EXISTS idx_alarms_ack;
DROP VIEW  IF EXISTS v_latest_alarms;

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
    alarm_fog_signal_on_while_no_fog     > 0 OR
    alarm_aton_call_request              > 0 OR
    alarm_aton_temperature               > 0 OR
    alarm_aton_voltage_light             > 0 OR
    alarm_aton_voltage_automat           > 0 OR
    alarm_aton_door_open                 > 0 OR
    alarm_aton_flash_code                > 0 OR
    alarm_aton_light_on_automat          > 0 OR
    alarm_aton_automat_on_light          > 0 OR
    alarm_aton_lamp_blown                > 0 OR
    alarm_aton_not_work_at_night         > 0 OR
    alarm_aton_photocell_error           > 0 OR
    alarm_aton_work_at_day               > 0
) STORED;

CREATE INDEX IF NOT EXISTS idx_alarms_active ON alarms (object_id, any_alarm_active) WHERE any_alarm_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_alarms_ack    ON alarms (acknowledged_at) WHERE acknowledged_at IS NULL AND any_alarm_active = TRUE;

-- ────────────────────────────────────────────────────────────────
-- 4. Alarm cache trigger — proširen novim tipovima
--    (nastavak na 010; brojanje vrsta alarma ostaje isto)
-- ────────────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION fn_update_alarm_cache()
RETURNS TRIGGER AS $$
DECLARE
    v_count      SMALLINT;
    v_worst      SMALLINT;
    v_summary    TEXT;
    v_any_active BOOLEAN;
    v_latest     RECORD;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM alarms
        WHERE object_id = NEW.object_id
          AND recorded_at >= NOW() - INTERVAL '24 hours'
          AND any_alarm_active = TRUE
          AND acknowledged_at IS NULL
    ) INTO v_any_active;

    IF v_any_active THEN
        SELECT * INTO v_latest
        FROM alarms
        WHERE object_id = NEW.object_id
          AND any_alarm_active = TRUE
          AND acknowledged_at IS NULL
        ORDER BY recorded_at DESC
        LIMIT 1;

        v_count := (
            (CASE WHEN v_latest.alarm_datalogger_high_temp       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_datalogger_high_voltage    > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_datalogger_other_error     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_battery_voltage_low        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_battery_voltage_flat       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_battery_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_garmin_comm_failed         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_garmin_other_error         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_station_out_of_radius      > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_night_light_off    > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_day_light_on       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_comm_failed        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_modem_network_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_modem_other_error          > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_station_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_visibility_comm_failed     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_visibility_error           > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_fog_signal_off_during_fog  > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_fog_signal_on_while_no_fog > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_call_request          > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_temperature           > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_voltage_light         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_voltage_automat       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_door_open             > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_flash_code            > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_light_on_automat      > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_automat_on_light      > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_lamp_blown            > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_not_work_at_night     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_photocell_error       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_aton_work_at_day           > 0 THEN 1 ELSE 0 END)
        );

        v_worst := CASE
            -- FATAL: svjetlo ne svijetli kad mora, ili je stanica izgubljena
            WHEN v_latest.alarm_battery_voltage_flat        > 0 OR
                 v_latest.alarm_lantern_night_light_off     > 0 OR
                 v_latest.alarm_fog_signal_off_during_fog   > 0 OR
                 v_latest.alarm_station_out_of_radius       > 0 OR
                 v_latest.alarm_aton_not_work_at_night      > 0 OR
                 v_latest.alarm_aton_lamp_blown             > 0 THEN 4
            -- ERROR
            WHEN v_latest.alarm_battery_voltage_low         > 0 OR
                 v_latest.alarm_garmin_comm_failed          > 0 OR
                 v_latest.alarm_lantern_comm_failed         > 0 OR
                 v_latest.alarm_modem_network_error         > 0 OR
                 v_latest.alarm_visibility_comm_failed      > 0 OR
                 v_latest.alarm_aton_voltage_light          > 0 OR
                 v_latest.alarm_aton_voltage_automat        > 0 OR
                 v_latest.alarm_aton_flash_code             > 0 OR
                 v_latest.alarm_aton_photocell_error        > 0 THEN 3
            -- WARN
            WHEN v_latest.alarm_datalogger_high_temp        > 0 OR
                 v_latest.alarm_datalogger_high_voltage     > 0 OR
                 v_latest.alarm_lantern_day_light_on        > 0 OR
                 v_latest.alarm_fog_signal_on_while_no_fog  > 0 OR
                 v_latest.alarm_visibility_error            > 0 OR
                 v_latest.alarm_aton_temperature            > 0 OR
                 v_latest.alarm_aton_door_open              > 0 OR
                 v_latest.alarm_aton_work_at_day            > 0 OR
                 v_latest.alarm_aton_light_on_automat       > 0 OR
                 v_latest.alarm_aton_automat_on_light       > 0 THEN 2
            ELSE 1
        END;
    ELSE
        v_count := 0;
        v_worst := NULL;
    END IF;

    IF NEW.any_alarm_active THEN
        SELECT string_agg(alarm_name, ', ') INTO v_summary FROM (
            SELECT unnest(ARRAY[
                CASE WHEN NEW.alarm_battery_voltage_flat          > 0 THEN 'Baterija prazna'                END,
                CASE WHEN NEW.alarm_battery_voltage_low           > 0 THEN 'Baterija niska'                 END,
                CASE WHEN NEW.alarm_lantern_night_light_off       > 0 THEN 'Fenjer ugašen noću'             END,
                CASE WHEN NEW.alarm_lantern_day_light_on          > 0 THEN 'Fenjer upaljen danju'           END,
                CASE WHEN NEW.alarm_lantern_comm_failed           > 0 THEN 'Fenjer: greška veze'            END,
                CASE WHEN NEW.alarm_garmin_comm_failed            > 0 THEN 'GPS: greška veze'               END,
                CASE WHEN NEW.alarm_station_out_of_radius         > 0 THEN 'Van radijusa'                   END,
                CASE WHEN NEW.alarm_modem_network_error           > 0 THEN 'Modem: nema mreže'              END,
                CASE WHEN NEW.alarm_datalogger_high_temp          > 0 THEN 'Visoka temperatura'             END,
                CASE WHEN NEW.alarm_datalogger_high_voltage       > 0 THEN 'Visoki napon'                   END,
                CASE WHEN NEW.alarm_visibility_comm_failed        > 0 THEN 'Vidljivost: greška veze'        END,
                CASE WHEN NEW.alarm_visibility_error              > 0 THEN 'Vidljivost: greška senzora'     END,
                CASE WHEN NEW.alarm_fog_signal_off_during_fog     > 0 THEN 'Maglenka: nije aktivna u magli' END,
                CASE WHEN NEW.alarm_fog_signal_on_while_no_fog    > 0 THEN 'Maglenka: aktivna bez magle'    END,
                CASE WHEN NEW.alarm_aton_lamp_blown               > 0 THEN 'Pregorena žarulja'              END,
                CASE WHEN NEW.alarm_aton_not_work_at_night        > 0 THEN 'Ne radi po noći'                END,
                CASE WHEN NEW.alarm_aton_photocell_error          > 0 THEN 'Greška fotoćelije'              END,
                CASE WHEN NEW.alarm_aton_flash_code               > 0 THEN 'Pogrešna karakteristika bljeska' END,
                CASE WHEN NEW.alarm_aton_voltage_light            > 0 THEN 'Napon baterije GL.SVJ.'         END,
                CASE WHEN NEW.alarm_aton_voltage_automat          > 0 THEN 'Napon baterije automata'        END,
                CASE WHEN NEW.alarm_aton_light_on_automat         > 0 THEN 'Svjetlo na bateriji automata'   END,
                CASE WHEN NEW.alarm_aton_automat_on_light         > 0 THEN 'Automat na bateriji svjetla'    END,
                CASE WHEN NEW.alarm_aton_work_at_day              > 0 THEN 'Svjetlo radi po danu'           END,
                CASE WHEN NEW.alarm_aton_temperature              > 0 THEN 'Temperatura izvan granica'      END,
                CASE WHEN NEW.alarm_aton_door_open                > 0 THEN 'Vrata otvorena'                 END,
                CASE WHEN NEW.alarm_aton_call_request             > 0 THEN 'Zahtjev za pozivom'             END
            ]) AS alarm_name
        ) t WHERE alarm_name IS NOT NULL;
    END IF;

    UPDATE objects SET
        alarm_active       = v_any_active,
        alarm_count        = COALESCE(v_count, 0),
        alarm_worst_level  = CASE WHEN v_any_active THEN v_worst                            ELSE NULL               END,
        alarm_last_seen_at = CASE WHEN v_any_active THEN NOW()                              ELSE alarm_last_seen_at END,
        alarm_summary      = CASE WHEN v_any_active THEN COALESCE(v_summary, alarm_summary) ELSE NULL               END
    WHERE id = NEW.object_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ────────────────────────────────────────────────────────────────
-- 5. Viewovi koji su ovisili o obrisanim objektima
-- ────────────────────────────────────────────────────────────────
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
    a.alarm_visibility_comm_failed,
    a.alarm_visibility_error,
    a.alarm_fog_signal_off_during_fog,
    a.alarm_fog_signal_on_while_no_fog,
    a.alarm_aton_call_request,
    a.alarm_aton_temperature,
    a.alarm_aton_voltage_light,
    a.alarm_aton_voltage_automat,
    a.alarm_aton_door_open,
    a.alarm_aton_flash_code,
    a.alarm_aton_light_on_automat,
    a.alarm_aton_automat_on_light,
    a.alarm_aton_lamp_blown,
    a.alarm_aton_not_work_at_night,
    a.alarm_aton_photocell_error,
    a.alarm_aton_work_at_day
FROM alarms a
ORDER BY a.object_id, a.recorded_at DESC;

-- v_latest_aton_readings: dodaj statusna polja
-- DROP + CREATE jer nove kolone idu prije `regs` (CREATE OR REPLACE ne smije
-- mijenjati redoslijed ni nazive postojećih kolona).
DROP VIEW IF EXISTS v_latest_aton_readings;
CREATE VIEW v_latest_aton_readings AS
SELECT DISTINCT ON (a.object_id)
    a.object_id,
    a.station_id,
    a.recorded_at,
    a.received_at,
    a.temp_trenutna_c,
    a.temp_0100_c,
    a.temp_1300_c,
    a.gl_svj_napon_v,
    a.gl_svj_struja_a,
    a.automat_napon_v,
    a.automat_struja_a,
    a.prosjek_napon_gl_svj_v,
    a.prosjek_napon_automat_v,
    a.punjenje_gl_svj_a,
    a.punjenje_automat_a,
    a.potrosnja_gl_svj_a,
    a.potrosnja_automat_a,
    a.potrosnja_izvor_a,
    a.dnevna_potrosnja_a,
    a.struja_led_a,
    a.doba_dana,
    a.pocetak_noci_min,
    a.kraj_noci_min,
    a.category,
    a.regs
FROM aton_readings a
ORDER BY a.object_id, a.recorded_at DESC;

-- v_objects: izloži kategoriju (DROP + CREATE jer se dodaje kolona)
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
    -- Kategorija izvora + AtoN konfiguracija
    o.source_kind,
    o.aton_snopsy_endpoint,
    o.aton_number,
    o.aton_addr,
    o.aton_reg_count,
    o.aton_sync_clock,
    o.aton_category,
    -- Alarm cache
    o.alarm_active,
    o.alarm_count,
    o.alarm_worst_level,
    o.alarm_last_seen_at,
    o.alarm_summary,
    -- Battery capacity estimator
    o.nominal_battery_capacity_ah,
    -- Silent station detection
    o.silence_timeout_minutes,
    o.last_measurement_at,
    (
        o.last_measurement_at IS NOT NULL
        AND o.is_active = TRUE
        AND o.last_measurement_at < NOW() - (o.silence_timeout_minutes * INTERVAL '1 minute')
    ) AS is_silent,
    -- Tip objekta
    st.code AS type_code,
    st.name AS type_name,
    st.icon AS type_icon,
    -- Regija
    r.id    AS region_id,
    r.name  AS region_name,
    r.code  AS region_code,
    r.color AS region_color,
    -- Slike
    (SELECT storage_url FROM object_images
     WHERE object_id = o.id AND is_primary = TRUE LIMIT 1) AS primary_image_url,
    (SELECT COUNT(*) FROM object_images WHERE object_id = o.id) AS image_count
FROM objects o
LEFT JOIN station_types st ON o.station_type_id = st.id
JOIN      regions        r  ON o.region_id = r.id;
