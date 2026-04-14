-- ================================================================
-- Migration 011: Battery Capacity Estimator + Silent Station Alert
--
-- Feature 1: Battery Capacity Estimator
--   Dodaje nominal_battery_capacity_ah na objects za usporedbu s
--   estimiranim kapacitetom iz battery_charge_tot/battery_discharge_tot.
--
-- Feature 2: "Tiha" stanica alert
--   Dodaje silence_timeout_minutes (konfigurabilan per stanica) i
--   last_measurement_at koji se automatski ažurira triggerom pri
--   svakom insertu mjerenja.
--   is_silent je computed kolona u v_objects viewu.
-- ================================================================

-- ================================================================
-- Nove kolone na objects tablici
-- ================================================================
ALTER TABLE objects
    ADD COLUMN IF NOT EXISTS nominal_battery_capacity_ah REAL,
    ADD COLUMN IF NOT EXISTS silence_timeout_minutes     INTEGER NOT NULL DEFAULT 120,
    ADD COLUMN IF NOT EXISTS last_measurement_at         TIMESTAMPTZ;

COMMENT ON COLUMN objects.nominal_battery_capacity_ah IS
    'Nominalni kapacitet baterije u Ah (npr. 200 za 200Ah bateriju). '
    'Koristi se za usporedbu s estimiranim efektivnim kapacitetom.';

COMMENT ON COLUMN objects.silence_timeout_minutes IS
    'Maksimalno dopušteno vrijeme bez mjerenja (u minutama) prije nego '
    'se stanica smatra "tihom". Default 120 min (2 sata).';

COMMENT ON COLUMN objects.last_measurement_at IS
    'Zadnji put kad je primljeno mjerenje od ove stanice (bilo koji tip). '
    'Automatski ažurirano triggerom pri insertu u measurements_10min.';

CREATE INDEX IF NOT EXISTS idx_obj_last_meas
    ON objects (last_measurement_at)
    WHERE is_active = TRUE;

-- ================================================================
-- Trigger: automatski ažuriraj last_measurement_at pri insertu mjerenja
-- Ažurira se samo ako je novi recorded_at noviji od dosadašnjeg
-- ================================================================
CREATE OR REPLACE FUNCTION fn_update_last_measurement()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.object_id IS NOT NULL THEN
        UPDATE objects
        SET last_measurement_at = GREATEST(
            COALESCE(last_measurement_at, NEW.recorded_at - INTERVAL '999 days'),
            NEW.recorded_at
        )
        WHERE id = NEW.object_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Pratimo samo 10min mjerenja (najčešća — svakih 10 min)
-- 1h i 24h su agregirane vrijednosti i kasne, pa bi mogle
-- lažno prikazati stariji last_measurement_at.
CREATE TRIGGER trg_last_meas_10min
    AFTER INSERT ON measurements_10min
    FOR EACH ROW EXECUTE FUNCTION fn_update_last_measurement();

-- ================================================================
-- Ažuriraj v_objects view (dodaj nova polja + is_silent computed)
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
    -- Battery capacity estimator
    o.nominal_battery_capacity_ah,
    -- Silent station detection
    o.silence_timeout_minutes,
    o.last_measurement_at,
    -- Computed: je li stanica "tiha" (nema mjerenja dulje od zadanog praga)?
    -- NULL last_measurement_at = nikad nije slala (nova stanica) → nije tiha
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
    -- Primarna slika
    (SELECT storage_url FROM object_images
     WHERE object_id = o.id AND is_primary = TRUE LIMIT 1) AS primary_image_url,
    (SELECT COUNT(*) FROM object_images WHERE object_id = o.id) AS image_count
FROM objects o
LEFT JOIN station_types st ON o.station_type_id = st.id
JOIN      regions        r  ON o.region_id = r.id;
