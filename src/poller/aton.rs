//! Poller za kategoriju izvora `aton_csd`.
//!
//! AtoN stanice (pomorske oznake) nemaju HTTP sučelje. Backend se preko TCP-a
//! spaja na `snopsy_r` proxy, sam postaje Modbus master — digne CSD poziv,
//! (opcionalno) sinkronizira sat RTU-a, pročita holding registre, dekodira
//! odgovor i **uvijek** spusti poziv. Protokol je u crateu [`aton_decode`].
//!
//! Ključno ograničenje: **jedan CSD poziv po modemu u isto vrijeme**. Jedan
//! `snopsy_r` endpoint = jedan modem = jedna linija, pa se objekti koji dijele
//! endpoint prozivaju serijski (mutex po endpointu). Objekti na različitim
//! endpointima idu paralelno.

use std::collections::HashMap;
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use aton_decode::{read_object, Aton, Category, ClockSet, DobaDana, ObjectConfig, SessionError};
use chrono::{Datelike, Timelike, Utc};
use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::db::aton as db_aton;
use crate::db::domain as db;
use crate::models::aton::{alarm_insert_from_aton, AtonPollConfig, AtonReadingInsert};
use crate::models::domain::Measurement10minInsert;

use super::SharedPollerStatus;

/// Read-timeout na TCP vezi prema snopsy_r-u. Kratak namjerno: driver sam
/// prati rokove (`connect_timeout` / `response_timeout`) i mora se moći
/// probuditi između pokušaja čitanja.
const LINK_READ_TIMEOUT: Duration = Duration::from_millis(200);

/// Donja granica intervala prozivanja. CSD poziv traje ~10-20 s i troši
/// minute na SIM-u — prečesto prozivanje nema smisla.
const MIN_POLL_INTERVAL_SEC: u64 = 300;

/// Koliko čekati TCP connect prema samom snopsy_r proxyju (ne CSD poziv).
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Registar mutexa po snopsy_r endpointu — jamči da po jednom modemu ide
/// najviše jedan CSD poziv u isto vrijeme. Procesno globalan namjerno: i
/// periodični poller i ručni poll iz sučelja moraju čekati istu bravu.
static ENDPOINT_LOCKS: OnceLock<std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();

/// Dohvati (ili stvori) bravu za zadani snopsy_r endpoint.
pub fn endpoint_lock(endpoint: &str) -> Arc<Mutex<()>> {
    let registry = ENDPOINT_LOCKS.get_or_init(Default::default);
    let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    guard.entry(endpoint.to_string()).or_default().clone()
}

/// Runtime konfiguracija jednog AtoN objekta.
#[derive(Debug, Clone)]
pub struct AtonStation {
    pub object_id:         uuid::Uuid,
    pub station_id:        String,
    pub name:              String,
    /// host:port snopsy_r proxyja
    pub endpoint:          String,
    pub poll_interval_sec: u64,
    pub sync_clock:        bool,
    pub cfg:               ObjectConfig,
}

impl AtonStation {
    /// Pretvori DB konfiguraciju u runtime oblik. `None` ako objektu nedostaje
    /// nešto bez čega se poziv ne može odraditi (endpoint, broj, adresa).
    pub fn from_poll_config(c: &AtonPollConfig) -> Option<Self> {
        let endpoint = c.aton_snopsy_endpoint.clone()?;
        let number   = c.aton_number.clone()?;
        let addr     = u8::try_from(c.aton_addr?).ok()?;
        let category = u8::try_from(c.aton_category).ok().and_then(Category::from_number)?;
        // Kategorija zna svoj broj registara; konfigurirana vrijednost je
        // rezerva za kategorije kojima mapa još nije poznata.
        let reg_count = category
            .reg_count()
            .unwrap_or_else(|| u16::try_from(c.aton_reg_count).unwrap_or(aton_decode::REG_COUNT as u16));

        Some(Self {
            object_id:  c.id,
            station_id: c.station_id.clone(),
            name:       c.name.clone(),
            endpoint,
            poll_interval_sec: (c.poll_interval_sec as u64).max(MIN_POLL_INTERVAL_SEC),
            sync_clock: c.aton_sync_clock,
            cfg: ObjectConfig {
                number,
                addr,
                category,
                reg_count,
                // Sat se popunjava tik prije poziva (mora biti aktualan).
                clock: None,
                connect_timeout:  Duration::from_secs(c.aton_connect_timeout_sec.max(1) as u64),
                response_timeout: Duration::from_secs(c.aton_response_timeout_sec.max(1) as u64),
            },
        })
    }
}

/// Sat za sinkronizaciju RTU-a. Šalje se lokalno vrijeme servera — vremenski
/// žigovi u RTU-u ("temperatura u 01:00 / 13:00") su lokalni, kao i u starom
/// nadzoru, pa server treba imati postavljenu zonu (Europe/Zagreb).
fn clock_now() -> ClockSet {
    let now = chrono::Local::now();
    ClockSet {
        year:    now.year() as u16,
        month:   now.month()  as u16,
        day:     now.day()    as u16,
        hour:    now.hour()   as u16,
        minute:  now.minute() as u16,
        second:  now.second() as u16,
        weekday: now.weekday().number_from_monday() as u16,
    }
}

/// Otvori TCP vezu prema snopsy_r-u i odradi jedan poziv. **Blokirajuće** —
/// zovi kroz `spawn_blocking`.
fn read_via_snopsy(endpoint: &str, cfg: &ObjectConfig) -> Result<Aton, SessionError> {
    let addr = endpoint
        .to_socket_addrs()
        .map_err(SessionError::Io)?
        .next()
        .ok_or_else(|| SessionError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("snopsy_r endpoint '{endpoint}' se ne razrješava"),
        )))?;

    let mut link = TcpStream::connect_timeout(&addr, TCP_CONNECT_TIMEOUT).map_err(SessionError::Io)?;
    link.set_read_timeout(Some(LINK_READ_TIMEOUT)).map_err(SessionError::Io)?;
    link.set_write_timeout(Some(Duration::from_secs(5))).map_err(SessionError::Io)?;
    link.set_nodelay(true).ok();

    read_object(&mut link, cfg)
}

/// Odradi jedan poll (poziv + upis u bazu) uz poštovanje serijalizacije po
/// modemu. Koristi ga i periodični poller i ručni poll iz sučelja.
pub async fn poll_aton_once(
    pool: &PgPool,
    station: &AtonStation,
    lock: &Mutex<()>,
) -> Result<Aton, SessionError> {
    let mut cfg = station.cfg.clone();
    if station.sync_clock {
        cfg.clock = Some(clock_now());
    }

    let endpoint = station.endpoint.clone();
    let station_id = station.station_id.clone();

    // Držimo mutex kroz cijeli poziv — nikad dva CSD poziva po istom modemu.
    let _guard = lock.lock().await;
    let started = Instant::now();
    info!(station = %station_id, endpoint = %endpoint, addr = cfg.addr, "CSD poziv → start");

    let result = tokio::task::spawn_blocking(move || read_via_snopsy(&endpoint, &cfg))
        .await
        .unwrap_or_else(|e| Err(SessionError::Io(io::Error::other(
            format!("blocking task panicked: {e}"),
        ))));

    info!(
        station = %station_id,
        elapsed_ms = started.elapsed().as_millis() as u64,
        ok = result.is_ok(),
        "CSD poziv → kraj"
    );

    let aton = result?;
    if let Err(e) = store_reading(pool, station, &aton).await {
        // Poziv je uspio, ali upis nije — podatak je ispravan, javi grešku
        // spremišta odvojeno da se ne miješa s greškama linije.
        error!(station = %station_id, error = %e, "Upis AtoN očitanja nije uspio");
    }
    Ok(aton)
}

/// Spremi očitanje: puni zapis u `aton_readings`, a podskup koji nadzor već
/// servira i u `measurements_10min` — tako AtoN objekt radi u svim postojećim
/// pregledima, grafovima, analitici baterije i detekciji tihe stanice.
async fn store_reading(pool: &PgPool, station: &AtonStation, a: &Aton) -> crate::errors::AppResult<()> {
    // Zaokruži na punu minutu — očitanje je trenutno stanje u času poziva.
    let recorded_at = Utc::now().with_second(0).and_then(|t| t.with_nanosecond(0)).unwrap_or_else(Utc::now);

    let reading = AtonReadingInsert::from_aton(
        Some(station.object_id), &station.station_id, recorded_at, station.cfg.category, a);
    db_aton::insert_aton_reading(pool, &reading).await?;

    db::insert_measurement_10min(pool, &Measurement10minInsert {
        object_id:           Some(station.object_id),
        station_id:          station.station_id.clone(),
        recorded_at,
        datalogger_temp_avg: Some(a.temp_trenutna_c),
        // Glavna baterija stanice = baterija glavnog svjetla.
        battery_voltage_avg: Some(a.gl_svj.napon_v),
        battery_current_avg: Some(a.gl_svj.struja_a),
        // Struja izvora svjetla je isti podatak koji CR300 stanice javljaju
        // kao struju fenjera, pa ide u isto polje i crta se istim grafom.
        lantern_current_avg: Some(a.struja_led_a),
        // Doba dana RTU sam računa iz tablice sunca — 1 = noć.
        solar_daylight_smp:  Some(i16::from(a.doba_dana != DobaDana::Noc)),
        ..Default::default()
    }).await?;

    // Alarmi idu istim putem kao i alarmi CR300 stanica: snapshot u `alarms`,
    // pa obavijesti samo za stvarno nove zapise (izbjegava ponavljanje pri
    // ponovnom prozivanju iste minute).
    let alarm = alarm_insert_from_aton(
        Some(station.object_id), &station.station_id, recorded_at, a);
    if db::insert_alarm(pool, &alarm).await? {
        crate::notify::dispatch_for_alarm(pool, &alarm).await;
    }

    Ok(())
}

// ── Periodični poller ─────────────────────────────────────────────────────

async fn poll_station_loop(
    station: AtonStation,
    lock: Arc<Mutex<()>>,
    pool: PgPool,
    status: SharedPollerStatus,
) {
    info!(
        station = %station.station_id,
        name = %station.name,
        endpoint = %station.endpoint,
        addr = station.cfg.addr,
        number = %station.cfg.number,
        interval_sec = station.poll_interval_sec,
        "AtoN poller pokrenut"
    );

    let mut interval = tokio::time::interval(Duration::from_secs(station.poll_interval_sec));

    loop {
        interval.tick().await;

        match poll_aton_once(&pool, &station, &lock).await {
            Ok(a) => {
                info!(
                    station = %station.station_id,
                    napon_v = a.gl_svj.napon_v,
                    struja_a = a.gl_svj.struja_a,
                    temp_c = a.temp_trenutna_c,
                    "AtoN očitanje zaprimljeno"
                );
                let mut s = status.write().await;
                s.online.insert(station.station_id.clone(), true);
                s.last_poll.insert(station.station_id.clone(), Utc::now());
                s.last_error.remove(&station.station_id);
            }
            Err(e) => {
                // Neuspio poll (timeout, BUSY, LRC/decode) nije fatalan —
                // objekt ostaje bez svježeg mjerenja i postojeća detekcija
                // tihe stanice ga označi kao nedostupnog.
                warn!(station = %station.station_id, error = %e, "AtoN poll neuspješan");
                let mut s = status.write().await;
                s.online.insert(station.station_id.clone(), false);
                s.last_error.insert(station.station_id.clone(), e.to_string());
            }
        }
    }
}

/// Pokreni periodične pollere za sve AtoN objekte.
pub fn start_aton_pollers(stations: Vec<AtonStation>, pool: PgPool, status: SharedPollerStatus) {
    for station in stations {
        let lock   = endpoint_lock(&station.endpoint);
        let pool   = pool.clone();
        let status = status.clone();
        tokio::spawn(async move { poll_station_loop(station, lock, pool, status).await; });
    }
}

/// Učitaj AtoN objekte iz baze (`source_kind = 'aton_csd'`, polling uključen).
pub async fn load_stations_from_db(pool: &PgPool) -> Vec<AtonStation> {
    match db_aton::list_pollable_aton_objects(pool).await {
        Ok(configs) => {
            let total = configs.len();
            let stations: Vec<AtonStation> = configs.iter().filter_map(|c| {
                let s = AtonStation::from_poll_config(c);
                if s.is_none() {
                    warn!(station = %c.station_id, "AtoN objekt ima nepotpunu konfiguraciju — preskačem");
                }
                s
            }).collect();
            info!("Učitano {} AtoN objekt(a) od {} kandidata", stations.len(), total);
            stations
        }
        Err(e) => {
            error!("Učitavanje AtoN konfiguracija iz baze nije uspjelo: {}", e);
            vec![]
        }
    }
}
