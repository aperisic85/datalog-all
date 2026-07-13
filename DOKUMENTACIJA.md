# Datalog — Dokumentacija sustava

## Pregled aplikacije

**Datalog** je sustav za nadzor i upravljanje pomorskim navigacijskim objektima — svjetionicima, plutačama i biljegama. Aplikacija prikuplja telemetrijske podatke s CR300 datalogera, analizira stanje opreme, generira alarme i predviđa kvarove na temelju mjerenih parametara.

**Tehnologije:**
- Backend: Rust (Axum framework) + PostgreSQL
- Frontend: React + TypeScript
- Prostorna pohrana: PostGIS (geolokacijske upite)
- Karte: OpenStreetMap putem Leaflet biblioteke

---

## 1. Autentifikacija i upravljanje korisnicima

### Prijava i tokeni
Sustav koristi JWT (JSON Web Token) autentifikaciju s dva tipa tokena: kratkoročni pristupni token i dugoročni refresh token. Korisnik se prijavljuje korisničkim imenom i lozinkom, a sustav izdaje oba tokena. Refresh token automatski obnavlja sesiju bez ponovne prijave.

### Uloge korisnika
Postoje tri razine pristupa:

| Uloga | Opis |
|-------|------|
| **Admin** | Puni pristup — upravljanje korisnicima, regijama, konfiguracijama i svim podacima |
| **Operator** | Regionalni pristup — može upravljati stanicama i alarmima unutar dodijeljenih regija, uključujući ručno upravljanje lantenom |
| **Viewer** | Samo čitanje — pregled podataka za dodijeljene regije |

### Regionalna prava pristupa
Operator i Viewer mogu imati pristup samo određenim regijama. Admin dodjeljuje ili oduzima pristup per-regija za svakog korisnika. Ovo znači da operator jedne regije ne može vidjeti podatke druge regije.

### Sigurnost lozinki
Lozinke su hashirane algoritmom bcrypt. Administrator može kreirati korisnike i dodijeliti im početnu lozinku; korisnici mogu mijenjati vlastitu lozinku.

### Revizijski dnevnik (Audit log)
Svaka akcija u sustavu — prijava, kreiranje, izmjena, brisanje — bilježi se u revizijski dnevnik s informacijom o korisniku, tipu akcije, entitetu i vremenskoj oznaci. Revizijski dnevnik dostupan je adminima za pregled i filtriranje.

---

## 2. Upravljanje stanicama (objektima)

### Registar objekata
Svaka praćena lokacija (svjetionik, plutača, biljega, meteorološka stanica…) registrirana je kao "objekt" s jedinstvenim identifikatorom koji odgovara konfiguraciji CR300 datalogera.

### Tipovi objekata
Aplikacija podržava 10 tipova objekata:
- Svjetionik, plutača, biljega, meteorološka stanica, i dr.

### Podaci koji se prate po objektu
- Naziv i jedinstveni ID stanice
- GPS koordinate (geografska širina i dužina)
- Regija kojoj objekt pripada
- Status (aktivan / neaktivan)
- Datumi stavljanja u pogon i povlačenja iz pogona
- URL, korisničko ime i lozinka za komunikaciju s CR300 datalogerom
- Nominalni kapacitet baterije (u Ah) za analizu
- Dozvoljeni polumjer kretanja (geofence) u metrima
- Timeout za detekciju "tihih stanica" (u minutama)
- Verzija programa i modularni program (JSON format)
- Slike — primarna slika i galerija

---

## 3. Prikupljanje podataka (data ingestion)

### Push mod (CR300 šalje podatke)
CR300 datalogeri sami šalju podatke na API endpoint aplikacije. Autentifikacija se vrši API ključem (SHA256 hash). Tablice koje se primaju:
- `Measurements_10min` — mjerni podaci svakih 10 minuta
- `Alarms_10min` — alarmi
- `Event_log` — zapis događaja

### Pull mod (aplikacija dohvaća podatke)
Aplikacija periodički pita CR300 datalogere za nove podatke. Polling je konfiguriran per-stanica (URL, interval, korisničko ime/lozinka). Više stanica se prati paralelno. Status pollera — zadnje vrijeme dohvata, online/offline, zadnja greška — vidljiv je u sučelju.

Moguće je i ručno pokrenuti poll za pojedinu stanicu direktno iz sučelja, korisno za dijagnostiku.

### Višestruka razlučivost podataka
Mjerni podaci se čuvaju u tri razlučivosti:
- **10 minuta** — sirovi, najdetaljniji podaci
- **1 sat** — satni prosjeci
- **24 sata** — dnevni sažeci

---

## 4. Telemetrija i mjereni parametri

Po svakoj stanici prikupljaju se:

**Datalogger:**
- Temperatura unutar kućišta

**Baterija:**
- Napon (prosječni i statusni), struja
- Status: FLAT / LOW / OK

**Solarni panel:**
- Napon, detekcija dnevnog svjetla, zastavica dan/noć

**Modem:**
- Stanje napajanja, internetska konekcija

**GPS (Garmin):**
- Broj vidljivih satelita, pozicija, komunikacijski status

**Lanterna:**
- Komunikacijski status, stanje aktivacije, struja

**Senzor vidljivosti** (modularni):
- Komunikacijski status, izmjerena vrijednost, alarmi

**Signal magle** (modularni):
- Status (uključen/isključen)

---

## 5. Sustav alarma

### Tipovi alarma
Definirano je 20 tipova alarma:

| Kategorija | Tipovi |
|-----------|--------|
| **Baterija** | nizak napon, kritično nizak napon, ostale greške |
| **Datalogger** | visoka temperatura, visok napon, ostale greške |
| **GPS (Garmin)** | komunikacijska greška, ostale greške |
| **Lanterna** | ugašena noću, upaljena danju, komunikacijska greška, ostale greške |
| **Modem** | greška mreže, ostale greške |
| **Stanica** | izvan dozvoljenog polumjera (geofence prekršaj) |
| **Senzor vidljivosti** | komunikacijska greška, senzorska greška |
| **Signal magle** | isključen za magle, uključen bez magle |
| **Opće** | opća greška stanice |

### Predmemorija alarma (alarm cache)
Za svaki objekt sustav održava predmemoriju s: statusom aktivnih alarma, brojem alarma, najgorom razinom i zadnjim vremenom alarma. Ovo omogućuje brze upite bez skupih JOIN operacija.

### Upravljanje alarmima
- Pregled svih aktivnih i povijesnih alarma
- Filtriranje po regiji, statusu, vremenskom rasponu
- Potvrda alarma (acknowledgment)
- Odlaganje alarma (shelving)
- Brisanje alarma
- Pregled po stanici

### Odlaganje alarma (shelving)
Alarm se može privremeno odložiti — po tipu alarma ili za cijeli objekt — na
odabrano vrijeme (5 minuta do 30 dana), uz opcijski razlog. Dok odlaganje traje:
- ne šalju se obavijesti (Telegram / Slack / webhook) za taj alarm,
- alarm se u sučelju prikazuje prigušeno s oznakom **ODL** i ne pokreće sirenu.

Odlaganje automatski istječe, a može se i ručno ukinuti. Ako je alarm nakon
isteka i dalje aktivan, obavijesti se ponovo šalju (re-annunciation). Sva
odlaganja i ukidanja bilježe se u audit log.

### Vizualizacija — toplinska mapa alarma
Za svaku stanicu generira se toplinska mapa koja prikazuje učestalost alarma po satu u danu i danu u tjednu. Ovo omogućuje lako uočavanje uzoraka (npr. alarmi koji se redovito javljaju noću ili vikendima).

---

## 6. Detekcija tihih stanica

### Što je "tiha stanica"?
Tiha stanica (silent station) je stanica koja prestane slati podatke — zbog kvara komunikacijske opreme, modemske greške, gubitka napajanja ili drugog problema — ali nije eksplicitno prijavila alarm.

### Kako radi detekcija?
- Svaki objekt ima konfigurirani **timeout tišine** (default: 120 minuta)
- Baza podataka automatski bilježi **zadnje vrijeme primljenog mjerenja** (`last_measurement_at`) putem database triggera koji se aktivira na svaki novi zapis
- Sustav računa zastavicu `is_silent`: ako od zadnjeg mjerenja prođe više minuta od konfiguriranog timeouta, stanica se označava tihom
- Tihe stanice vidljive su u popisima objekata, na dashboardu i u filtrima

### Razlika od alarma
Alarm se javlja kad datalogger pošalje eksplicitnu grešku. Tiha stanica se detektira pasivno — kad podaci jednostavno prestanu dolaziti. To su dva komplementarna mehanizma za detekciju problema.

---

## 7. Analiza i predviđanje stanja baterije

Ovo je jedna od najnaprednijih značajki sustava. Koriste se tri različite analitičke metode.

### 7.1 Linearna regresija — predviđanje pražnjenja

**Cilj:** Predvidjeti kada će baterija dosegnuti kritičnu razinu napona.

**Metoda:** OLS (Ordinary Least Squares) linearna regresija na dnevnim minimumima napona. Koriste se dnevni minimumi (a ne prosjeci) jer eliminiraju utjecaj solarnog punjenja — dnevni minimum se mjeri noću, kad se baterija najrealističnije odražava.

**Izlazne veličine:**
- Nagib krivulje (V/sat) — brzina degradacije
- Trenutni trend napona
- Procijenjeno vrijeme do **upozoravajućeg praga** (11.5 V)
- Procijenjeno vrijeme do **kritičnog praga** (10.5 V)
- R² koeficijent — pouzdanost procjene
- Klasifikacija stanja: `stable` / `degrading` / `warning` / `critical` / `charging`

**Posebna napomena za tihe stanice:** Algoritam ispravno rukuje slučajem gdje stanica trenutno ne šalje podatke — koristi zadnji poznati uzorak kao referentnu točku, a ne ekstrapolira u budućnost od trenutnog trenutka. Ovo sprečava lažne alarme za stanice koje su privremeno tihе.

---

### 7.2 Procjena kapaciteta baterije iz totalizatora

**Cilj:** Procijeniti koliki je stvarni kapacitet baterije (u Ah) i je li smanjen u odnosu na nominalnu vrijednost.

**Metoda:** Analiza dnevnih vrijednosti totalizatora punjenja i pražnjenja. Algoritam pronalazi **najdulji uzastopni period deficita** — dana u kojima je pražnjenje veće od punjenja — i iz toga estimira stvarni kapacitet baterije.

**Izlazne veličine:**
- Procijenjeni kapacitet (Ah) — donja granica
- Postotak zdravlja (0–100%)
- Maksimalno dnevno pražnjenje
- Maksimalni period deficita (dani)
- Status: `good` / `degraded` / `replace` / `insufficient_discharge` / `insufficient_data`

**Posebna zaštita od lažnih alarmi:** Ako solarni panel pokriva 100% potrošnje i baterija se nikad ne prazni, algoritam vraća status `insufficient_discharge` umjesto da pogrešno klasificira bateriju kao degradiranu.

---

### 7.3 Procjena zdravlja iz napona (night sag analiza)

**Cilj:** Otkriti baterije koje ne drže napon ili imaju mali efektivni kapacitet.

**Metoda:** Analiza razlike između dnevnog maksimuma i noćnog minimuma napona. Sustav automatski prepoznaje je li sustav na 12V ili 24V (autoskaliranje).

**Što se detektira:**
- **Noćni pad napona (night voltage sag):** baterija se dobro napuni, ali brzo gubi napon noću — znak degradiranog kapaciteta
- **Dnevna amplituda napona (daily swing):** velika razlika max/min ukazuje na mali efektivni kapacitet

**Izlazni status:** `good` / `degraded` / `replace` / `insufficient_data`

---

## 8. Analiza solarnih panela

### Usporedba s vremenskim podacima
Sustav integrira **Open-Meteo API** (besplatni meteorološki servis) koji daje satne podatke o:
- Solarnoj iradijanciji (W/m²)
- Pokrivenosti oblacima
- Brzini vjetra, oborinama, temperaturi zraka

### Ocjena efikasnosti solarnog panela
**Metoda:** Usporedba stvarnog napona solarnog panela s teoretskim izlazom na temelju izmjerene iradijancije.

**Izlazne veličine:**
- Bazni omjer (30-dnevni prosjek napon/iradijancija)
- Nedavni omjer (7-dnevni prosjek)
- Dnevna insolacija (kWh/m²)
- Ocjena efikasnosti (0–100 bodova): `nedavni omjer / bazni omjer × 100`

**Pragovi:**
- Ispod 75% → upozorenje (`warn`)
- Ispod 55% → kritično (`critical`)

**Primjena:** Otkrivanje zaprljanih, oštećenih ili zasjenjenih solarnih panela — panel koji generira manje struje nego što bi trebao prema vremenskim uvjetima.

---

## 9. Sustav notifikacija

### Kanali obavješćivanja
Podržana su tri kanala:

| Kanal | Konfiguracija |
|-------|--------------|
| **Telegram** | Bot token + Chat ID |
| **Webhook** | Generični HTTP POST na URL |
| **Slack** | Incoming webhook URL |

### Pravila usmjeravanja
Za svaki kanal definiraju se pravila s filtiranjem:
- **Regija:** Notifikacije samo za određene regije ili sve
- **Minimalna razina ozbiljnosti:** INFO / WARN / ERROR / FATAL
- **Tihe sate (quiet hours):** Vremenski raspon u UTC-u unutar kojeg se ne šalju nekritične notifikacije
- **Cooldown:** Sprečavanje spama — isti alarm se ne šalje ponovo dok ne prođe N minuta
- **Notifikacija o rješavanju:** Opcijsko slanje poruke kada se alarm ugasi
- **Enable/disable:** Svako pravilo može se privremeno isključiti

### Praćenje stanja alarma
Sustav prati tranzicije stanja per (objekt, tip alarma):
- Prijelaz **neaktivan → aktivan**: šalje alarm
- Prijelaz **aktivan → neaktivan**: šalje poruku "riješeno" (ako je uključeno)

Ovo sprečava lažna ponavljanja i slanje notifikacija za alarmе koji su već poznati.

Obavijesti se šalju **samo za nove zapise alarma**: duplikati koje poller ponovo
dohvati (restart servera, tablica bez broja zapisa) odbijaju se na bazi i ne
okidaju ponovno slanje. Ponovljena obavijest za i dalje aktivan alarm (nakon
cooldowna) jasno je označena s "ALARM I DALJE AKTIVAN" i navodi otkad alarm traje.
Odloženi (shelvani) alarmi ne šalju obavijesti dok odlaganje traje.

### Dnevnik notifikacija
Sve poslane notifikacije se bilježe s: vremenom slanja, kanalom, statusom (uspjeh/greška), sadržajem poruke i eventualnom greškom.

---

## 10. Telegram Bot (dvosmjerna komunikacija)

Bot ne zahtijeva javni URL — koristi long-polling prema Telegram serverima. Odgovara samo na registrirane chat ID-ove (iz konfiguracije kanala).

### Naredbe bota

| Naredba | Opis |
|---------|------|
| `/status` | Sažetak po regijama: broj objekata, broj alarma |
| `/alarmi` | Lista trenutno aktivnih alarma |
| `/objekt <naziv>` | Detalji o pojedinoj stanici |
| `/ai <pitanje>` | Eksplicitan upit prirodnim jezikom (vidi niže) |
| `/pomoc` | Prikaz dostupnih naredbi |

### Upiti prirodnim jezikom (AI)

Ako je postavljena env varijabla `LLM_API_KEY`, bot uz naredbe prima i **slobodan
tekst** te ga preko besplatnog LLM-a pretvori u odgovarajuću naredbu. Primjeri:

- „koliki je sad napon baterije na objektu Barbarinac?“ → `/objekt Barbarinac`
- „je li Galija u alarmu?“ → `/objekt Galija`
- „daj mi pregled stanja“ → `/status`
- „koji su aktivni alarmi?“ → `/alarmi`

**Prirodni odgovori (hibrid):** za pitanja o pojedinom objektu bot vraća kratku,
prirodnu rečenicu umjesto fiksne kartice. Npr. „radi li svjetlo na objektu Umag?“
→ „Ne radi svjetlo na objektu Umag (0%).“ Radi u dva koraka:
1. LLM prepozna namjeru, objekt i *fokus* pitanja (`svjetlo` / `baterija` /
   `alarm` / `mjerenje` / `sve`);
2. backend iz baze složi **točne činjenice** za taj fokus, a LLM ih samo
   preformulira u rečenicu — ne smije mijenjati brojeve.

Za opća pitanja („reci mi sve o objektu X“) i dalje se vraća puna kartica.
Ako drugi LLM poziv padne, bot vraća točne činjenice u jednostavnom obliku.

**Važno:** LLM služi isključivo za *prepoznavanje namjere* i *formulaciju*. Sve
stvarne vrijednosti (napon baterije, alarmi, mjerenja) dohvaćaju se iz baze
podataka, pa nema rizika od izmišljenih podataka.

Radi s bilo kojim OpenAI-kompatibilnim endpointom koji ima besplatni tier
(default: **Groq**, `llama-3.3-70b-versatile`). Konfiguracija preko
`LLM_API_KEY`, `LLM_API_URL`, `LLM_MODEL` (vidi `.env.example`). Ako ključ nije
postavljen, bot radi kao i prije — samo s `/` naredbama.

---

## 11. Dashboard i vizualizacija

### Glavna nadzorna ploča
- Kartice po regijama s live statistikama
- Ukupan broj objekata, aktivnih objekata, objekata u alarmu
- Broj lanterni uključenih/isključenih
- Automatsko osvježavanje svakih 60 sekundi
- Animirani statistički prijelazi (smooth count transitions)

### Popis objekata
- Paginiran popis svih stanica
- Filtriranje po: regiji, aktivnosti, statusu alarma, pretraga po nazivu
- Stupci: naziv, ID, regija, status, stanje alarma, datum stavljanja u pogon
- Direktan pristup detaljima, uređivanju i brisanju

### Stranica detalja objekta
Organizirana u tabove:

| Tab | Sadržaj |
|-----|---------|
| **Pregled** | Trenutni napon, temperatura, koordinate, status |
| **Grafovi** | Vremenski nizovi (6h / 24h / 7 dana / proizvoljni datum): napon baterije, solarni napon, modem/GPS metrike |
| **Alarmi** | Povijesni alarmi stanice s opcijama upravljanja |
| **Događaji** | Event log — promjene stanja i dijagnostički zapisi |
| **Toplinska mapa** | Frekvencija alarma po satu × dan u tjednu |

- Karta s prikazom lokacije i geofence polumjera
- Editabilna polja: konfiguracija, koordinate, napomene
- Kontrole: ručni poll, brisanje, potvrda alarma

### Povijesni pregled podataka (proizvoljni datum)
Uz brze raspone (6h / 24h / 7d), tab **Grafovi** ima i opciju **Datum** — odabir
proizvoljnog datuma ili raspona datuma (od–do):
- Rezolucija podataka bira se automatski prema širini raspona: do 3 dana →
  10-minutni podaci, do 42 dana → satni prosjeci, dulje → dnevni sažeci
- Strelice ◀ ▶ pomiču odabrani raspon naprijed/natrag za njegovu širinu —
  praktično za listanje dan po dan
- Vremenski podaci (Open-Meteo) prikazuju se za povijesne raspone unutar
  zadnjih 30 dana
- **Tablica mjerenja** — prikaz točnih izmjerenih vrijednosti (napon, struja,
  solar, temperatura, svjetlo…) s vremenskim oznakama, dostupna za bilo koji raspon
- **CSV izvoz** — preuzimanje svih mjerenja odabranog raspona (svi stupci,
  format kompatibilan s Excelom)

### Stranica usporedbe stanica
- Izbor do 4 stanice za usporedbu
- Preklapajući grafovi napona na istom prikazu
- Svaka stanica u drugoj boji
- Odabir vremenskog raspona

### Karta
- OpenStreetMap karta s Leaflet bibliotekom
- Kružni markeri po statusu: zelena (OK), crvena (alarm), siva (neaktivna)
- Badge s brojem alarma na markerima
- Filtri: sve / samo alarmi / samo neaktivne
- Popup s detaljima na klik

---

## 12. Upravljanje datalogerom

### SetValue naredba
Operatori i admini mogu slati SetValue naredbe CR300 datalogeru za:
- Ručno paljenje/gašenje lanterne
- Upisivanje vrijednosti u Public tablicu datalogera

Naredba se bilježi u revizijskom dnevniku.

### Ručni poll
Za dijagnostiku moguće je odmah pokrenuti dohvat podataka s datalogera za odabranu stanicu. Rezultat prikazuje broj novih zapisa po tablici.

### Status pollera
Javno dostupan endpoint (bez autentifikacije) koji prikazuje:
- Zadnje vrijeme dohvata po stanici
- Online/offline status
- Zadnja greška (ako postoji)

---

## 13. Admin sučelje

### Upravljanje korisnicima
- Popis svih korisnika s ulogama i zadnjim loginima
- Kreiranje korisnika (korisničko ime, email, lozinka, uloga)
- Aktivacija/deaktivacija korisnika (soft delete)
- Dodjela/oduzimanje regionalnih prava

### Upravljanje regijama
- Kreiranje, uređivanje i brisanje regija
- Boja po regiji (za vizualnu distinkciju u sučelju)
- Aktivacija/deaktivacija regije

### Upravljanje notifikacijama
- Kanali: kreiranje, uređivanje, testiranje, brisanje
- Pravila: kreiranje, uređivanje, brisanje
- Log: pregled svih poslanih notifikacija s filterima

### Revizijski dnevnik
- Prikaz svih akcija u sustavu
- Filtriranje po: korisniku, tipu akcije, entitetu, vremenskom rasponu
- Paginacija

---

## 14. Geofence — nadzor položaja

Svaki objekt ima konfigurirani **dozvoljeni polumjer kretanja** u metrima. Ako GPS pozicija stanice prijeđe taj polumjer, sustav automatski generira alarm tipa "izvan dozvoljenog polumjera".

Aplikacija koristi **PostGIS** prostornu ekstenziju PostgreSQL baze za geolokacijske upite. Na karti je vidljiv polumjer geofencea za svaki objekt.

---

## 15. Sigurnost sustava

| Aspekt | Implementacija |
|--------|---------------|
| Autentifikacija API ključa | SHA256 hash provjera |
| JWT tokeni | Konfigurirano trajanje, server-side invalidacija |
| Lozinke | bcrypt hash |
| SQL injection | sqlx prepared statements (ne string konkatenacija) |
| Uloge i permisije | RBAC + regionalni scope |
| Revizija | Potpuna audit trail svih admin akcija |

---

## 16. Modularnost datalogera

CR300 datalogeri mogu imati različite programe s različitim senzorima. Sustav podržava ovo kroz `program_features` JSON polje po objektu, koje opisuje koji moduli su instalirani (npr. senzor vidljivosti, signal magle). Frontend i backend dinamički prilagođavaju prikaz i analizu prema instaliranim modulima.

---

## Sažetak ključnih analitičkih metoda

| Značajka | Metoda | Prag |
|---------|--------|------|
| Predviđanje pražnjenja baterije | Linearna regresija (OLS) na dnevnim minimumima | 11.5V upozorenje, 10.5V kritično |
| Procjena kapaciteta | Analiza totalizatora — najdulji deficit niz | Usporedba s nominalnim kapacitetom |
| Zdravlje baterije | Night sag + dnevna amplituda napona | 12V / 24V auto-skaliranje |
| Efikasnost solarnog panela | Omjer stvarni napon / iradijancija | 75% warn, 55% critical |
| Detekcija tihe stanice | Timeout od zadnjeg mjerenja (DB trigger) | Konfigurirano po stanici (default 120 min) |
| Geofence alarm | PostGIS prostorni upit | Konfigurirani polumjer po stanici |
| Toplinska mapa alarma | Frekvencija po satu × dan (grid vizualizacija) | Vizualni uvid, bez praga |
