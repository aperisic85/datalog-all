-- Energetska prognoza — cache izračuna po objektu.
-- Pozadinski scheduler periodički računa 7-dnevnu prognozu stanja baterije
-- (prognozirana insolacija × naučeni omjer punjenja − naučena potrošnja)
-- i sprema rezultat ovdje, da dashboard i jutarnji brifing ne moraju
-- zvati Open-Meteo za svaku stanicu na svaki upit.

CREATE TABLE IF NOT EXISTS energy_forecast_cache (
    object_id           UUID PRIMARY KEY REFERENCES objects(id) ON DELETE CASCADE,
    computed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- "ok" | "warning" | "critical" | "insufficient_data"
    status              TEXT        NOT NULL,
    status_label        TEXT        NOT NULL DEFAULT '',
    -- Prvi dan u prognozi kad napon padne ispod praga (NULL = ne pada)
    first_warning_date  DATE,
    first_critical_date DATE,
    -- Najniži predviđeni SOC (%) unutar horizonta prognoze
    min_soc_pct         REAL,
    -- Kompletna prognoza (dnevni niz + parametri modela) za prikaz u sučelju
    forecast            JSONB       NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_energy_forecast_status
    ON energy_forecast_cache (status)
    WHERE status IN ('warning', 'critical');
