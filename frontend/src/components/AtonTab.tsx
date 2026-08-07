import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { format, parseISO } from 'date-fns';
import { BatteryCharging, PhoneCall, RefreshCw, Thermometer } from 'lucide-react';
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
          <div><span>Tel. podatkovni</span><code>{obj.aton_number ?? '—'}</code></div>
          <div><span>Modbus adresa</span><code>{obj.aton_addr ?? '—'}</code></div>
          <div><span>Registara</span><code>{obj.aton_reg_count}</code></div>
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
                <RefreshCw size={14} />
                <h4>Dnevni prosjek potrošnje</h4>
              </div>
              <div className="aton-big">{fmt(latest.dnevna_potrosnja_a, 'A')}</div>
              <div className="aton-rows">
                <div><span>Struja potrošnje (izvor svj.)</span><b>{fmt(latest.potrosnja_izvor_a, 'A')}</b></div>
                <div><span>Zaprimljeno</span><b>{ts(latest.received_at)}</b></div>
              </div>
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
                  Mapa alarm/status bitova još nije razriješena — registri se čuvaju
                  uz svako očitanje da se mogu naknadno mapirati bez novog poziva.
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
                    <td>{fmt(r.dnevna_potrosnja_a, 'A')}</td>
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
