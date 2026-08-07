# Registarska mapa — AtoN stanice (`csd_verzija`, kategorija 7)

Mapa je **verificirana prema izvornom kodu RTU-a** — funkcija
`CreateReturnStringToCenter` (`funkcije.c`) pakira odgovor kao niz
`sprintf("%04X", …)` poziva na fiksnim offsetima, pa je registar `i` upravo
offset `i * 4`. Referentni poll za brojčane primjere: **2026-08-07 10:59:47**,
stanica Prišnjak, ID oznaka **51 (0x33)**. Verificirano testovima u
`aton_decode` (`decodes_to_screen_values`, `decodes_status_and_alarms_from_real_frame`,
`decodes_every_alarm_flag`, `reg17_is_a_bitmask`).

## Verzije programa

Program na RTU-u zove se **`csd_verzija`** i ima **7 podverzija (kategorija)**
ovisno o tome koje podatke stanica šalje. Ovdje je opisana **kategorija 7** —
puni set od 31 registra. Ostale kategorije su prepoznate u kodu
(`aton_decode::Category`), ali im mapa još nije razriješena; dok se ne
razriješi, dekoder za njih vraća `UnsupportedCategory` umjesto da nagađa.

## Protokol

- **ID oznaka objekta** (`OBJECT_ID` u RTU kodu) pakira se **na početak
  Modbus okvira** — to je ujedno Modbus adresa uređaja. Centar po njoj
  prepoznaje tko javlja.
- Čitanje: **Read Holding Registers (func 0x03)**, start reg **0**, količina **31**.
- Svaki registar 16-bit, big-endian.
- **Skaliranje: analogno = `i16` ÷ 100** (signed — struje potrošnje negativne).
  Alarmi i statusi su cjelobrojne zastavice, **ne** dijele se sa 100.
- Prije čitanja master šalje **Write Multiple Registers (func 0x10)** @ reg 100,
  9 registara = sinkronizacija sata: `[1, godina, mjesec, dan, sat, min, sek, dan_u_tjednu, 0]`.
  Deveti registar RTU čita kao **zabranu dojave** (`alertSupress`).

## Analogne vrijednosti

| reg | RTU izvor | polje | skala | primjer |
|----:|-----------|-------|-------|--------:|
| 0  | `temperatureOPS`              | temperatura trenutna         | ÷100 °C | 32.64 |
| 1  | `currentAutomat`              | struja — AUTOMAT             | ÷100 A  | 0.54  |
| 2  | `currentLight`                | struja — GL. SVJ.            | ÷100 A  | 0.32  |
| 3  | `voltageAutomat`              | napon — AUTOMAT              | ÷100 V  | 13.71 |
| 4  | `voltageLight`                | napon — GL. SVJ.             | ÷100 V  | 13.47 |
| 10 | `temperature01h`              | temperatura u 01:00          | ÷100 °C | 32.66 |
| 11 | `temperature13h`              | temperatura u 13:00          | ÷100 °C | 32.38 |
| 19 | `avgLightVoltage`             | dnevni prosjek napona GL.SVJ.| ÷100 V  | 13.09 |
| 20 | `avgAutomatVoltage`           | dnevni prosjek napona AUTOMAT| ÷100 V  | 13.16 |
| 21 | `avgAutomatChargeCurrent`     | struja punjenja — AUTOMAT    | ÷100 A  | 0.62  |
| 22 | `avgAutomatDischargeCurrent`  | struja potrošnje — AUTOMAT   | ÷100 A  | -0.15 |
| 23 | `avgLightChargeCurrent`       | struja punjenja — GL.SVJ.    | ÷100 A  | 0.44  |
| 24 | `avgLightDischargeCurrent`    | struja potrošnje — GL.SVJ.   | ÷100 A  | -0.16 |
| 26 | `currentMaxi`                 | **struja izvora svjetla (LED/Maxi Halo)** | ÷100 A | 0.00 danju |
| 27 | `avgMaxiDischargeCurrent`     | dnevni prosjek potrošnje izvora | ÷100 A | -0.13 |
| 28 | `sumMaxiDischargeEnergy`      | **dnevna potrošnja izvora [Ah]** | ÷100 Ah | -1.39 |

**Uzorak uparivanja:** dva kanala kroz cijelu mapu — **GL. SVJ.** (glavno
svjetlo) i **AUTOMAT**. Pazi na redoslijed: u bloku 1–4 automat je na
neparnima (1,3), gl.svj. na parnima (2,4); u prosjecima 19/20 obrnuto.

Registar 28 je **energija (Ah)**, ne struja — RTU ga računa kao
`sumMaxiDischargeEnergy = Σ struja / (3600 / FREQ_VALUES_FOR_STATISTICS_CURRENT)`.
Stari nadzor ga prikazuje pod „DNEVNA POTROŠNJA".

## Statusi

| reg | RTU izvor | značenje |
|----:|-----------|----------|
| 12 | `dayCycle` | **doba dana**: 0 = sumrak/svitanje, 1 = noć, 2 = dan. RTU interno ima i `SUNSET = -1`, ali ga pri pakiranju izjednačuje sa `SUNRISE` (0). |
| 29 | `sunRiseSetTable[dan][2]` | **početak noći** — minuta od ponoći. Primjer 1123 = 18:43. |
| 30 | `sunRiseSetTable[dan][1]` | **kraj noći** — minuta od ponoći. Primjer 203 = 03:23. |

Registri 29 i 30 su **minute, ne analogne vrijednosti** — ne dijele se sa 100.

## Alarmi

Sve su zastavice `CheckingIsBitOn(alarms, AL_…)` → 0 ili 1, osim registra 17
koji je bitmaska s tri alarma.

| reg | RTU alarm | značenje |
|----:|-----------|----------|
| 5  | `AL_CALL_REQUEST`                | zahtjev za pozivom centra |
| 6  | `AL_TEMPERATURE`                 | temperatura izvan granica |
| 7  | `AL_VOLTAGE_LIGHT`               | napon baterije glavnog svjetla izvan granica |
| 8  | `AL_VOLTAGE_AUTOMAT`             | napon baterije automata izvan granica |
| 9  | `AL_DOOR`                        | vrata otvorena |
| 13 | `AL_FLASH_FIL1`                  | karakteristika bljeska ne odgovara zadanoj |
| 14 | —                                | bljesak 2. žarne niti; RTU uvijek šalje 0 |
| 15 | `AL_LIGHT_ON_AUTOMAT`            | svjetlo se napaja s baterija automata |
| 16 | `AL_AUTOMAT_ON_LIGHT`            | automat se napaja s baterija svjetla |
| 17 | **bitmaska** | bit 0 = `AL_BLOWN_FIL1` (pregorena žarulja / greška izvora)<br>bit 1 = `AL_NOT_WORK_AT_NIGHT_FIL1` (ne radi po noći)<br>bit 2 = `AL_NOT_WORK_AT_NIGHT_PHOTOCELL` (greška fotoćelije) |
| 18 | —                                | pregorena 2. žarna nit; RTU uvijek šalje 0 |
| 25 | `AL_WORK_AT_DAY_FIL1`            | svjetlo radi po danu |

Registar 17 RTU pakira kao
`AL_BLOWN_FIL1 + 2 * AL_NOT_WORK_AT_NIGHT_FIL1 + 4 * AL_NOT_WORK_AT_NIGHT_PHOTOCELL`.

## Rezerve

| reg | primjer | napomena |
|----:|--------:|----------|
| 14, 18 | 0 | druga žarna nit — nije u upotrebi na ovom tipu |

Registri 5–9, 13–18 i 25 su u referentnoj (dnevnoj, bezalarmnoj) snimci bili
nula upravo zato što alarma nije bilo — nisu rezerva.

## Smjer centar → RTU

`DecodeCommunicationInputValues` čita 10 vrijednosti iz clock-write okvira:
zastavica upisa vremena, godina, mjesec, dan, sat, minuta, sekunda, dan u
tjednu, **zabrana dojave** i checksum. Deveta vrijednost (`alertSupress`)
zaustavlja RTU da sam zove centar pri promjeni alarma.

## Napomene

- **Opći podaci o objektu** (oznaka, koordinate, karakteristika, domet, sektor,
  tip opreme, tel.) su hardkodirani u nadzoru — **ne** stižu Modbusom.
- **VRIJEME** na ekranu je masterov sat (isti koji se šalje u func 0x10 write).
- RTU sam zove centar (`CENTER_NUMBER`) kad se promijeni alarmno stanje, osim
  ako je postavljena zabrana dojave.
- U RTU kodu `ChangeStateDigitalInputs` upisuje `doorOPS` iz `I_DI_CALLREQ`, a
  `callRequest` iz `I_DI_DOOR` — ulazi su zamijenjeni u odnosu na nazive.
  Dekoder poštuje ono što okvir deklarira (reg 5 = poziv, reg 9 = vrata); ako
  se u praksi pokaže obrnuto, zamjena ide ovdje, uz snimku kao dokaz.
