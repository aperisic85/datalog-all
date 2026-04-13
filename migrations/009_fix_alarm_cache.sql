-- ================================================================
-- Migration 009: Ispravka alarm cache okidača
--
-- Bug: fn_update_alarm_cache() je postavljala alarm_active na objektu
-- na temelju NEW.any_alarm_active (samo tek umetnutog retka), a ne
-- na temelju ukupnog stanja NEPOTVRĐENIH aktivnih alarma za objekt.
--
-- Posljedica: kada bi stigao novi alarm zapis s any_alarm_active=FALSE
-- (sva polja alarma = 0, npr. redoviti statusni update bez aktivnih
-- alarma), okidač bi postavio alarm_active=FALSE na objektu — čak i
-- ako postoje stariji nepotvrđeni aktivni alarmi u alarms tablici.
-- Rezultat: tab "Alarmi" prikazuje aktivne alarme, a lista objekata
-- prikazuje "OK" za iste objekte.
--
-- Ispravak:
--   1. SELECT za v_count i v_worst filtrira po acknowledged_at IS NULL
--      (broji samo nepotvrđene aktivne alarme)
--   2. v_any_active se izračunava iz v_count (> 0), a ne iz NEW retka
--   3. alarm_summary se zadržava ako novi zapis nema aktivnih alarma,
--      ali objekt još uvijek ima nepotvrđenih aktivnih alarma
-- ================================================================

CREATE OR REPLACE FUNCTION fn_update_alarm_cache()
RETURNS TRIGGER AS $$
DECLARE
    v_count      SMALLINT;
    v_worst      SMALLINT;
    v_summary    TEXT;
    v_any_active BOOLEAN;
BEGIN
    -- Izbroji SVE nepotvrđene aktivne alarme u zadnjih 24h za ovaj objekt
    -- (nije samo NEW red — uključuje i starije nepotvrđene alarme)
    SELECT
        COUNT(*) FILTER (WHERE any_alarm_active AND acknowledged_at IS NULL),
        MAX(
            CASE
                -- FATAL (razina 4)
                WHEN alarm_battery_voltage_flat           > 0 OR
                     alarm_lantern_night_light_off         > 0 OR
                     alarm_fog_signal_off_during_fog       > 0 OR
                     alarm_station_out_of_radius           > 0  THEN 4
                -- ERROR (razina 3)
                WHEN alarm_battery_voltage_low             > 0 OR
                     alarm_garmin_comm_failed              > 0 OR
                     alarm_lantern_comm_failed             > 0 OR
                     alarm_modem_network_error             > 0 OR
                     alarm_visibility_comm_failed          > 0  THEN 3
                -- WARN (razina 2)
                WHEN alarm_datalogger_high_temp            > 0 OR
                     alarm_datalogger_high_voltage         > 0 OR
                     alarm_lantern_day_light_on            > 0 OR
                     alarm_fog_signal_on_while_no_fog      > 0 OR
                     alarm_visibility_error                > 0  THEN 2
                ELSE 1
            END
        )
    INTO v_count, v_worst
    FROM alarms
    WHERE object_id = NEW.object_id
      AND recorded_at >= NOW() - INTERVAL '24 hours'
      AND any_alarm_active = TRUE
      AND acknowledged_at IS NULL;

    -- v_any_active je TRUE samo ako postoji barem jedan nepotvrđen aktivni alarm
    v_any_active := COALESCE(v_count, 0) > 0;

    -- Kratki opis: koristimo polja novog retka ako sam ima aktivne alarme.
    -- Ako novi red nema aktivnih alarma ali objekt još uvijek ima
    -- nepotvrđenih alarma, v_summary ostaje NULL i čuvamo postojeći opis.
    IF NEW.any_alarm_active THEN
        SELECT string_agg(alarm_name, ', ') INTO v_summary FROM (
            SELECT unnest(ARRAY[
                CASE WHEN NEW.alarm_battery_voltage_flat          > 0 THEN 'Baterija prazna'                END,
                CASE WHEN NEW.alarm_battery_voltage_low           > 0 THEN 'Baterija niska'                 END,
                CASE WHEN NEW.alarm_lantern_night_light_off       > 0 THEN 'Fenjer ugašen noću'             END,
                CASE WHEN NEW.alarm_lantern_day_light_on          > 0 THEN 'Fenjer upaljen danju'           END,
                CASE WHEN NEW.alarm_lantern_comm_failed           > 0 THEN 'Fenjer: greška veze'            END,
                CASE WHEN NEW.alarm_garmin_comm_failed            > 0 THEN 'GPS: greška veze'               END,
                CASE WHEN NEW.alarm_station_out_of_radius         > 0 THEN 'Van radijusa'                   END,
                CASE WHEN NEW.alarm_modem_network_error           > 0 THEN 'Modem: nema mreže'              END,
                CASE WHEN NEW.alarm_datalogger_high_temp          > 0 THEN 'Visoka temperatura'             END,
                CASE WHEN NEW.alarm_datalogger_high_voltage       > 0 THEN 'Visoki napon'                   END,
                CASE WHEN NEW.alarm_visibility_comm_failed        > 0 THEN 'Vidljivost: greška veze'        END,
                CASE WHEN NEW.alarm_visibility_error              > 0 THEN 'Vidljivost: greška senzora'     END,
                CASE WHEN NEW.alarm_fog_signal_off_during_fog     > 0 THEN 'Maglenka: nije aktivna u magli' END,
                CASE WHEN NEW.alarm_fog_signal_on_while_no_fog    > 0 THEN 'Maglenka: aktivna bez magle'    END
            ]) AS alarm_name
        ) t WHERE alarm_name IS NOT NULL;
    END IF;

    UPDATE objects SET
        alarm_active       = v_any_active,
        alarm_count        = COALESCE(v_count, 0),
        alarm_worst_level  = CASE WHEN v_any_active THEN v_worst           ELSE NULL          END,
        alarm_last_seen_at = CASE WHEN v_any_active THEN NOW()             ELSE alarm_last_seen_at END,
        -- Ako v_summary je NULL (novi red bez alarma) ali još ima aktivnih alarma,
        -- zadržavamo postojeći alarm_summary iz tablice (COALESCE)
        alarm_summary      = CASE WHEN v_any_active THEN COALESCE(v_summary, alarm_summary) ELSE NULL END
    WHERE id = NEW.object_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


-- ================================================================
-- Jednokratno osvježavanje cache-a za sve objekte
-- Ispravlja trenutno netočno stanje alarm_active koje je nastalo
-- zbog gornjeg bug-a
-- ================================================================
UPDATE objects o SET
    alarm_active = sub.is_active,
    alarm_count  = sub.cnt,
    alarm_worst_level = CASE WHEN sub.is_active THEN sub.worst ELSE NULL END,
    alarm_summary     = CASE WHEN sub.is_active THEN alarm_summary       ELSE NULL END,
    alarm_last_seen_at = CASE WHEN sub.is_active THEN alarm_last_seen_at ELSE NULL END
FROM (
    SELECT
        object_id,
        (COUNT(*) FILTER (WHERE any_alarm_active AND acknowledged_at IS NULL) > 0) AS is_active,
        COUNT(*) FILTER (WHERE any_alarm_active AND acknowledged_at IS NULL)        AS cnt,
        MAX(CASE
            WHEN alarm_battery_voltage_flat       > 0 OR
                 alarm_lantern_night_light_off     > 0 OR
                 alarm_fog_signal_off_during_fog   > 0 OR
                 alarm_station_out_of_radius       > 0 THEN 4
            WHEN alarm_battery_voltage_low         > 0 OR
                 alarm_garmin_comm_failed          > 0 OR
                 alarm_lantern_comm_failed         > 0 OR
                 alarm_modem_network_error         > 0 OR
                 alarm_visibility_comm_failed      > 0 THEN 3
            WHEN alarm_datalogger_high_temp        > 0 OR
                 alarm_datalogger_high_voltage     > 0 OR
                 alarm_lantern_day_light_on        > 0 OR
                 alarm_fog_signal_on_while_no_fog  > 0 OR
                 alarm_visibility_error            > 0 THEN 2
            ELSE 1
        END) AS worst
    FROM alarms
    WHERE recorded_at >= NOW() - INTERVAL '24 hours'
    GROUP BY object_id
) sub
WHERE o.id = sub.object_id;
