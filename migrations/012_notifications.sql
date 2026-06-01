-- ================================================================
-- 012_notifications.sql
-- Sustav obavještavanja o alarmima
--   • notification_channels — kanali isporuke (Telegram / Webhook / Slack)
--   • notification_rules    — pravila: kada i kome slati
--   • notification_state    — stanje po (objekt, tip alarma) za detekciju
--                              prijelaza (novi alarm / riješen) i deduplikaciju
--   • notification_log      — povijest poslanih obavijesti
-- ================================================================

-- ── Kanali isporuke ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS notification_channels (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(100) NOT NULL,
    kind        VARCHAR(20)  NOT NULL CHECK (kind IN ('telegram', 'webhook', 'slack')),
    -- Konfiguracija ovisna o vrsti kanala:
    --   telegram → { "bot_token": "...", "chat_id": "..." }
    --   webhook  → { "url": "https://..." }
    --   slack    → { "url": "https://hooks.slack.com/..." }
    config      JSONB        NOT NULL DEFAULT '{}'::jsonb,
    enabled     BOOLEAN      NOT NULL DEFAULT TRUE,
    created_by  UUID         REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ── Pravila obavještavanja ───────────────────────────────────────
CREATE TABLE IF NOT EXISTS notification_rules (
    id                UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name              VARCHAR(100) NOT NULL,
    channel_id        UUID         NOT NULL REFERENCES notification_channels(id) ON DELETE CASCADE,
    -- NULL = primjenjuje se na sve regije
    region_id         UUID         REFERENCES regions(id) ON DELETE CASCADE,
    -- Minimalna ozbiljnost za slanje (1=INFO, 2=WARN, 3=ERROR, 4=FATAL)
    min_severity      SMALLINT     NOT NULL DEFAULT 3 CHECK (min_severity BETWEEN 1 AND 4),
    -- Šalji li obavijest i kad se alarm riješi (npr. "svjetlo opet radi")
    notify_on_clear   BOOLEAN      NOT NULL DEFAULT TRUE,
    -- Tihi sati (UTC, 0–23). Suspendiraju samo ne-kritične (<4) obavijesti.
    quiet_hours_start SMALLINT     CHECK (quiet_hours_start BETWEEN 0 AND 23),
    quiet_hours_end   SMALLINT     CHECK (quiet_hours_end   BETWEEN 0 AND 23),
    -- Ponovno javljanje za i dalje aktivan alarm (minute)
    cooldown_minutes  INTEGER      NOT NULL DEFAULT 360,
    enabled           BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notif_rules_region  ON notification_rules (region_id);
CREATE INDEX IF NOT EXISTS idx_notif_rules_channel ON notification_rules (channel_id);

-- ── Stanje po (objekt, tip alarma) za detekciju prijelaza ────────
CREATE TABLE IF NOT EXISTS notification_state (
    object_id         UUID        NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    alarm_type        VARCHAR(50) NOT NULL,
    active            BOOLEAN     NOT NULL DEFAULT FALSE,
    since             TIMESTAMPTZ,
    last_notified_at  TIMESTAMPTZ,
    PRIMARY KEY (object_id, alarm_type)
);

-- ── Povijest poslanih obavijesti ─────────────────────────────────
CREATE TABLE IF NOT EXISTS notification_log (
    id           BIGSERIAL    PRIMARY KEY,
    channel_id   UUID         REFERENCES notification_channels(id) ON DELETE SET NULL,
    channel_name VARCHAR(100),
    object_id    UUID         REFERENCES objects(id) ON DELETE SET NULL,
    object_name  VARCHAR(150),
    alarm_type   VARCHAR(50),
    severity     SMALLINT,
    -- 'raised' = alarm nastao, 'cleared' = alarm riješen, 'test' = probna poruka
    event        VARCHAR(20)  NOT NULL DEFAULT 'raised',
    status       VARCHAR(20)  NOT NULL,            -- 'sent' | 'failed'
    error        TEXT,
    message      TEXT,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notif_log_created ON notification_log (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notif_log_object  ON notification_log (object_id, created_at DESC);
