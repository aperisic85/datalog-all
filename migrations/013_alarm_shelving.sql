-- ================================================================
-- 013_alarm_shelving.sql
-- Alarm shelving (privremeno odlaganje alarma, ISA-18.2)
--
-- Shelvani alarm se privremeno isključuje iz obavještavanja
-- (Telegram/Slack/webhook) i vizualno označava u sučelju.
-- Shelf automatski istječe (expires_at) ili se ručno ukine
-- (unshelved_at). Nakon isteka alarm se ponovo javlja ako je
-- i dalje aktivan.
--
--   alarm_type = NULL  →  shelvani su SVI alarmi objekta
--   alarm_type = ključ →  shelvan samo taj tip (npr. 'battery_voltage_low')
-- ================================================================

CREATE TABLE IF NOT EXISTS alarm_shelves (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    object_id    UUID         NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    alarm_type   VARCHAR(50),
    reason       TEXT,
    shelved_by   TEXT         NOT NULL,
    shelved_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ  NOT NULL,
    unshelved_at TIMESTAMPTZ,
    unshelved_by TEXT,
    CHECK (expires_at > shelved_at)
);

-- Brzi lookup aktivnih shelfova pri dispatchu obavijesti
CREATE INDEX IF NOT EXISTS idx_alarm_shelves_active
    ON alarm_shelves (object_id, expires_at)
    WHERE unshelved_at IS NULL;
