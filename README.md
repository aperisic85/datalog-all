# Datalog

Sustav za nadzor i upravljanje pomorskim navigacijskim objektima — svjetionicima, plutačama, biljegama i meteorološkim stanicama.

Datalog prikuplja telemetriju s CR300 datalogera i AtoN RTU uređaja, prati stanje opreme, generira alarme te pomaže predvidjeti probleme s baterijama, solarnim panelima i komunikacijom.

> Projekt je u aktivnom razvoju. Detaljan opis funkcionalnosti nalazi se u [DOKUMENTACIJA.md](DOKUMENTACIJA.md).

## Glavne mogućnosti

- push i pull prikupljanje podataka s CR300 datalogera
- AtoN/CSD komunikacija preko `snopsy_r` proxyja i Modbus protokola
- mjerenja u razlučivosti od 10 minuta, jednog sata i jednog dana
- alarmi, potvrđivanje alarma, odlaganje i ponovna najava
- detekcija tihih stanica i geofence nadzor
- analiza zdravlja i predviđanje pražnjenja baterija
- analiza učinkovitosti solarnih panela uz Open-Meteo podatke
- Telegram, Slack i webhook obavijesti
- Telegram bot s opcionalnim upitima prirodnim jezikom
- karta objekata, grafovi, toplinske mape alarma i CSV izvoz
- upravljanje korisnicima uz RBAC i regionalna prava pristupa
- revizijski dnevnik administrativnih i operativnih akcija

## Arhitektura

```text
CR300 datalogeri ── HTTP push/pull ─┐
                                    ├── Rust / Axum API ── PostgreSQL + PostGIS
AtoN RTU ── CSD / snopsy_r / Modbus ┘          │
                                               ├── React web-aplikacija
Open-Meteo ────────────────────────────────────┤
                                               └── Telegram / Slack / webhook
```

| Dio sustava | Tehnologije |
| --- | --- |
| Backend | Rust, Axum, Tokio, SQLx |
| Frontend | React, TypeScript, Vite |
| Baza | PostgreSQL, PostGIS |
| Karte | Leaflet, OpenStreetMap |
| Grafovi | Recharts |
| Pokretanje | Docker, Docker Compose |

Dekoder AtoN protokola izdvojen je u samostalni crate `crates/aton_decode`. Registarska mapa dokumentirana je u [REGISTAR_MAPA.md](crates/aton_decode/REGISTAR_MAPA.md).

## Brzo pokretanje

### Preduvjeti

- Docker
- Docker Compose

### Pokretanje cijelog sustava

```bash
git clone https://github.com/aperisic85/datalog-all.git
cd datalog-all
cp .env.example .env
docker compose up --build
```

Nakon pokretanja dostupni su:

- web-aplikacija: [http://localhost:8086](http://localhost:8086)
- backend API: [http://localhost:8095](http://localhost:8095)
- provjera zdravlja: [http://localhost:8095/health](http://localhost:8095/health)
- PostgreSQL: `localhost:5432`

Za opcionalni pgAdmin:

```bash
docker compose --profile tools up --build
```

pgAdmin će biti dostupan na [http://localhost:5050](http://localhost:5050).

## Konfiguracija

Primjer svih varijabli nalazi se u [.env.example](.env.example). Najvažnije su:

| Varijabla | Namjena |
| --- | --- |
| `DATABASE_URL` | veza prema PostgreSQL bazi |
| `PORT` | port backend poslužitelja |
| `RUST_LOG` | razina i filtri zapisivanja |
| `JWT_SECRET` | tajna za potpisivanje JWT tokena |
| `TELEGRAM_BOT_TOKEN` | opcionalni Telegram bot |
| `LLM_API_KEY` | opcionalno tumačenje upita prirodnim jezikom |
| `LLM_API_URL` | OpenAI-kompatibilan LLM endpoint |
| `LLM_MODEL` | model koji bot koristi |
| `DATALOGGER_STATIONS` | konfiguracija više CR300 stanica |

Prije produkcijskog pokretanja obavezno promijenite zadane lozinke i postavite snažan `JWT_SECRET`. Produkcijske tajne nemojte spremati u Git.

## Lokalni razvoj

### Backend

Potrebni su Rust toolchain i PostgreSQL/PostGIS baza.

```bash
cargo run
```

### Frontend

```bash
cd frontend
npm install
npm run dev
```

Provjera frontenda:

```bash
npm run lint
npm run build
```

Provjera Rust workspacea:

```bash
cargo test --workspace
```

## Struktura repozitorija

```text
.
├── crates/aton_decode/   # AtoN/Modbus dekoder i testovi
├── frontend/             # React web-aplikacija
├── migrations/           # SQL migracije
├── src/                  # Rust backend
├── .sqlx/                # SQLx offline metapodaci
├── docker-compose.yml
├── Dockerfile
└── DOKUMENTACIJA.md      # detaljna funkcionalna dokumentacija
```

## Dokumentacija

Za opis autentifikacije, alarma, pollera, analitike, AtoN/CSD komunikacije, notifikacija i korisničkog sučelja pogledajte [DOKUMENTACIJA.md](DOKUMENTACIJA.md).
