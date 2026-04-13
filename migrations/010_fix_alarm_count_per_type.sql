-- ================================================================
-- Migration 010: Ispravka alarm_count — broji vrste alarma, ne zapise
--
-- Bug: fn_update_alarm_cache() je postavljala alarm_count na ukupan broj
-- nepotvrđenih aktivnih ZAPISA (redova) u zadnjih 24h za objekt.
-- Primjer: uređaj šalje podatke svakih 30 minuta i ima aktivan alarm
-- "Baterija niska" → nakon 1.5h postoje 3 zapisa s istim alarmom →
-- alarm_count = 3, a badge na listi objekata prikazuje "3 alarma".
--
-- Istovremeno, lista alarma (AlarmsPage) koristi DISTINCT ON (object_id)
-- pa prikazuje JEDAN red po objektu (najnoviji). Frontend tada broji
-- aktivne VRSTE alarma u tom jednom redu (ALARM_DEFS.filter(d => value > 0)).
-- Rezultat: badge kaže "3 alarma", lista alarma prikazuje 1 unos → neslaganje.
--
-- Ispravak:
--   alarm_count sada broji koliko je različitih VRSTA alarma aktivno
--   u najnovijem nepotvrđenom aktivnom zapisu za objekt.
--   To se podudara s brojem tag-ova koje AlarmsPage prikazuje za taj objekt.
-- ================================================================

CREATE OR REPLACE FUNCTION fn_update_alarm_cache()
RETURNS TRIGGER AS $$
DECLARE
    v_count      SMALLINT;
    v_worst      SMALLINT;
    v_summary    TEXT;
    v_any_active BOOLEAN;
    v_latest     RECORD;
BEGIN
    -- Provjeri postoji li nepotvrđeni aktivni alarm u zadnjih 24h za ovaj objekt
    SELECT EXISTS (
        SELECT 1 FROM alarms
        WHERE object_id = NEW.object_id
          AND recorded_at >= NOW() - INTERVAL '24 hours'
          AND any_alarm_active = TRUE
          AND acknowledged_at IS NULL
    ) INTO v_any_active;

    IF v_any_active THEN
        -- Dohvati najnoviji nepotvrđeni aktivni alarm zapis
        SELECT * INTO v_latest
        FROM alarms
        WHERE object_id = NEW.object_id
          AND any_alarm_active = TRUE
          AND acknowledged_at IS NULL
        ORDER BY recorded_at DESC
        LIMIT 1;

        -- Broj aktivnih VRSTA alarma u tom zapisu (podudara se s AlarmTags na frontendu)
        v_count := (
            (CASE WHEN v_latest.alarm_datalogger_high_temp       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_datalogger_high_voltage    > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_datalogger_other_error     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_battery_voltage_low        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_battery_voltage_flat       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_battery_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_garmin_comm_failed         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_garmin_other_error         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_station_out_of_radius      > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_night_light_off    > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_day_light_on       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_comm_failed        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_lantern_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_modem_network_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_modem_other_error          > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_station_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_visibility_comm_failed     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_visibility_error           > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_fog_signal_off_during_fog  > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN v_latest.alarm_fog_signal_on_while_no_fog > 0 THEN 1 ELSE 0 END)
        );

        -- Najteža razina alarma iz tog zapisa
        v_worst := CASE
            WHEN v_latest.alarm_battery_voltage_flat        > 0 OR
                 v_latest.alarm_lantern_night_light_off     > 0 OR
                 v_latest.alarm_fog_signal_off_during_fog   > 0 OR
                 v_latest.alarm_station_out_of_radius       > 0 THEN 4
            WHEN v_latest.alarm_battery_voltage_low         > 0 OR
                 v_latest.alarm_garmin_comm_failed          > 0 OR
                 v_latest.alarm_lantern_comm_failed         > 0 OR
                 v_latest.alarm_modem_network_error         > 0 OR
                 v_latest.alarm_visibility_comm_failed      > 0 THEN 3
            WHEN v_latest.alarm_datalogger_high_temp        > 0 OR
                 v_latest.alarm_datalogger_high_voltage     > 0 OR
                 v_latest.alarm_lantern_day_light_on        > 0 OR
                 v_latest.alarm_fog_signal_on_while_no_fog  > 0 OR
                 v_latest.alarm_visibility_error            > 0 THEN 2
            ELSE 1
        END;
    ELSE
        v_count := 0;
        v_worst := NULL;
    END IF;

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
        alarm_worst_level  = CASE WHEN v_any_active THEN v_worst                           ELSE NULL               END,
        alarm_last_seen_at = CASE WHEN v_any_active THEN NOW()                             ELSE alarm_last_seen_at END,
        alarm_summary      = CASE WHEN v_any_active THEN COALESCE(v_summary, alarm_summary) ELSE NULL              END
    WHERE id = NEW.object_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


-- ================================================================
-- Jednokratno osvježavanje cache-a za sve objekte
-- Ispravlja trenutno netočno stanje alarm_count koje je nastalo
-- zbog gornjeg bug-a (broji zapise umjesto vrsta alarma)
-- ================================================================
UPDATE objects o SET
    alarm_active      = sub.is_active,
    alarm_count       = sub.cnt,
    alarm_worst_level = CASE WHEN sub.is_active THEN sub.worst ELSE NULL END,
    alarm_summary     = CASE WHEN sub.is_active THEN o.alarm_summary       ELSE NULL END,
    alarm_last_seen_at = CASE WHEN sub.is_active THEN o.alarm_last_seen_at ELSE NULL END
FROM (
    -- Za svaki objekt koji ima nepotvrđene aktivne alarme u zadnjih 24h,
    -- uzmi najnoviji takav zapis i iz njega broji aktivne vrste alarma
    SELECT
        latest.object_id,
        TRUE AS is_active,
        (
            (CASE WHEN latest.alarm_datalogger_high_temp       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_datalogger_high_voltage    > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_datalogger_other_error     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_battery_voltage_low        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_battery_voltage_flat       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_battery_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_garmin_comm_failed         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_garmin_other_error         > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_station_out_of_radius      > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_lantern_night_light_off    > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_lantern_day_light_on       > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_lantern_comm_failed        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_lantern_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_modem_network_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_modem_other_error          > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_station_other_error        > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_visibility_comm_failed     > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_visibility_error           > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_fog_signal_off_during_fog  > 0 THEN 1 ELSE 0 END) +
            (CASE WHEN latest.alarm_fog_signal_on_while_no_fog > 0 THEN 1 ELSE 0 END)
        )::SMALLINT AS cnt,
        CASE
            WHEN latest.alarm_battery_voltage_flat        > 0 OR
                 latest.alarm_lantern_night_light_off     > 0 OR
                 latest.alarm_fog_signal_off_during_fog   > 0 OR
                 latest.alarm_station_out_of_radius       > 0 THEN 4
            WHEN latest.alarm_battery_voltage_low         > 0 OR
                 latest.alarm_garmin_comm_failed          > 0 OR
                 latest.alarm_lantern_comm_failed         > 0 OR
                 latest.alarm_modem_network_error         > 0 OR
                 latest.alarm_visibility_comm_failed      > 0 THEN 3
            WHEN latest.alarm_datalogger_high_temp        > 0 OR
                 latest.alarm_datalogger_high_voltage     > 0 OR
                 latest.alarm_lantern_day_light_on        > 0 OR
                 latest.alarm_fog_signal_on_while_no_fog  > 0 OR
                 latest.alarm_visibility_error            > 0 THEN 2
            ELSE 1
        END AS worst
    FROM (
        -- Najnoviji nepotvrđeni aktivni alarm po objektu (DISTINCT ON)
        SELECT DISTINCT ON (object_id) *
        FROM alarms
        WHERE recorded_at >= NOW() - INTERVAL '24 hours'
          AND any_alarm_active = TRUE
          AND acknowledged_at IS NULL
        ORDER BY object_id, recorded_at DESC
    ) latest
) sub
WHERE o.id = sub.object_id;

-- Postavi alarm_active = FALSE za objekte koji nisu u gornjoj listi
-- ali i dalje imaju alarm_active = TRUE (stari cache)
UPDATE objects
SET alarm_active      = FALSE,
    alarm_count       = 0,
    alarm_worst_level = NULL,
    alarm_summary     = NULL,
    alarm_last_seen_at = NULL
WHERE alarm_active = TRUE
  AND id NOT IN (
      SELECT DISTINCT object_id
      FROM alarms
      WHERE recorded_at >= NOW() - INTERVAL '24 hours'
        AND any_alarm_active = TRUE
        AND acknowledged_at IS NULL
  );
