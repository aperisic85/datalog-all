//! Modbus ASCII protokol i dekoder za AtoN stanice (tip Prišnjak / SDN) preko
//! CSD-a i `snopsy_r` proxyja.
//!
//! Pokriva **obje strane** komunikacije:
//! - **master** (gradnja upita): [`build_read_holding`], [`build_clock_write`],
//!   te driver [`read_object`] koji diže poziv, (opcionalno) sinkronizira sat,
//!   proziva i spušta poziv;
//! - **dekodiranje odgovora**: [`decode_ascii`] → typed [`Aton`].
//!
//! Registarska mapa rekonstruirana korelacijom sirovih okvira (kroz `snopsy_r`)
//! s prikazom starog nadzora — poll 2026-08-07 10:59:47, RTU adresa 51. Svi
//! analogni podaci su `i16` ÷100 (signed; struje potrošnje negativne).
//!
//! Bez vanjskih ovisnosti (samo `std`) — spušta se direktno u tvoj projekt.
//! Driver je sinkroni/blocking; u async servisu ga zovi kroz `spawn_blocking`.

#![warn(clippy::all)]
#![deny(missing_docs)]

use std::fmt;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

/// Modbus adresa RTU-a stanice Prišnjak.
pub const PRISNJAK_ADDR: u8 = 51;
/// Broj holding registara koje ova stanica vraća na Read (func 0x03).
pub const REG_COUNT: usize = 31;

// ───────────────────────── dekodiranje odgovora ─────────────────────────

/// Greška pri dekodiranju Modbus ASCII okvira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Okvir ne počinje znakom ':'.
    MissingStart,
    /// Neparan broj hex znamenki ili nevaljan hex znak.
    BadHex,
    /// Okvir je prekratak za addr + func + lrc.
    TooShort,
    /// Izračunati LRC se ne slaže s primljenim.
    LrcMismatch {
        /// LRC iz okvira.
        expected: u8,
        /// LRC koji smo izračunali.
        actual: u8,
    },
    /// Funkcija nije 0x03 (Read Holding Registers).
    NotReadResponse {
        /// Primljeni kod funkcije.
        func: u8,
    },
    /// Broj registara ne odgovara očekivanom za ovu stanicu.
    UnexpectedRegCount {
        /// Koliko smo registara dobili.
        got: usize,
        /// Koliko ih očekujemo (`REG_COUNT`).
        want: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStart => write!(f, "okvir ne počinje s ':'"),
            Self::BadHex => write!(f, "nevaljan hex sadržaj okvira"),
            Self::TooShort => write!(f, "okvir prekratak (addr/func/lrc)"),
            Self::LrcMismatch { expected, actual } => {
                write!(f, "LRC ne valja: primljen 0x{expected:02X}, izračunat 0x{actual:02X}")
            }
            Self::NotReadResponse { func } => {
                write!(f, "nije Read odgovor (func 0x{func:02X}, očekivano 0x03)")
            }
            Self::UnexpectedRegCount { got, want } => {
                write!(f, "neočekivan broj registara: {got}, očekivano {want}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Napon i struja jedne baterije (glavno svjetlo ili automat).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Baterija {
    /// Napon baterije [V].
    pub napon_v: f32,
    /// Trenutna struja [A] (predznak ovisi o kanalu: punjenje + / potrošnja −).
    pub struja_a: f32,
}

/// Dekodirano očitanje AtoN stanice.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Aton {
    /// Trenutna temperatura [°C] (reg 0).
    pub temp_trenutna_c: f32,
    /// Temperatura zabilježena u 01:00 [°C] (reg 10).
    pub temp_0100_c: f32,
    /// Temperatura zabilježena u 13:00 [°C] (reg 11).
    pub temp_1300_c: f32,
    /// Baterija glavnog svjetla — napon reg 4, struja reg 2.
    pub gl_svj: Baterija,
    /// Baterija automata — napon reg 3, struja reg 1.
    pub automat: Baterija,
    /// Dnevni prosjek napona baterije glavnog svjetla [V] (reg 19).
    pub prosjek_napon_gl_svj_v: f32,
    /// Dnevni prosjek napona baterije automata [V] (reg 20).
    pub prosjek_napon_automat_v: f32,
    /// Struja punjenja baterije glavnog svjetla [A] (reg 23).
    pub punjenje_gl_svj_a: f32,
    /// Struja punjenja baterije automata [A] (reg 21).
    pub punjenje_automat_a: f32,
    /// Struja potrošnje baterije glavnog svjetla [A], obično negativna (reg 24).
    pub potrosnja_gl_svj_a: f32,
    /// Struja potrošnje baterije automata [A], obično negativna (reg 22).
    pub potrosnja_automat_a: f32,
    /// Dnevni prosjek struje potrošnje (izvor svjetla) [A] (reg 27).
    pub potrosnja_izvor_a: f32,
    /// Dnevna potrošnja struje [A] (reg 28).
    pub dnevna_potrosnja_a: f32,
    /// Svih 31 sirovih registara — za još nemapirane (12, 29, 30), rezerve i
    /// buduće alarm/status bitmaske.
    pub regs: [u16; REG_COUNT],
}

/// Raščlanjeni Modbus ASCII okvir (LRC već provjeren).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Adresa uređaja.
    pub addr: u8,
    /// Kod funkcije.
    pub func: u8,
    /// Payload (bez adrese, funkcije i LRC-a). Za Read odgovor: `[byte_count, data…]`.
    pub payload: Vec<u8>,
}

/// i16 pa ÷100 — jedinstveno skaliranje svih analognih kanala ove stanice.
fn v100(reg: u16) -> f32 {
    f32::from(reg as i16) / 100.0
}

/// LRC = dvojni komplement zbroja svih bajtova (mod 256).
#[must_use]
pub fn lrc(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)).wrapping_neg()
}

fn decode_hex(hex: &[u8]) -> Result<Vec<u8>, DecodeError> {
    fn nib(c: u8) -> Result<u8, DecodeError> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(DecodeError::BadHex),
        }
    }
    hex.chunks_exact(2)
        .map(|p| Ok((nib(p[0])? << 4) | nib(p[1])?))
        .collect()
}

/// Raščlani jedan Modbus ASCII okvir (`:` HEX… LRC [CRLF]) i provjeri LRC.
///
/// CRLF na kraju je opcionalan.
///
/// # Errors
/// [`DecodeError`] ako okvir ne počinje s ':', ima nevaljan hex, prekratak je,
/// ili mu LRC ne valja.
pub fn parse_ascii_frame(raw: &[u8]) -> Result<Frame, DecodeError> {
    let raw = raw.strip_prefix(b":").ok_or(DecodeError::MissingStart)?;
    let end = raw
        .iter()
        .rposition(u8::is_ascii_hexdigit)
        .map_or(0, |p| p + 1);
    let hex = &raw[..end];
    if hex.is_empty() || hex.len() % 2 != 0 {
        return Err(DecodeError::BadHex);
    }
    let bytes = decode_hex(hex)?;
    if bytes.len() < 3 {
        return Err(DecodeError::TooShort);
    }
    let (body, lrc_rx) = bytes.split_at(bytes.len() - 1);
    let calc = lrc(body);
    if calc != lrc_rx[0] {
        return Err(DecodeError::LrcMismatch { expected: lrc_rx[0], actual: calc });
    }
    Ok(Frame { addr: body[0], func: body[1], payload: body[2..].to_vec() })
}

/// Dekodira Read odgovor ove stanice u [`Aton`].
///
/// # Errors
/// [`DecodeError`] ako funkcija nije 0x03 ili broj registara nije `REG_COUNT`.
pub fn decode_aton(frame: &Frame) -> Result<Aton, DecodeError> {
    if frame.func != 0x03 {
        return Err(DecodeError::NotReadResponse { func: frame.func });
    }
    let data = frame.payload.get(1..).unwrap_or(&[]); // [0] = byte_count
    if data.len() != REG_COUNT * 2 {
        return Err(DecodeError::UnexpectedRegCount { got: data.len() / 2, want: REG_COUNT });
    }
    let mut regs = [0u16; REG_COUNT];
    for (r, w) in regs.iter_mut().zip(data.chunks_exact(2)) {
        *r = u16::from_be_bytes([w[0], w[1]]);
    }
    Ok(Aton {
        temp_trenutna_c: v100(regs[0]),
        temp_0100_c: v100(regs[10]),
        temp_1300_c: v100(regs[11]),
        gl_svj: Baterija { napon_v: v100(regs[4]), struja_a: v100(regs[2]) },
        automat: Baterija { napon_v: v100(regs[3]), struja_a: v100(regs[1]) },
        prosjek_napon_gl_svj_v: v100(regs[19]),
        prosjek_napon_automat_v: v100(regs[20]),
        punjenje_gl_svj_a: v100(regs[23]),
        punjenje_automat_a: v100(regs[21]),
        potrosnja_gl_svj_a: v100(regs[24]),
        potrosnja_automat_a: v100(regs[22]),
        potrosnja_izvor_a: v100(regs[27]),
        dnevna_potrosnja_a: v100(regs[28]),
        regs,
    })
}

/// Pogodnost: raščlani okvir i odmah dekodiraj u [`Aton`].
///
/// # Errors
/// Propagira greške iz [`parse_ascii_frame`] i [`decode_aton`].
pub fn decode_ascii(raw: &[u8]) -> Result<Aton, DecodeError> {
    decode_aton(&parse_ascii_frame(raw)?)
}

// ───────────────────────── gradnja upita (master) ─────────────────────────

/// Sastavi Modbus ASCII okvir (`:` HEX… LRC CRLF) iz adrese, funkcije i payloada.
#[must_use]
pub fn build_frame(addr: u8, func: u8, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(2 + payload.len());
    body.push(addr);
    body.push(func);
    body.extend_from_slice(payload);
    let l = lrc(&body);

    let mut s = String::with_capacity(3 + (body.len() + 1) * 2);
    s.push(':');
    for &b in &body {
        s.push_str(&format!("{b:02X}"));
    }
    s.push_str(&format!("{l:02X}"));
    s.push_str("\r\n");
    s.into_bytes()
}

/// Read Holding Registers (func 0x03).
#[must_use]
pub fn build_read_holding(addr: u8, start: u16, count: u16) -> Vec<u8> {
    let mut p = Vec::with_capacity(4);
    p.extend_from_slice(&start.to_be_bytes());
    p.extend_from_slice(&count.to_be_bytes());
    build_frame(addr, 0x03, &p)
}

/// Write Multiple Registers (func 0x10).
#[must_use]
pub fn build_write_registers(addr: u8, start: u16, regs: &[u16]) -> Vec<u8> {
    let qty = u16::try_from(regs.len()).unwrap_or(u16::MAX);
    let byte_count = u8::try_from(regs.len() * 2).unwrap_or(u8::MAX);
    let mut p = Vec::with_capacity(5 + regs.len() * 2);
    p.extend_from_slice(&start.to_be_bytes());
    p.extend_from_slice(&qty.to_be_bytes());
    p.push(byte_count);
    for r in regs {
        p.extend_from_slice(&r.to_be_bytes());
    }
    build_frame(addr, 0x10, &p)
}

/// Vrijednosti za sinkronizaciju sata RTU-a (func 0x10 @ reg 100).
///
/// Master ovo šalje prije prozivanja kako bi vremenski žigovi (npr. temperatura
/// „u 01:00" / „u 13:00") bili točni. `weekday`: 1 = ponedjeljak … 7 = nedjelja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSet {
    /// Godina, npr. 2026.
    pub year: u16,
    /// Mjesec 1–12.
    pub month: u16,
    /// Dan 1–31.
    pub day: u16,
    /// Sat 0–23.
    pub hour: u16,
    /// Minuta 0–59.
    pub minute: u16,
    /// Sekunda 0–59.
    pub second: u16,
    /// Dan u tjednu, 1 = pon … 7 = ned.
    pub weekday: u16,
}

/// Sastavi clock-set okvir (func 0x10 @ reg 100, 9 registara).
#[must_use]
pub fn build_clock_write(addr: u8, cs: &ClockSet) -> Vec<u8> {
    let regs = [
        1, cs.year, cs.month, cs.day, cs.hour, cs.minute, cs.second, cs.weekday, 0,
    ];
    build_write_registers(addr, 100, &regs)
}

// ───────────────────────── driver preko snopsy_r ─────────────────────────

/// Greška tijekom poziva/prozivanja preko snopsy_r-a.
#[derive(Debug)]
pub enum SessionError {
    /// I/O greška na TCP vezi prema snopsy_r-u.
    Io(io::Error),
    /// Modem nije javio CONNECT unutar zadanog vremena.
    ConnectTimeout,
    /// Modem je javio BUSY/NO CARRIER/ERROR pri biranju.
    DialFailed(String),
    /// Veza zatvorena prije nego je dovršen poziv.
    Disconnected,
    /// Odgovor nije stigao (cijeli `:`…CRLF okvir) unutar zadanog vremena.
    ResponseTimeout,
    /// Odgovor stigao ali se ne dekodira.
    Decode(DecodeError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::ConnectTimeout => write!(f, "nema CONNECT (timeout)"),
            Self::DialFailed(s) => write!(f, "biranje neuspješno: {s}"),
            Self::Disconnected => write!(f, "veza zatvorena tijekom poziva"),
            Self::ResponseTimeout => write!(f, "nema odgovora (timeout)"),
            Self::Decode(e) => write!(f, "dekodiranje: {e}"),
        }
    }
}
impl std::error::Error for SessionError {}
impl From<io::Error> for SessionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<DecodeError> for SessionError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

fn is_timeout(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut)
}

/// Skuplja bajtove i vraća cijele linije (razdvojene po '\n', bez '\r').
#[derive(Default)]
struct LineAccumulator {
    buf: Vec<u8>,
}
impl LineAccumulator {
    fn push(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.buf.drain(..=pos).collect();
            while matches!(line.last(), Some(b'\n' | b'\r')) {
                line.pop();
            }
            if !line.is_empty() {
                out.push(line);
            }
        }
        out
    }
}

/// Konfiguracija jednog objekta (stanice) na CSD liniji.
#[derive(Debug, Clone)]
pub struct ObjectConfig {
    /// Podatkovni telefonski broj RTU-a (npr. Prišnjak: vidljiv u nadzoru).
    pub number: String,
    /// Modbus adresa RTU-a (npr. [`PRISNJAK_ADDR`]).
    pub addr: u8,
    /// Koliko holding registara čitati (npr. [`REG_COUNT`]).
    pub reg_count: u16,
    /// Ako je `Some`, prije prozivanja se sinkronizira sat RTU-a.
    pub clock: Option<ClockSet>,
    /// Koliko čekati CONNECT nakon biranja.
    pub connect_timeout: Duration,
    /// Koliko čekati cijeli Modbus odgovor.
    pub response_timeout: Duration,
}

/// Odradi jedan poziv preko već otvorene veze prema snopsy_r-u: biranje →
/// (opcionalno) sat → Read → dekodiranje → spuštanje poziva.
///
/// `link` je npr. `TcpStream` spojen na `snopsy_r:2007`. Postavi mu kratki
/// read-timeout (npr. 200 ms) prije poziva da driver može pratiti rokove.
///
/// # Errors
/// [`SessionError`] za I/O, timeout biranja/odgovora, neuspješno biranje ili
/// grešku dekodiranja.
pub fn read_object<S: Read + Write>(link: &mut S, cfg: &ObjectConfig) -> Result<Aton, SessionError> {
    dial(link, &cfg.number, cfg.connect_timeout)?;
    let result = (|| {
        if let Some(cs) = &cfg.clock {
            link.write_all(&build_clock_write(cfg.addr, cs))?;
            // ack write-a pročitamo i odbacimo (isti okvir natrag, kraći)
            let _ = read_modbus_line(link, cfg.response_timeout)?;
        }
        link.write_all(&build_read_holding(cfg.addr, 0, cfg.reg_count))?;
        let frame = read_modbus_line(link, cfg.response_timeout)?;
        Ok(decode_ascii(&frame)?)
    })();
    // uvijek spusti poziv, čak i ako je čitanje puklo
    let _ = hangup(link);
    result
}

/// Digni CSD poziv: pošalji `ATD <broj>` i čekaj CONNECT.
///
/// # Errors
/// [`SessionError::ConnectTimeout`], [`SessionError::DialFailed`],
/// [`SessionError::Disconnected`] ili I/O.
pub fn dial<S: Read + Write>(link: &mut S, number: &str, timeout: Duration) -> Result<(), SessionError> {
    link.write_all(format!("ATD {number}\r").as_bytes())?;
    let deadline = Instant::now() + timeout;
    let mut acc = LineAccumulator::default();
    let mut buf = [0u8; 256];
    loop {
        if Instant::now() >= deadline {
            return Err(SessionError::ConnectTimeout);
        }
        match link.read(&mut buf) {
            Ok(0) => return Err(SessionError::Disconnected),
            Ok(n) => {
                for line in acc.push(&buf[..n]) {
                    let l = String::from_utf8_lossy(&line);
                    if l.contains("CONNECT") {
                        return Ok(());
                    }
                    if ["BUSY", "NO CARRIER", "NO ANSWER", "ERROR", "NO DIALTONE"]
                        .iter()
                        .any(|k| l.contains(k))
                    {
                        return Err(SessionError::DialFailed(l.trim().to_string()));
                    }
                }
            }
            Err(ref e) if is_timeout(e) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

/// Spusti poziv: `+++` (guard time) pa `ATH`.
///
/// # Errors
/// I/O greška pri pisanju na vezu.
pub fn hangup<S: Write>(link: &mut S) -> Result<(), SessionError> {
    std::thread::sleep(Duration::from_millis(1100));
    link.write_all(b"+++")?;
    std::thread::sleep(Duration::from_millis(1100));
    link.write_all(b"ATH\r")?;
    Ok(())
}

/// Čitaj dok ne stigne cijela `:`…CRLF Modbus ASCII linija.
fn read_modbus_line<S: Read>(link: &mut S, timeout: Duration) -> Result<Vec<u8>, SessionError> {
    let deadline = Instant::now() + timeout;
    let mut acc = LineAccumulator::default();
    let mut buf = [0u8; 256];
    loop {
        if Instant::now() >= deadline {
            return Err(SessionError::ResponseTimeout);
        }
        match link.read(&mut buf) {
            Ok(0) => return Err(SessionError::Disconnected),
            Ok(n) => {
                for line in acc.push(&buf[..n]) {
                    if line.first() == Some(&b':') {
                        return Ok(line);
                    }
                }
            }
            Err(ref e) if is_timeout(e) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_FRAME: &[u8] =
        b":33033E0CC000360020055B0543000000000000000000000CC20CA6000200000000000000000000000\
0051D0524003EFFF1002CFFF000000000FFF3FF75046300CB14\r\n";

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 0.005, "{a} != {b}");
    }

    #[test]
    fn decodes_to_screen_values() {
        let a = decode_ascii(REAL_FRAME).expect("dekodira");
        approx(a.temp_trenutna_c, 32.64);
        approx(a.temp_0100_c, 32.66);
        approx(a.temp_1300_c, 32.38);
        approx(a.gl_svj.napon_v, 13.47);
        approx(a.gl_svj.struja_a, 0.32);
        approx(a.automat.napon_v, 13.71);
        approx(a.automat.struja_a, 0.54);
        approx(a.prosjek_napon_gl_svj_v, 13.09);
        approx(a.prosjek_napon_automat_v, 13.16);
        approx(a.punjenje_gl_svj_a, 0.44);
        approx(a.punjenje_automat_a, 0.62);
        approx(a.potrosnja_gl_svj_a, -0.16);
        approx(a.potrosnja_automat_a, -0.15);
        approx(a.potrosnja_izvor_a, -0.13);
        approx(a.dnevna_potrosnja_a, -1.39);
    }

    #[test]
    fn rejects_bad_lrc() {
        let mut bad = REAL_FRAME.to_vec();
        bad[5] = b'F';
        assert!(matches!(parse_ascii_frame(&bad), Err(DecodeError::LrcMismatch { .. })));
    }

    #[test]
    fn read_request_matches_captured_master() {
        // stari master je slao točno ovo za Prišnjak
        assert_eq!(build_read_holding(PRISNJAK_ADDR, 0, 31), b":33030000001FAB\r\n".to_vec());
    }

    #[test]
    fn clock_write_matches_captured_master() {
        let cs = ClockSet {
            year: 2026, month: 8, day: 7, hour: 10, minute: 59, second: 47, weekday: 5,
        };
        assert_eq!(
            build_clock_write(PRISNJAK_ADDR, &cs),
            b":33100064000912000107EA00080007000A003B002F00050000C4\r\n".to_vec()
        );
    }

    #[test]
    fn line_accumulator_splits_fragmented() {
        let mut acc = LineAccumulator::default();
        assert!(acc.push(b":3303").is_empty());
        let out = acc.push(b"3E00\r\nOK\r\n");
        assert_eq!(out, vec![b":33033E00".to_vec(), b"OK".to_vec()]);
    }
}
