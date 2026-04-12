-- ================================================================
-- Migration 008: Unique constraints za sprječavanje duplikata
-- Poller pri restartu servera ponovo dohvaća zadnjih N zapisa,
-- stoga je potrebno na bazi odbijati duplikate.
-- ================================================================

-- ── Deduplikacija measurements_10min ──────────────────────────────
DELETE FROM measurements_10min a
USING measurements_10min b
WHERE a.id > b.id
  AND a.object_id  = b.object_id
  AND a.recorded_at = b.recorded_at;

ALTER TABLE measurements_10min
    DROP CONSTRAINT IF EXISTS uq_m10_obj_time;
ALTER TABLE measurements_10min
    ADD CONSTRAINT uq_m10_obj_time UNIQUE (object_id, recorded_at);

-- ── Deduplikacija measurements_1h ────────────────────────────────
DELETE FROM measurements_1h a
USING measurements_1h b
WHERE a.id > b.id
  AND a.object_id  = b.object_id
  AND a.recorded_at = b.recorded_at;

ALTER TABLE measurements_1h
    DROP CONSTRAINT IF EXISTS uq_m1h_obj_time;
ALTER TABLE measurements_1h
    ADD CONSTRAINT uq_m1h_obj_time UNIQUE (object_id, recorded_at);

-- ── Deduplikacija measurements_24h ───────────────────────────────
DELETE FROM measurements_24h a
USING measurements_24h b
WHERE a.id > b.id
  AND a.object_id  = b.object_id
  AND a.recorded_at = b.recorded_at;

ALTER TABLE measurements_24h
    DROP CONSTRAINT IF EXISTS uq_m24_obj_time;
ALTER TABLE measurements_24h
    ADD CONSTRAINT uq_m24_obj_time UNIQUE (object_id, recorded_at);

-- ── Deduplikacija alarms ──────────────────────────────────────────
DELETE FROM alarms a
USING alarms b
WHERE a.id > b.id
  AND a.object_id  = b.object_id
  AND a.recorded_at = b.recorded_at;

ALTER TABLE alarms
    DROP CONSTRAINT IF EXISTS uq_alarms_obj_time;
ALTER TABLE alarms
    ADD CONSTRAINT uq_alarms_obj_time UNIQUE (object_id, recorded_at);

-- ── Deduplikacija event_logs ──────────────────────────────────────
-- Više logova može imati isti timestamp pa koristimo kombinaciju
-- (object_id, recorded_at, log_message) za identifikaciju duplikata.
DELETE FROM event_logs a
USING event_logs b
WHERE a.id > b.id
  AND a.object_id  = b.object_id
  AND a.recorded_at = b.recorded_at
  AND a.log_message = b.log_message;

ALTER TABLE event_logs
    DROP CONSTRAINT IF EXISTS uq_evlog_obj_time_msg;
ALTER TABLE event_logs
    ADD CONSTRAINT uq_evlog_obj_time_msg UNIQUE (object_id, recorded_at, log_message);
