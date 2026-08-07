-- ================================================================
-- Migration 014: AtoN preko CSD-a / snopsy_r proxyja
--
-- Nova KATEGORIJA IZVORA podataka. Dosad je svaki objekt bio CR300
-- datalogger koji se proziva HTTP-om (ili sam pusha). AtoN stanice
-- (pomorske oznake tipa Prišnjak) nemaju HTTP — backend se preko
-- TCP-a spaja na `snopsy_r` proxy, sam postaje Modbus master, diže
-- CSD poziv, proziva RTU i dekodira Modbus ASCII odgovor.
--
-- Postojeći izvori ostaju netaknuti: source_kind = 'cr300_http'.
-- ================================================================

-- ── Kategorija izvora + AtoN konfiguracija po objektu ──────────────
ALTER TABLE objects
    ADD COLUMN IF NOT EXISTS source_kind              VARCHAR(20) NOT NULL DEFAULT 'cr300_http',
    -- snopsy_r endpoint (host:port novog Pija, port 2007).
    -- Više objekata smije dijeliti isti endpoint — prozivaju se serijski.
    ADD COLUMN IF NOT EXISTS aton_snopsy_endpoint     VARCHAR(200),
    -- Podatkovni telefonski broj RTU-a (u nadzoru: "TEL. PODATKOVNI")
    ADD COLUMN IF NOT EXISTS aton_number              VARCHAR(40),
    -- Modbus adresa RTU-a (Prišnjak = 51)
    ADD COLUMN IF NOT EXISTS aton_addr                SMALLINT,
    ADD COLUMN IF NOT EXISTS aton_reg_count           SMALLINT    NOT NULL DEFAULT 31,
    -- Sinkronizirati sat RTU-a (func 0x10 @ reg 100) prije prozivanja?
    ADD COLUMN IF NOT EXISTS aton_sync_clock          BOOLEAN     NOT NULL DEFAULT FALSE,
    -- CSD poziv traje ~10-20 s → konzervativni rokovi i interval
    ADD COLUMN IF NOT EXISTS aton_connect_timeout_sec SMALLINT    NOT NULL DEFAULT 15,
    ADD COLUMN IF NOT EXISTS aton_response_timeout_sec SMALLINT   NOT NULL DEFAULT 10;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'objects_source_kind_check'
    ) THEN
        ALTER TABLE objects
            ADD CONSTRAINT objects_source_kind_check
            CHECK (source_kind IN ('cr300_http', 'aton_csd'));
    END IF;
END $$;

COMMENT ON COLUMN objects.source_kind IS
    'Kategorija izvora: cr300_http = Campbell CR300 (HTTP push/pull), aton_csd = AtoN RTU preko CSD-a i snopsy_r proxyja';
COMMENT ON COLUMN objects.aton_snopsy_endpoint IS
    'host:port snopsy_r proxyja. Jedan endpoint = jedan modem = jedna linija (serijsko prozivanje).';
COMMENT ON COLUMN objects.aton_number IS
    'Podatkovni telefonski broj RTU-a koji se bira (ATD).';

CREATE INDEX IF NOT EXISTS idx_objects_aton_pollable
    ON objects (source_kind)
    WHERE source_kind = 'aton_csd' AND polling_enabled = TRUE;

-- ── Očitanja AtoN stanica (typed + sirovi registri) ───────────────
-- Puni dekodirani Aton se čuva ovdje; podskup koji postojeći nadzor
-- već servira (temperatura, napon/struja glavne baterije) paralelno
-- ide u measurements_10min, pa AtoN objekt radi u svim postojećim
-- pregledima, grafovima i analitici baterije bez ikakvih iznimaka.
CREATE TABLE IF NOT EXISTS aton_readings (
    id                      BIGSERIAL    PRIMARY KEY,
    object_id               UUID         REFERENCES objects(id) ON DELETE CASCADE,
    station_id              VARCHAR(50)  NOT NULL,
    recorded_at             TIMESTAMPTZ  NOT NULL,
    received_at             TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- Temperature [°C]
    temp_trenutna_c         REAL,
    temp_0100_c             REAL,
    temp_1300_c             REAL,

    -- Trenutno stanje baterija: napon [V], struja [A]
    gl_svj_napon_v          REAL,
    gl_svj_struja_a         REAL,
    automat_napon_v         REAL,
    automat_struja_a        REAL,

    -- Dnevni prosjeci
    prosjek_napon_gl_svj_v  REAL,
    prosjek_napon_automat_v REAL,
    punjenje_gl_svj_a       REAL,
    punjenje_automat_a      REAL,
    potrosnja_gl_svj_a      REAL,   -- negativna
    potrosnja_automat_a     REAL,   -- negativna
    potrosnja_izvor_a       REAL,   -- negativna
    dnevna_potrosnja_a      REAL,   -- negativna

    -- Svih 31 sirovih registara — mapa još nije potpuna (alarm/status
    -- bitmaske), pa ih čuvamo za naknadno mapiranje bez ponovnog polla.
    regs                    JSONB        NOT NULL,

    UNIQUE (object_id, recorded_at)
);

CREATE INDEX IF NOT EXISTS idx_aton_readings_object_time
    ON aton_readings (object_id, recorded_at DESC);

-- Isti mehanizam kao za ostale tablice: object_id se razriješi iz station_id
DROP TRIGGER IF EXISTS trg_resolve_obj_aton ON aton_readings;
CREATE TRIGGER trg_resolve_obj_aton
    BEFORE INSERT ON aton_readings
    FOR EACH ROW EXECUTE FUNCTION fn_resolve_object_id();

-- Zadnje očitanje po objektu
CREATE OR REPLACE VIEW v_latest_aton_readings AS
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
    a.regs
FROM aton_readings a
ORDER BY a.object_id, a.recorded_at DESC;

-- ── v_objects: izloži kategoriju izvora i AtoN konfiguraciju ──────
-- DROP + CREATE (kao u 011) jer se dodaju nove kolone.
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
    -- Kategorija izvora + AtoN konfiguracija (bez tel. broja u API-u? — broj
    -- je vidljiv i u starom nadzoru, pa ga izlažemo kao i datalogger_url)
    o.source_kind,
    o.aton_snopsy_endpoint,
    o.aton_number,
    o.aton_addr,
    o.aton_reg_count,
    o.aton_sync_clock,
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
    -- Primarna slika
    (SELECT storage_url FROM object_images
     WHERE object_id = o.id AND is_primary = TRUE LIMIT 1) AS primary_image_url,
    (SELECT COUNT(*) FROM object_images WHERE object_id = o.id) AS image_count
FROM objects o
LEFT JOIN station_types st ON o.station_type_id = st.id
JOIN      regions        r  ON o.region_id = r.id;
