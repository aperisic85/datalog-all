-- Migration 001: API keys (push authentication)
-- Sve ostale tablice su u migraciji 002

CREATE TABLE IF NOT EXISTS api_keys (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    key_hash    VARCHAR(255) NOT NULL UNIQUE,
    description VARCHAR(255),
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Default API key iz config.CR300: "50rl3puELRKVTR0UhtYMt7I9"
INSERT INTO api_keys (key_hash, description)
VALUES (
    encode(sha256('50rl3puELRKVTR0UhtYMt7I9'::bytea), 'hex'),
    'Default datalogger key (from config.CR300)'
)
ON CONFLICT DO NOTHING;
