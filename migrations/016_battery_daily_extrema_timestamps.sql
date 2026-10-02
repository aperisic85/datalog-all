-- Battery current extrema timestamps from Campbell Measurements_24h.
-- TMn/TMax are the exact times at which the daily minimum/maximum occurred.

ALTER TABLE measurements_24h
    ADD COLUMN IF NOT EXISTS battery_current_tmn  TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS battery_current_tmax TIMESTAMPTZ;

COMMENT ON COLUMN measurements_24h.battery_current_tmn
    IS 'Campbell Battery_current_TMn: timestamp when daily minimum current occurred';

COMMENT ON COLUMN measurements_24h.battery_current_tmax
    IS 'Campbell Battery_current_TMax: timestamp when daily maximum current occurred';
