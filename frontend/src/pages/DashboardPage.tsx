import { useEffect, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { regionSummary, getEnergyRisks } from '../api/endpoints';
import { AlertTriangle, Battery, Zap, Radio, CheckCircle, ChevronRight, CalendarClock } from 'lucide-react';
import { format, parseISO } from 'date-fns';
import { hr } from 'date-fns/locale';
import './DashboardPage.css';

function AlarmLevel({ level }: { level?: number | null }) {
  if (!level && level !== 0) return null;
  if (level >= 3) return <span className="badge badge-danger">Kritično</span>;
  if (level >= 2) return <span className="badge badge-warning">Upozorenje</span>;
  return <span className="badge badge-success">OK</span>;
}

function AnimatedStat({ value }: { value: number | undefined }) {
  const [display, setDisplay] = useState(0);
  const rafRef = useRef<number | null>(null);
  useEffect(() => {
    if (value == null) return;
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    const start = performance.now();
    const end = value;
    const animate = (now: number) => {
      const t = Math.min((now - start) / 750, 1);
      const eased = 1 - Math.pow(1 - t, 3);
      setDisplay(Math.round(eased * end));
      if (t < 1) rafRef.current = requestAnimationFrame(animate);
    };
    rafRef.current = requestAnimationFrame(animate);
    return () => { if (rafRef.current) cancelAnimationFrame(rafRef.current); };
  }, [value]);
  return <>{display}</>;
}

function LiveRing({ intervalMs, dataUpdatedAt }: { intervalMs: number; dataUpdatedAt: number }) {
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    const tick = () => setElapsed(Date.now() - dataUpdatedAt);
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [dataUpdatedAt]);
  const progress = Math.min(elapsed / intervalMs, 1);
  const r = 9;
  const circ = 2 * Math.PI * r;
  const remaining = Math.max(0, Math.round((intervalMs - elapsed) / 1000));
  return (
    <div className="live-ring-wrap" title={`Osvježava se za ${remaining}s`}>
      <svg width="26" height="26" style={{ transform: 'rotate(-90deg)' }}>
        <circle cx="13" cy="13" r={r} fill="none" stroke="var(--border)" strokeWidth="2" />
        <circle cx="13" cy="13" r={r} fill="none" stroke="var(--success)" strokeWidth="2"
          strokeDasharray={circ} strokeDashoffset={circ * (1 - progress)} strokeLinecap="round" />
      </svg>
      <span className="live-label">LIVE</span>
    </div>
  );
}

/**
 * Stanice kojima energetska prognoza predviđa pad napona u sljedećih 7 dana.
 * Prikazuje se samo kad ima rizika — inače kartica ne zauzima prostor.
 */
function EnergyRiskCard() {
  const navigate = useNavigate();
  const { data, isLoading } = useQuery({
    queryKey: ['energy-risks'],
    queryFn: getEnergyRisks,
    refetchInterval: 15 * 60_000,
    retry: 1,
  });

  if (isLoading || !data?.length) return null;

  const fmtDate = (iso?: string) => {
    if (!iso) return null;
    try {
      return format(parseISO(iso), 'EEEE dd.MM.', { locale: hr });
    } catch {
      return iso;
    }
  };

  return (
    <div className="energy-risk-card card">
      <div className="energy-risk-header">
        <CalendarClock size={15} style={{ color: 'var(--warning)' }} />
        <h3>Energetski rizik — sljedećih 7 dana</h3>
        <span className="energy-risk-count">{data.length}</span>
      </div>
      <div className="energy-risk-list">
        {data.map((r) => {
          const critical = r.status === 'critical';
          const when = fmtDate(critical ? r.first_critical_date : r.first_warning_date);
          return (
            <button
              key={r.object_id}
              className={`energy-risk-row ${critical ? 'critical' : 'warning'}`}
              onClick={() => navigate(`/objects/${r.object_id}`)}
            >
              <span className="energy-risk-dot" style={{ background: r.region_color || 'var(--text3)' }} />
              <div className="energy-risk-body">
                <div className="energy-risk-name">
                  {r.object_name}
                  <span className="energy-risk-region">{r.region_name}</span>
                </div>
                <div className="energy-risk-msg">{r.message}</div>
              </div>
              <div className="energy-risk-meta">
                {when && <span className={critical ? 'energy-risk-when-crit' : 'energy-risk-when'}>{when}</span>}
                {r.min_soc_pct != null && (
                  <span className="energy-risk-soc">min. SOC {Math.round(r.min_soc_pct)}%</span>
                )}
              </div>
              <ChevronRight size={14} style={{ color: 'var(--text3)', flexShrink: 0 }} />
            </button>
          );
        })}
      </div>
    </div>
  );
}

export default function DashboardPage() {
  const navigate = useNavigate();
  const { data: summaries, isLoading, error, dataUpdatedAt } = useQuery({
    queryKey: ['region-summary'],
    queryFn: regionSummary,
    refetchInterval: 60_000,
  });

  if (isLoading) return <div className="page-spinner"><div className="spinner" /></div>;
  if (error) return <div className="error-msg">Greška pri učitavanju dashboarda</div>;

  const total = summaries?.reduce(
    (acc, s) => ({
      objects: acc.objects + (s.total_objects || 0),
      active: acc.active + (s.active_objects || 0),
      alarms: acc.alarms + (s.objects_in_alarm || 0),
      lanterns: acc.lanterns + (s.lanterns_on_count || 0),
    }),
    { objects: 0, active: 0, alarms: 0, lanterns: 0 }
  );

  return (
    <div className="dashboard">
      <div className="page-header">
        <div>
          <h2>Dashboard</h2>
          <span className="text-muted">Pregled sistema u realnom vremenu</span>
        </div>
        {dataUpdatedAt > 0 && <LiveRing intervalMs={60_000} dataUpdatedAt={dataUpdatedAt} />}
      </div>

      <div className="stat-cards">
        <button className="stat-card stat-card-blue card stat-card-btn" onClick={() => navigate('/objects')}>
          <div className="stat-icon stat-icon-blue"><Radio size={20} /></div>
          <div>
            <div className="stat-value"><AnimatedStat value={total?.objects} /></div>
            <div className="stat-label">Ukupno objekata</div>
          </div>
        </button>
        <button className="stat-card stat-card-green card stat-card-btn" onClick={() => navigate('/objects?active=true')}>
          <div className="stat-icon stat-icon-green"><CheckCircle size={20} /></div>
          <div>
            <div className="stat-value"><AnimatedStat value={total?.active} /></div>
            <div className="stat-label">Aktivnih</div>
          </div>
        </button>
        <button className="stat-card stat-card-red card stat-card-btn" onClick={() => navigate('/alarms')}>
          <div className="stat-icon stat-icon-red"><AlertTriangle size={20} /></div>
          <div>
            <div className="stat-value"><AnimatedStat value={total?.alarms} /></div>
            <div className="stat-label">U alarmu</div>
          </div>
        </button>
        <button className="stat-card stat-card-yellow card stat-card-btn" onClick={() => navigate('/objects')}>
          <div className="stat-icon stat-icon-yellow"><Zap size={20} /></div>
          <div>
            <div className="stat-value"><AnimatedStat value={total?.lanterns} /></div>
            <div className="stat-label">Svjetla uključena</div>
          </div>
        </button>
      </div>

      <EnergyRiskCard />

      <div className="regions-header">
        <h3>Regije</h3>
        {summaries && <span className="region-count-badge">{summaries.length}</span>}
      </div>
      <div className="region-grid">
        {summaries?.map((s) => (
          <div key={s.region_id} className="region-card card">
            <div className="region-header">
              <div className="region-color-dot" style={{ background: s.region_color || '#666' }} />
              <div>
                <div className="region-name">{s.region_name}</div>
                <div className="region-code">{s.region_code}</div>
              </div>
              <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 8 }}>
                <AlarmLevel level={s.worst_alarm_level} />
              </div>
            </div>

            <div className="region-stats">
              <div className="region-stat">
                <Radio size={14} />
                <span>{s.active_objects} / {s.total_objects} aktivnih</span>
              </div>
              {(s.total_objects ?? 0) > 0 && (
                <div className="region-progress">
                  <div
                    className="region-progress-fill"
                    style={{ width: `${Math.round(((s.active_objects ?? 0) / (s.total_objects ?? 1)) * 100)}%` }}
                  />
                </div>
              )}
              {(s.objects_in_alarm ?? 0) > 0 && (
                <div className="region-stat region-stat-alarm">
                  <AlertTriangle size={14} />
                  <span>{s.objects_in_alarm} u alarmu</span>
                </div>
              )}
              {s.avg_battery_voltage != null && (
                <div className="region-stat">
                  <Battery size={14} />
                  <span>Avg. baterija: {s.avg_battery_voltage.toFixed(2)} V</span>
                </div>
              )}
              {(s.battery_flat_count ?? 0) > 0 && (
                <div className="region-stat region-stat-danger">
                  <Battery size={14} />
                  <span>{s.battery_flat_count} praznih baterija</span>
                </div>
              )}
              {(s.battery_low_count ?? 0) > 0 && (
                <div className="region-stat region-stat-warn">
                  <Battery size={14} />
                  <span>{s.battery_low_count} slabih baterija</span>
                </div>
              )}
              {s.lanterns_on_count != null && (
                <div className="region-stat">
                  <Zap size={14} />
                  <span>{s.lanterns_on_count} svjetala uključeno</span>
                </div>
              )}
            </div>

            <div className="region-actions">
              <button className="region-action-btn" onClick={() => navigate(`/objects?region_id=${s.region_id}`)}>
                <Radio size={13} /> Objekti <ChevronRight size={13} />
              </button>
              {(s.objects_in_alarm ?? 0) > 0 && (
                <button className="region-action-btn region-action-alarm" onClick={() => navigate(`/alarms?region_id=${s.region_id}`)}>
                  <AlertTriangle size={13} /> Alarmi <ChevronRight size={13} />
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
