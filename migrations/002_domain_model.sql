-- ================================================================
-- Migration 002: Kompletan domenski model
-- Regije (flat) → Objekti → Mjerenja + Alarmi + Slike
-- Korisnici → Role → Pristup regijama
-- ================================================================

-- ================================================================
-- TIPOVI OBJEKATA
-- ================================================================
CREATE TABLE IF NOT EXISTS station_types (
    id      SMALLSERIAL  PRIMARY KEY,
    code    VARCHAR(30)  NOT NULL UNIQUE,
    name    VARCHAR(100) NOT NULL,
    icon    VARCHAR(50)
);
INSERT INTO station_types (code, name, icon) VALUES
    ('LIGHTHOUSE',      'Far (Svjetionik)',       'lighthouse'),
    ('LIGHT_BUOY',      'Svjetleća plutača',      'light-buoy'),
    ('BUOY',            'Plutača',                'buoy'),
    ('LIGHT_BEACON',    'Svjetleći marker',       'light-beacon'),
    ('BEACON',          'Marker',                 'beacon'),
    ('SECTOR_LIGHT',    'Sektorsko svjetlo',      'sector-light'),
    ('LEADING_LIGHT',   'Vodilično svjetlo',      'leading-light'),
    ('WRECK_BUOY',      'Plutača na olupini',     'wreck'),
    ('WEATHER_STATION', 'Meteorološka postaja',   'weather'),
    ('OTHER',           'Ostalo',                 'other')
ON CONFLICT DO NOTHING;

-- ================================================================
-- REGIJE  (flat lista, admin kreira)
-- ================================================================
CREATE TABLE IF NOT EXISTS regions (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(150) NOT NULL UNIQUE,
    code        VARCHAR(30)  NOT NULL UNIQUE,
    description TEXT,
    color       VARCHAR(7)   NOT NULL DEFAULT '#2563eb',
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
INSERT INTO regions (name, code, color) VALUES
    ('Split',       'REG-ST', '#2563eb'),
    ('Zadar',       'REG-ZD', '#16a34a'),
    ('Šibenik',     'REG-SI', '#dc2626'),
    ('Rijeka',      'REG-RI', '#9333ea'),
    ('Dubrovnik',   'REG-DU', '#ea580c'),
    ('Ploče',       'REG-PL', '#0891b2')
ON CONFLICT DO NOTHING;

-- ================================================================
-- KORISNICI
-- ================================================================
CREATE TABLE IF NOT EXISTS users (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    username        VARCHAR(100) NOT NULL UNIQUE,
    email           VARCHAR(255) NOT NULL UNIQUE,
    password_hash   VARCHAR(255) NOT NULL,
    full_name       VARCHAR(200),
    -- admin    → puni pristup svemu
    -- operator → upravljanje na dodijeljenim regijama (SetValue, edit)
    -- viewer   → samo pregled na dodijeljenim regijama
    role            VARCHAR(20)  NOT NULL DEFAULT 'viewer'
                    CHECK (role IN ('admin', 'operator', 'viewer')),
    is_active       BOOLEAN      NOT NULL DEFAULT TRUE,
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_by      UUID         REFERENCES users(id) ON DELETE SET NULL
);
-- Defaultni admin  (lozinka: admin123 — PROMIJENI!)
INSERT INTO users (username, email, password_hash, full_name, role) VALUES
    ('admin','admin@plovput.hr',
     '$2b$12$iUbD1mrTvNe4EQpML3mFv.g2MVTbgW6u2T4hKjA6AbOn3z9fvVF1m',
     'System Administrator','admin')
ON CONFLICT DO NOTHING;

-- ================================================================
-- OBJEKTI  (navigacijski znakovi)
-- station_id mora biti identičan kao c_station_name u config.CR300
-- ================================================================
CREATE TABLE IF NOT EXISTS objects (
    id                  UUID         PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Ključna veza s datalogerom
    station_id          VARCHAR(50)  NOT NULL UNIQUE,

    name                VARCHAR(150) NOT NULL,
    short_name          VARCHAR(50),
    region_id           UUID         NOT NULL REFERENCES regions(id) ON DELETE RESTRICT,
    station_type_id     SMALLINT     REFERENCES station_types(id),

    -- GPS
    latitude            DOUBLE PRECISION,
    longitude           DOUBLE PRECISION,
    location_name       VARCHAR(200),

    -- Opis
    description         TEXT,
    notes               TEXT,

    -- Datalogger konekcija (pull mode poller)
    datalogger_url      VARCHAR(500),
    datalogger_user     VARCHAR(100),
    datalogger_pass     VARCHAR(200),
    poll_interval_sec   INTEGER      NOT NULL DEFAULT 60,
    polling_enabled     BOOLEAN      NOT NULL DEFAULT FALSE,

    -- Alarm cache — ažurira se automatski iz push/pull handlera
    -- Eliminira potrebu za JOIN na alarms pri svakom prikazu liste
    alarm_active        BOOLEAN      NOT NULL DEFAULT FALSE,
    alarm_count         SMALLINT     NOT NULL DEFAULT 0,
    alarm_worst_level   SMALLINT,              -- NULL=ok 2=warn 3=error 4=fatal
    alarm_last_seen_at  TIMESTAMPTZ,
    alarm_summary       TEXT,                  -- kratak opis zadnjeg aktivnog alarma

    is_active           BOOLEAN      NOT NULL DEFAULT TRUE,
    commissioned_at     DATE,
    decommissioned_at   DATE,
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_by          UUID         REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX idx_obj_region    ON objects (region_id);
CREATE INDEX idx_obj_sid       ON objects (station_id);
CREATE INDEX idx_obj_active    ON objects (is_active);
CREATE INDEX idx_obj_alarm     ON objects (alarm_active) WHERE alarm_active = TRUE;
CREATE EXTENSION IF NOT EXISTS cube;
CREATE EXTENSION IF NOT EXISTS earthdistance;
CREATE INDEX idx_obj_geo ON objects USING GIST (ll_to_earth(latitude, longitude))
    WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- ================================================================
-- SLIKE OBJEKATA
-- ================================================================
CREATE TABLE IF NOT EXISTS object_images (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    object_id       UUID         NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    filename        VARCHAR(255) NOT NULL,
    original_name   VARCHAR(255),
    mime_type       VARCHAR(50)  NOT NULL DEFAULT 'image/jpeg',
    file_size_bytes INTEGER,
    storage_path    VARCHAR(500) NOT NULL,
    storage_url     VARCHAR(500),
    is_primary      BOOLEAN      NOT NULL DEFAULT FALSE,
    caption         VARCHAR(300),
    taken_at        DATE,
    uploaded_by     UUID         REFERENCES users(id) ON DELETE SET NULL,
    uploaded_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_img_obj ON object_images (object_id);
CREATE UNIQUE INDEX idx_img_one_primary ON object_images (object_id) WHERE is_primary = TRUE;

-- ================================================================
-- PRISTUP KORISNIKA REGIJAMA
-- Admin dodjeljuje korisniku pristup pojedinoj regiji
-- ================================================================
CREATE TABLE IF NOT EXISTS user_region_access (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    region_id   UUID        NOT NULL REFERENCES regions(id) ON DELETE CASCADE,
    permission  VARCHAR(20) NOT NULL DEFAULT 'viewer'
                CHECK (permission IN ('operator', 'viewer')),
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    granted_by  UUID        REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE (user_id, region_id)
);
CREATE INDEX idx_ura_user   ON user_region_access (user_id);
CREATE INDEX idx_ura_region ON user_region_access (region_id);

-- ================================================================
-- JWT REFRESH TOKENI
-- ================================================================
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  VARCHAR(255) NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ  NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    ip_address  INET,
    user_agent  TEXT
);
CREATE INDEX idx_rt_hash   ON refresh_tokens (token_hash);
CREATE INDEX idx_rt_user   ON refresh_tokens (user_id);
CREATE INDEX idx_rt_expiry ON refresh_tokens (expires_at);

-- ================================================================
-- AUDIT LOG
-- ================================================================
CREATE TABLE IF NOT EXISTS audit_log (
    id          BIGSERIAL    PRIMARY KEY,
    user_id     UUID         REFERENCES users(id) ON DELETE SET NULL,
    username    VARCHAR(100),
    action      VARCHAR(100) NOT NULL,
    entity_type VARCHAR(50),
    entity_id   TEXT,
    details     JSONB,
    ip_address  INET,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_audit_created ON audit_log (created_at DESC);
CREATE INDEX idx_audit_entity  ON audit_log (entity_type, entity_id);

-- ================================================================
-- MJERENJA 10 MINUTA
-- Sve vrijednosti iz DataTable(Measurements_10min) u main.CR300
-- ================================================================
CREATE TABLE IF NOT EXISTS measurements_10min (
    id              BIGSERIAL    PRIMARY KEY,
    object_id       UUID         NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    station_id      VARCHAR(50)  NOT NULL,
    recorded_at     TIMESTAMPTZ  NOT NULL,
    received_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- DATALOGGER
    datalogger_temp_avg         REAL,           -- °C  Datalogger_temperature Avg

    -- BATERIJA
    battery_voltage_avg         REAL,           -- V   Battery_voltage Avg
    battery_current_avg         REAL,           -- A   Battery_current Avg
    battery_status_smp          SMALLINT,       --     Battery_status Sample (1=FLAT 2=LOW 3=OK)
    battery_status_avg          REAL,           --     Battery_status Avg

    -- SOLARNI PANEL
    solar_voltage_avg           REAL,           -- V   Solar_panel_voltage Avg
    solar_daylight_smp          SMALLINT,       --     Solar_panel_day_light Sample (0/1)
    solar_daylight_avg          REAL,           --     Solar_panel_day_light Avg

    -- MODEM
    modem_power_avg             REAL,           --     Modem_power_state Avg
    internet_ok_avg             REAL,           --     Internet_connection_ok Avg

    -- GARMIN GPS
    garmin_comm_ok_avg          REAL,           --     Garmin_communication_ok Avg
    garmin_satellites_avg       REAL,           --     Garmin_number_of_sattelites Avg
    garmin_latitude_avg         DOUBLE PRECISION, --   Garmin_latitude Avg
    garmin_longitude_avg        DOUBLE PRECISION, --   Garmin_longitude Avg
    garmin_distance_avg         REAL,           -- m   Garmin_distance Avg

    -- FENJER (zajednička polja za SL i MBL160)
    lantern_comm_ok_avg         REAL,           --     Lantern_communication_ok Avg
    lantern_light_active_avg    REAL,           --     Lantern_light_active Avg
    lantern_current_avg         REAL,           -- A   Lantern_current Avg
    lantern_latitude_avg        DOUBLE PRECISION, --   Lantern_latitude Avg
    lantern_longitude_avg       DOUBLE PRECISION, --   Lantern_longitude Avg
    lantern_distance_avg        REAL            -- m   Lantern_distance Avg
);
CREATE INDEX idx_m10_obj_time ON measurements_10min (object_id, recorded_at DESC);
CREATE INDEX idx_m10_sid_time ON measurements_10min (station_id, recorded_at DESC);

-- ================================================================
-- MJERENJA 1 SAT
-- DataTable(Measurements_1h)
-- ================================================================
CREATE TABLE IF NOT EXISTS measurements_1h (
    id              BIGSERIAL    PRIMARY KEY,
    object_id       UUID         NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    station_id      VARCHAR(50)  NOT NULL,
    recorded_at     TIMESTAMPTZ  NOT NULL,
    received_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- DATALOGGER
    datalogger_temp_avg         REAL,           -- °C

    -- BATERIJA
    battery_voltage_avg         REAL,           -- V
    battery_current_avg         REAL,           -- A
    battery_charge_tot          REAL,           -- Ah  Battery_charge Totalize
    battery_discharge_tot       REAL,           -- Ah  Battery_discharge Totalize
    battery_status_avg          REAL,           --     1=FLAT 2=LOW 3=OK

    -- SOLARNI PANEL
    solar_voltage_avg           REAL,           -- V
    solar_daylight_avg          REAL,           --     0..1

    -- MODEM
    modem_power_avg             REAL,           --     0..1

    -- FENJER
    lantern_light_active_avg    REAL,           --     0..1
    lantern_current_avg         REAL            -- A
);
CREATE INDEX idx_m1h_obj_time ON measurements_1h (object_id, recorded_at DESC);
CREATE INDEX idx_m1h_sid_time ON measurements_1h (station_id, recorded_at DESC);

-- ================================================================
-- MJERENJA 24 SATA
-- DataTable(Measurements_24h)
-- ================================================================
CREATE TABLE IF NOT EXISTS measurements_24h (
    id              BIGSERIAL    PRIMARY KEY,
    object_id       UUID         NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    station_id      VARCHAR(50)  NOT NULL,
    recorded_at     TIMESTAMPTZ  NOT NULL,
    received_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- DATALOGGER
    datalogger_temp_avg         REAL,           -- °C

    -- BATERIJA
    battery_voltage_avg         REAL,           -- V
    battery_current_avg         REAL,           -- A
    battery_current_min         REAL,           -- A   Minimum
    battery_current_max         REAL,           -- A   Maximum
    battery_charge_tot          REAL,           -- Ah  Totalize
    battery_discharge_tot       REAL,           -- Ah  Totalize
    battery_status_avg          REAL,           --     1..3

    -- SOLARNI PANEL
    solar_daylight_avg          REAL,           --     0..1

    -- MODEM
    modem_power_avg             REAL,           --     0..1

    -- FENJER
    lantern_light_active_avg    REAL,           --     0..1
    lantern_current_avg         REAL            -- A
);
CREATE INDEX idx_m24_obj_time ON measurements_24h (object_id, recorded_at DESC);
CREATE INDEX idx_m24_sid_time ON measurements_24h (station_id, recorded_at DESC);

-- ================================================================
-- ALARMI  (10 minutni snapshot)
-- DataTable(Alarms_10min) — sva polja iz main.CR300 r.314-336
-- ================================================================
CREATE TABLE IF NOT EXISTS alarms (
    id              BIGSERIAL    PRIMARY KEY,
    object_id       UUID         NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    station_id      VARCHAR(50)  NOT NULL,
    recorded_at     TIMESTAMPTZ  NOT NULL,
    received_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- DATALOGGER alarmi
    -- Datalogger temperatura previsoka (>60°C warn, >70°C error, >80°C fatal)
    alarm_datalogger_high_temp      SMALLINT NOT NULL DEFAULT 0,
    -- Datalogger napon previsok (>16V warn, >18V fatal)
    alarm_datalogger_high_voltage   SMALLINT NOT NULL DEFAULT 0,
    -- Ostale datalogger greške
    alarm_datalogger_other_error    SMALLINT NOT NULL DEFAULT 0,

    -- BATERIJA alarmi
    -- Napon baterije nizak (status=2, avg < 11.5V)
    alarm_battery_voltage_low       SMALLINT NOT NULL DEFAULT 0,
    -- Baterija prazna (status=1, avg < 10.5V)
    alarm_battery_voltage_flat      SMALLINT NOT NULL DEFAULT 0,
    -- Ostale greške baterije
    alarm_battery_other_error       SMALLINT NOT NULL DEFAULT 0,

    -- GARMIN GPS alarmi
    -- Komunikacija s Garmin GPS modulom neuspješna
    alarm_garmin_comm_failed        SMALLINT NOT NULL DEFAULT 0,
    -- Garmin ostale greške (broj satelita < 3)
    alarm_garmin_other_error        SMALLINT NOT NULL DEFAULT 0,

    -- STANICA alarmi
    -- GPS koordinate izvan dozvoljenog radijusa (c_maximum_radius_m = 50m)
    alarm_station_out_of_radius     SMALLINT NOT NULL DEFAULT 0,

    -- FENJER alarmi
    -- Fenjer ne svijetli noću (solar=dark AND light=off)
    alarm_lantern_night_light_off   SMALLINT NOT NULL DEFAULT 0,
    -- Fenjer svijetli danju (solar=daylight AND light=on)
    alarm_lantern_day_light_on      SMALLINT NOT NULL DEFAULT 0,
    -- Komunikacija s fenjером neuspješna (Modbus)
    alarm_lantern_comm_failed       SMALLINT NOT NULL DEFAULT 0,
    -- Fenjer ostale greške (struja van raspona c_lantern_ON_min/max_avg_current)
    alarm_lantern_other_error       SMALLINT NOT NULL DEFAULT 0,

    -- MODEM alarmi
    -- Modem nema mrežne veze (PingIP fail)
    alarm_modem_network_error       SMALLINT NOT NULL DEFAULT 0,
    -- Modem ostale greške
    alarm_modem_other_error         SMALLINT NOT NULL DEFAULT 0,

    -- OSTALO
    alarm_station_other_error       SMALLINT NOT NULL DEFAULT 0,

    -- Izvedeno: je li ijedan alarm aktivan u ovom zapisu
    any_alarm_active BOOLEAN NOT NULL GENERATED ALWAYS AS (
        alarm_datalogger_high_temp    > 0 OR
        alarm_datalogger_high_voltage > 0 OR
        alarm_datalogger_other_error  > 0 OR
        alarm_battery_voltage_low     > 0 OR
        alarm_battery_voltage_flat    > 0 OR
        alarm_battery_other_error     > 0 OR
        alarm_garmin_comm_failed      > 0 OR
        alarm_garmin_other_error      > 0 OR
        alarm_station_out_of_radius   > 0 OR
        alarm_lantern_night_light_off > 0 OR
        alarm_lantern_day_light_on    > 0 OR
        alarm_lantern_comm_failed     > 0 OR
        alarm_lantern_other_error     > 0 OR
        alarm_modem_network_error     > 0 OR
        alarm_modem_other_error       > 0 OR
        alarm_station_other_error     > 0
    ) STORED
);
CREATE INDEX idx_alarms_obj_time    ON alarms (object_id, recorded_at DESC);
CREATE INDEX idx_alarms_sid_time    ON alarms (station_id, recorded_at DESC);
CREATE INDEX idx_alarms_active      ON alarms (object_id, any_alarm_active) WHERE any_alarm_active = TRUE;

-- ================================================================
-- EVENT LOG  (DataTable Event_log)
-- ================================================================
CREATE TABLE IF NOT EXISTS event_logs (
    id              BIGSERIAL    PRIMARY KEY,
    object_id       UUID         REFERENCES objects(id) ON DELETE CASCADE,
    station_id      VARCHAR(50)  NOT NULL,
    recorded_at     TIMESTAMPTZ  NOT NULL,
    received_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    -- 1=INFO  2=WARN  3=ERROR  4=FATAL
    log_level       SMALLINT     NOT NULL CHECK (log_level BETWEEN 1 AND 4),
    log_message     TEXT         NOT NULL
);
CREATE INDEX idx_evlog_obj_time  ON event_logs (object_id, recorded_at DESC);
CREATE INDEX idx_evlog_level     ON event_logs (log_level, recorded_at DESC);

-- ================================================================
-- UPDATED_AT trigger
-- ================================================================
CREATE OR REPLACE FUNCTION fn_update_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_regions_upd BEFORE UPDATE ON regions
    FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER trg_objects_upd BEFORE UPDATE ON objects
    FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();
CREATE TRIGGER trg_users_upd   BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION fn_update_updated_at();

-- ================================================================
-- TRIGGER: automatski osvježi alarm cache na objects tablici
-- Poziva se nakon INSERT u alarms tablicu
-- ================================================================
CREATE OR REPLACE FUNCTION fn_update_alarm_cache()
RETURNS TRIGGER AS $$
DECLARE
    v_count       SMALLINT;
    v_worst       SMALLINT;
    v_summary     TEXT;
    v_any_active  BOOLEAN;
BEGIN
    -- Izbroji aktivne alarme u zadnjem zapisu
    v_any_active := NEW.any_alarm_active;

    -- Pronađi najgori nivel u zadnjih 24h
    SELECT
        COUNT(*) FILTER (WHERE any_alarm_active),
        MAX(
            CASE
                WHEN alarm_battery_voltage_flat > 0 OR
                     alarm_lantern_night_light_off > 0 OR
                     alarm_station_out_of_radius > 0  THEN 4
                WHEN alarm_battery_voltage_low > 0 OR
                     alarm_garmin_comm_failed > 0 OR
                     alarm_lantern_comm_failed > 0 OR
                     alarm_modem_network_error > 0   THEN 3
                WHEN alarm_datalogger_high_temp > 0 OR
                     alarm_datalogger_high_voltage > 0 THEN 2
                ELSE 1
            END
        )
    INTO v_count, v_worst
    FROM alarms
    WHERE object_id = NEW.object_id
      AND recorded_at >= NOW() - INTERVAL '24 hours'
      AND any_alarm_active = TRUE;

    -- Kratki opis zadnjeg alarma
    IF NEW.any_alarm_active THEN
        SELECT string_agg(alarm_name, ', ') INTO v_summary FROM (
            SELECT unnest(ARRAY[
                CASE WHEN NEW.alarm_battery_voltage_flat    > 0 THEN 'Baterija prazna'   END,
                CASE WHEN NEW.alarm_battery_voltage_low     > 0 THEN 'Baterija niska'    END,
                CASE WHEN NEW.alarm_lantern_night_light_off > 0 THEN 'Fenjer ugašen noću' END,
                CASE WHEN NEW.alarm_lantern_day_light_on    > 0 THEN 'Fenjer upaljen danju' END,
                CASE WHEN NEW.alarm_lantern_comm_failed     > 0 THEN 'Fenjer: greška veze' END,
                CASE WHEN NEW.alarm_garmin_comm_failed      > 0 THEN 'GPS: greška veze'  END,
                CASE WHEN NEW.alarm_station_out_of_radius   > 0 THEN 'Van radijusa'      END,
                CASE WHEN NEW.alarm_modem_network_error     > 0 THEN 'Modem: nema mreže' END,
                CASE WHEN NEW.alarm_datalogger_high_temp    > 0 THEN 'Visoka temp.'      END,
                CASE WHEN NEW.alarm_datalogger_high_voltage > 0 THEN 'Visoki napon'      END
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

CREATE TRIGGER trg_alarm_cache_update
    AFTER INSERT ON alarms
    FOR EACH ROW EXECUTE FUNCTION fn_update_alarm_cache();

-- ================================================================
-- TRIGGER: poveži mjerenja s object_id po station_id
-- Automatski popunjava object_id iz station_id pri insertu
-- ================================================================
CREATE OR REPLACE FUNCTION fn_resolve_object_id()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.object_id IS NULL THEN
        SELECT id INTO NEW.object_id FROM objects WHERE station_id = NEW.station_id;
        IF NOT FOUND THEN
            RAISE WARNING 'No object found for station_id=%', NEW.station_id;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_resolve_obj_m10
    BEFORE INSERT ON measurements_10min
    FOR EACH ROW EXECUTE FUNCTION fn_resolve_object_id();
CREATE TRIGGER trg_resolve_obj_m1h
    BEFORE INSERT ON measurements_1h
    FOR EACH ROW EXECUTE FUNCTION fn_resolve_object_id();
CREATE TRIGGER trg_resolve_obj_m24
    BEFORE INSERT ON measurements_24h
    FOR EACH ROW EXECUTE FUNCTION fn_resolve_object_id();
CREATE TRIGGER trg_resolve_obj_alarms
    BEFORE INSERT ON alarms
    FOR EACH ROW EXECUTE FUNCTION fn_resolve_object_id();
CREATE TRIGGER trg_resolve_obj_evlog
    BEFORE INSERT ON event_logs
    FOR EACH ROW EXECUTE FUNCTION fn_resolve_object_id();

-- ================================================================
-- VIEWS
-- ================================================================

-- Puni prikaz objekta
CREATE OR REPLACE VIEW v_objects AS
SELECT
    o.id,
    o.station_id,
    o.name,
    o.short_name,
    o.latitude,
    o.longitude,
    o.location_name,
    o.description,
    o.notes,
    o.is_active,
    o.polling_enabled,
    o.datalogger_url,
    o.poll_interval_sec,
    o.commissioned_at,
    -- Alarm cache
    o.alarm_active,
    o.alarm_count,
    o.alarm_worst_level,
    o.alarm_last_seen_at,
    o.alarm_summary,
    -- Tip
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

-- Zadnje mjerenje po objektu (za dashboard live status)
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
    m.lantern_distance_avg
FROM measurements_10min m
ORDER BY m.object_id, m.recorded_at DESC;

-- Zadnji alarm po objektu
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
    a.alarm_station_other_error
FROM alarms a
ORDER BY a.object_id, a.recorded_at DESC;

-- Dashboard summary po regiji
CREATE OR REPLACE VIEW v_region_summary AS
SELECT
    r.id    AS region_id,
    r.name  AS region_name,
    r.code  AS region_code,
    r.color AS region_color,
    COUNT(o.id)                                            AS total_objects,
    COUNT(o.id)  FILTER (WHERE o.is_active)               AS active_objects,
    COUNT(o.id)  FILTER (WHERE o.alarm_active)            AS objects_in_alarm,
    MAX(o.alarm_worst_level)                              AS worst_alarm_level,
    -- Statistika baterije u zadnjih sat vremena
    ROUND(AVG(lm.battery_voltage_avg)::numeric, 2)       AS avg_battery_voltage,
    COUNT(lm.object_id) FILTER (
        WHERE lm.battery_status_smp = 1)                  AS battery_flat_count,
    COUNT(lm.object_id) FILTER (
        WHERE lm.battery_status_smp = 2)                  AS battery_low_count,
    COUNT(lm.object_id) FILTER (
        WHERE lm.lantern_light_active_avg > 0.5)          AS lanterns_on_count
FROM regions r
LEFT JOIN objects           o  ON o.region_id = r.id
LEFT JOIN v_latest_measurements lm ON lm.object_id = o.id
GROUP BY r.id, r.name, r.code, r.color;
