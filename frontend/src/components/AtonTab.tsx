import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { format, parseISO } from 'date-fns';
import { AlertTriangle, BatteryCharging, CheckCircle2, Lightbulb, PhoneCall, Thermometer } from 'lucide-react';
import { getAtonReadings, getLatestAtonReading, pollAtonNow } from '../api/endpoints';
import type { AtonReading, ObjectView } from '../types';
import './AtonTab.css';

/** Formatiraj broj s jedinicom; prazna vrijednost → "—". */
function fmt(v: number | undefined | null, unit: string, digits = 2): string {
  return v == null ? '—' : `${v.toFixed(digits)} ${unit}`;
}

function ts(iso?: string): string {
  if (!iso) return '—';
  try { return format(parseISO(iso), 'dd.MM.yyyy. HH:mm:ss'); } catch { return iso; }
}

/** Minuta od ponoći → HH:MM (RTU šalje prozor noći u minutama). */
function minutaUSat(min?: number): string {
  if (min == null || min < 0) return '—';
  return `${String(Math.floor(min / 60) % 24).padStart(2, '0')}:${String(min % 60).padStart(2, '0')}`;
}

const DOBA_DANA: Record<number, string> = { 0: 'Sumrak', 1: 'Noć', 2: 'Dan' };

type AtonFlag = {
  label: string;
  reg: number;
  mask?: number;
  severity: 'danger' | 'warning';
};

/** Alarmne zastavice prema RTU funkciji CreateReturnStringToCenter. */
const ATON_FLAGS: AtonFlag[] = [
  { label: 'Zahtjev za pozivom centra', reg: 5, severity: 'warning' },
  { label: 'Temperatura izvan granica', reg: 6, severity: 'warning' },
  { label: 'Napon baterije GL. SVJ.', reg: 7, severity: 'danger' },
  { label: 'Napon baterije automata', reg: 8, severity: 'danger' },
  { label: 'Vrata otvorena', reg: 9, severity: 'warning' },
  { label: 'Pogrešna karakteristika bljeska', reg: 13, severity: 'danger' },
  { label: 'Bljesak 2. žarne niti', reg: 14, severity: 'danger' },
  { label: 'Svjetlo na bateriji automata', reg: 15, severity: 'warning' },
  { label: 'Automat na bateriji svjetla', reg: 16, severity: 'warning' },
  { label: 'Pregorena žarulja', reg: 17, mask: 0b001, severity: 'danger' },
  { label: 'Ne radi po noći', reg: 17, mask: 0b010, severity: 'danger' },
  { label: 'Greška fotoćelije', reg: 17, mask: 0b100, severity: 'danger' },
  { label: 'Pregorena 2. žarna nit', reg: 18, severity: 'danger' },
  { label: 'Svjetlo radi po danu', reg: 25, severity: 'warning' },
];

/** Zasebni alarmni registri su 0/1; registar 17 je bitmaska. */
function isAtonFlagActive(regs: number[] | undefined, flag: AtonFlag): boolean {
  const value = regs?.[flag.reg] ?? 0;
  return flag.mask == null ? value !== 0 : (value & flag.mask) !== 0;
}

/** Dva kanala kroz cijelu mapu: glavno svjetlo i automat. */
function ChannelCard({
  title, napon, struja, prosjekNapon, punjenje, potrosnja,
}: {
  title: string;
  napon?: number;
  struja?: number;
  prosjekNapon?: number;
  punjenje?: number;
  potrosnja?: number;
}) {
  return (
    <div className="aton-card">
      <div className="aton-card-head">
        <BatteryCharging size={14} />
        <h4>{title}</h4>
      </div>
      <div className="aton-big">{fmt(napon, 'V')}</div>
      <div className="aton-rows">
        <div><span>Struja</span><b>{fmt(struja, 'A')}</b></div>
        <div><span>Prosjek napona (dnevni)</span><b>{fmt(prosjekNapon, 'V')}</b></div>
        <div><span>Struja punjenja</span><b>{fmt(punjenje, 'A')}</b></div>
        <div><span>Struja potrošnje</span><b>{fmt(potrosnja, 'A')}</b></div>
      </div>
    </div>
  );
}

export default function AtonTab({ obj, canControl }: { obj: ObjectView; canControl: boolean }) {
  const qc = useQueryClient();
  const [polling, setPolling] = useState(false);
  const [pollMsg, setPollMsg] = useState<string | null>(null);
  const [showRegs, setShowRegs] = useState(false);

  const { data: latest, isLoading } = useQuery({
    queryKey: ['aton-latest', obj.id],
    queryFn: () => getLatestAtonReading(obj.id),
    refetchInterval: 60_000,
  });

  const { data: history } = useQuery({
    queryKey: ['aton-readings', obj.id],
    queryFn: () => getAtonReadings(obj.id, { limit: 50 }),
  });

  const flags = ATON_FLAGS.map((flag) => ({
    ...flag,
    active: isAtonFlagActive(latest?.regs, flag),
  }));
  const activeFlagCount = flags.filter((flag) => flag.active).length;

  const handlePoll = async () => {
    setPolling(true);
    setPollMsg(null);
    try {
      const r = await pollAtonNow(obj.id);
      setPollMsg(r.success
        ? `Poziv uspješan — ${fmt(r.gl_svj_napon_v, 'V')}, ${fmt(r.temperatura_c, '°C', 1)}`
        : `Poziv neuspješan: ${r.error ?? 'nepoznata greška'}`);
      await qc.invalidateQueries({ queryKey: ['aton-latest', obj.id] });
      await qc.invalidateQueries({ queryKey: ['aton-readings', obj.id] });
    } catch (e) {
      setPollMsg(e instanceof Error ? e.message : 'Poziv nije uspio');
    } finally {
      setPolling(false);
    }
  };

  return (
    <div className="aton-tab">
      {/* ── Veza ── */}
      <div className="aton-conn card">
        <div className="aton-conn-info">
          <div><span>snopsy_r</span><code>{obj.aton_snopsy_endpoint ?? '—'}</code></div>
          <div><span>GSM broj (podatkovni)</span><code>{obj.aton_number ?? '—'}</code></div>
          <div><span>ID oznaka</span><code>{obj.aton_addr ?? '—'}</code></div>
          <div><span>Registara</span><code>{obj.aton_reg_count}</code></div>
          <div><span>Program</span><code>csd_verzija · kat. {obj.aton_category}</code></div>
          <div><span>Sinkr. sata</span><code>{obj.aton_sync_clock ? 'da' : 'ne'}</code></div>
          <div><span>Zadnje očitanje</span><code>{ts(latest?.recorded_at)}</code></div>
        </div>
        {canControl && (
          <button className="btn btn-secondary" onClick={handlePoll} disabled={polling}>
            <PhoneCall size={14} className={polling ? 'spin' : undefined} />
            {polling ? 'Poziv u tijeku…' : 'Prozovi sada'}
          </button>
        )}
      </div>
      {pollMsg && <div className="aton-poll-msg">{pollMsg}</div>}

      {isLoading && <div className="aton-empty">Učitavanje…</div>}
      {!isLoading && !latest && (
        <div className="aton-empty">
          Još nema očitanja. CSD poziv traje ~10-20 s — pričekaj sljedeći interval
          prozivanja ili pokreni poziv ručno.
        </div>
      )}

      {latest && (
        <>
          <div className="aton-grid">
            <div className="aton-card">
              <div className="aton-card-head">
                <Thermometer size={14} />
                <h4>Temperatura</h4>
              </div>
              <div className="aton-big">{fmt(latest.temp_trenutna_c, '°C', 1)}</div>
              <div className="aton-rows">
                <div><span>U 01:00</span><b>{fmt(latest.temp_0100_c, '°C', 1)}</b></div>
                <div><span>U 13:00</span><b>{fmt(latest.temp_1300_c, '°C', 1)}</b></div>
              </div>
            </div>

            <ChannelCard
              title="Baterija — GL. SVJ."
              napon={latest.gl_svj_napon_v}
              struja={latest.gl_svj_struja_a}
              prosjekNapon={latest.prosjek_napon_gl_svj_v}
              punjenje={latest.punjenje_gl_svj_a}
              potrosnja={latest.potrosnja_gl_svj_a}
            />

            <ChannelCard
              title="Baterija — AUTOMAT"
              napon={latest.automat_napon_v}
              struja={latest.automat_struja_a}
              prosjekNapon={latest.prosjek_napon_automat_v}
              punjenje={latest.punjenje_automat_a}
              potrosnja={latest.potrosnja_automat_a}
            />

            <div className="aton-card">
              <div className="aton-card-head">
                <Lightbulb size={14} />
                <h4>Izvor svjetla</h4>
              </div>
              <div className="aton-big">{fmt(latest.struja_led_a, 'A')}</div>
              <div className="aton-rows">
                <div><span>Doba dana</span><b>{latest.doba_dana != null ? (DOBA_DANA[latest.doba_dana] ?? `?${latest.doba_dana}`) : '—'}</b></div>
                <div><span>Noć traje</span><b>{minutaUSat(latest.pocetak_noci_min)} – {minutaUSat(latest.kraj_noci_min)}</b></div>
                <div><span>Dnevni prosjek potrošnje</span><b>{fmt(latest.potrosnja_izvor_a, 'A')}</b></div>
                <div><span>Dnevna potrošnja</span><b>{fmt(latest.dnevna_potrosnja_a, 'Ah')}</b></div>
              </div>
            </div>
          </div>

          {/* ── Alarmi i bitovna stanja ── */}
          <div className={`aton-flags card ${activeFlagCount > 0 ? 'aton-flags-active' : ''}`}>
            <div className="aton-flags-head">
              <div>
                {activeFlagCount > 0
                  ? <AlertTriangle size={16} />
                  : <CheckCircle2 size={16} />}
                <h3>Alarmi i stanja</h3>
              </div>
              <strong>{activeFlagCount > 0 ? `${activeFlagCount} aktivno` : 'Sve uredno'}</strong>
            </div>
            <div className="aton-flags-grid">
              {flags.map((flag) => (
                <div
                  key={`${flag.reg}-${flag.mask ?? 0}`}
                  className={`aton-flag ${flag.active ? `aton-flag-${flag.severity}` : ''}`}
                >
                  <span className="aton-flag-dot" aria-hidden="true" />
                  <span>{flag.label}</span>
                  <b>{flag.active ? 'AKTIVNO' : 'U redu'}</b>
                </div>
              ))}
            </div>
          </div>

          {/* ── Sirovi registri ── */}
          <div className="aton-regs card">
            <button className="aton-regs-toggle" onClick={() => setShowRegs((v) => !v)}>
              {showRegs ? 'Sakrij' : 'Prikaži'} sirove registre ({latest.regs?.length ?? 0})
            </button>
            {showRegs && (
              <>
                <p className="aton-regs-note">
                  Mapa je verificirana prema izvornom kodu RTU-a. Sirovi registri se
                  i dalje čuvaju uz svako očitanje — služe za provjeru i za kategorije
                  kojima mapa još nije poznata. Druga vrijednost je sirovi registar,
                  treća je <code>i16 ÷ 100</code> (vrijedi samo za analogne kanale).
                </p>
                <div className="aton-regs-grid">
                  {(latest.regs ?? []).map((r, i) => (
                    <div key={i} className="aton-reg">
                      <span>{i}</span>
                      <b>{r}</b>
                      <em>{((r << 16 >> 16) / 100).toFixed(2)}</em>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        </>
      )}

      {/* ── Povijest ── */}
      {history && history.length > 0 && (
        <div className="aton-history card">
          <h3>Zadnja očitanja</h3>
          <div className="aton-table-wrap">
            <table className="aton-table">
              <thead>
                <tr>
                  <th>Vrijeme</th>
                  <th>Temp.</th>
                  <th>GL.SVJ. napon</th>
                  <th>GL.SVJ. struja</th>
                  <th>AUTOMAT napon</th>
                  <th>AUTOMAT struja</th>
                  <th>Struja izvora</th>
                  <th>Dnevna potrošnja</th>
                </tr>
              </thead>
              <tbody>
                {history.map((r: AtonReading) => (
                  <tr key={r.id ?? r.recorded_at}>
                    <td>{ts(r.recorded_at)}</td>
                    <td>{fmt(r.temp_trenutna_c, '°C', 1)}</td>
                    <td>{fmt(r.gl_svj_napon_v, 'V')}</td>
                    <td>{fmt(r.gl_svj_struja_a, 'A')}</td>
                    <td>{fmt(r.automat_napon_v, 'V')}</td>
                    <td>{fmt(r.automat_struja_a, 'A')}</td>
                    <td>{fmt(r.struja_led_a, 'A')}</td>
                    <td>{fmt(r.dnevna_potrosnja_a, 'Ah')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

