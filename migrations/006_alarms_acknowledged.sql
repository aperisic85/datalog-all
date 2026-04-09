-- Dodavanje polja za potvrdu alarma
ALTER TABLE alarms
  ADD COLUMN IF NOT EXISTS acknowledged_at  TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS acknowledged_by  TEXT;

CREATE INDEX IF NOT EXISTS idx_alarms_ack
  ON alarms (acknowledged_at)
  WHERE acknowledged_at IS NULL AND any_alarm_active = TRUE;
