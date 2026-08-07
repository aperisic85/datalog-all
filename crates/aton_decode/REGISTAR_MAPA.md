# Registarska mapa — AtoN stanica "Prišnjak" (PS br. 446)

Rekonstruirana korelacijom sirovih Modbus ASCII okvira (kroz `snopsy_r`) s
prikazom na starom nadzoru. Referentni poll: **2026-08-07 10:59:47**, RTU
**adresa 51 (0x33)**. Verificirano u kodu (`aton_decode`, test
`decodes_to_screen_values`).

## Protokol

- Čitanje: **Read Holding Registers (func 0x03)**, start reg **0**, količina **31**.
- Svaki registar 16-bit, big-endian.
- **Skaliranje: sve analogno = `i16` (signed!) ÷ 100.** Struje potrošnje su negativne.
- Prije čitanja master šalje **Write Multiple Registers (func 0x10)** @ reg 100,
  9 registara = sinkronizacija sata: `[1, godina, mjesec, dan, sat, min, sek, dan_u_tjednu, 0]`.

## Mapa (potvrđeno ekranom)

| reg | polje | skala | primjer | prikaz na nadzoru |
|----:|-------|-------|--------:|-------------------|
| 0  | temperatura trenutna        | ÷100 °C | 32.64 | TEMPERATURA / TRENUTNA |
| 1  | struja — AUTOMAT            | ÷100 A  | 0.54  | STANJE BATERIJA / STRUJA / AUTOMAT |
| 2  | struja — GL. SVJ.           | ÷100 A  | 0.32  | STANJE BATERIJA / STRUJA / GL. SVJ. |
| 3  | napon — AUTOMAT             | ÷100 V  | 13.71 | STANJE BATERIJA / NAPON / AUTOMAT |
| 4  | napon — GL. SVJ.            | ÷100 V  | 13.47 | STANJE BATERIJA / NAPON / GL. SVJ. |
| 10 | temperatura u 01:00         | ÷100 °C | 32.66 | TEMPERATURA / U 01:00 |
| 11 | temperatura u 13:00         | ÷100 °C | 32.38 | TEMPERATURA / U 13:00 |
| 19 | dnevni prosjek napon GL.SVJ.| ÷100 V  | 13.09 | DNEVNI PROSJECI / NAPON / GL.SVJ. |
| 20 | dnevni prosjek napon AUTOMAT| ÷100 V  | 13.16 | DNEVNI PROSJECI / NAPON / AUTOMAT |
| 21 | struja punjenja — AUTOMAT   | ÷100 A  | 0.62  | DNEVNI PROSJECI / STRUJA PUNJENJA / AUTOMAT |
| 22 | struja potrošnje bat.— AUTOMAT | ÷100 A | -0.15 | DNEVNI PROSJECI / STRUJA POTROŠNJE / AUTOMAT |
| 23 | struja punjenja — GL.SVJ.   | ÷100 A  | 0.44  | DNEVNI PROSJECI / STRUJA PUNJENJA / GL.SVJ. |
| 24 | struja potrošnje bat.— GL.SVJ. | ÷100 A | -0.16 | DNEVNI PROSJECI / STRUJA POTROŠNJE / GL.SVJ. |
| 27 | struja potrošnje (izvor svj.) | ÷100 A | -0.13 | DNEVNI PROSJEK POTROŠNJE / STRUJA POTROŠNJE |
| 28 | dnevna potrošnja struje     | ÷100 A  | -1.39 | DNEVNI PROSJEK POTROŠNJE / DNEVNA POTROŠNJA |

**Uzorak uparivanja:** dva kanala kroz cijelu mapu — **GL. SVJ.** (glavno svjetlo)
i **AUTOMAT**. Pazi na redoslijed: u bloku 1–4 automat je na neparnima (1,3),
gl.svj. na parnima (2,4); u prosjecima 19/20 obrnuto.

## Još nemapirano

| reg | sirovo | ÷100 | napomena |
|----:|-------:|-----:|----------|
| 12 | 2     | 0.02 | vjerojatno status/brojač — nije analogno |
| 29 | 1123  | 11.23 | nije na ovom ekranu (prag? drugi napon?) |
| 30 | 203   | 2.03  | nije na ovom ekranu |
| 5–9, 13–18, 25, 26 | 0 | — | rezerva / trenutno neaktivni kanali |

**Za dovršetak mape treba još snimaka:**

- **Noćna snimka** (svjetlo upaljeno) — `STRUJA LED SVJETLA` je sad 0, pa se ne
  vidi koji je registar; kad svijetli, taj registar skoči i identificira se.
- **Snimka s alarmom / promjenom statusa** — `DOBA DANA` (DAN/NOĆ), `VRATA`
  (OTVORENA/ZATVORENA), `LED ŽARULJA`, `BLJESAK`, `KOMUNIKACIJA`, `NAPAJANJE` su
  enumeracije; vjerojatno sjede u nekom od sad-nul registara ili kao bitovi.

## Napomene

- **Opći podaci o objektu** (oznaka, koordinate, karakteristika, domet, sektor,
  tip opreme, tel.) su hardkodirani u nadzoru — **ne** stižu Modbusom.
- **VRIJEME** na ekranu je masterov sat (isti koji se šalje u func 0x10 write).
- Ostali RTU-i (adrese 40, 15, …) su **zasebne stanice** — mogu imati istu
  mapu (isti firmware) ili drukčiju; provjeri prvom snimkom po adresi.
