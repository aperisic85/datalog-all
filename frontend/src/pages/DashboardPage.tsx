import { useEffect, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { listObjects, regionSummary } from '../api/endpoints';
import { AlertTriangle, Battery, Radio, CheckCircle2, ChevronRight, WifiOff, MapPin } from 'lucide-react';
import './DashboardPage.css';

function AlarmLevel({ level }: { level?: number | null }) {
  if (!level && level !== 0) return null;
  if (level >= 3) return <span className="badge badge-danger">Kritično</span>;
  if (level >= 2) return <span className="badge badge-warning">Upozorenje</span>;
  return <span className="badge badge-success">Uredno</span>;
}

function AnimatedStat({ value }: { value: number | undefined }) {
  const [display, setDisplay] = useState(0);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    if (value == null) return;
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    const start = performance.now();
    const animate = (now: number) => {
      const t = Math.min((now - start) / 650, 1);
      setDisplay(Math.round((1 - Math.pow(1 - t, 3)) * value));
      if (t < 1) rafRef.current = requestAnimationFrame(animate);
    };
    rafRef.current = requestAnimationFrame(animate);
    return () => { if (rafRef.current) cancelAnimationFrame(rafRef.current); };
  }, [value]);

  return <>{display}</>;
}

function LiveStatus({ dataUpdatedAt }: { dataUpdatedAt: number }) {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const update = () => setSeconds(Math.max(0, Math.round((Date.now() - dataUpdatedAt) / 1000)));
    update();
    const id = window.setInterval(update, 1000);
    return () => window.clearInterval(id);
  }, [dataUpdatedAt]);

  return (
    <div className="live-status" title={`Podaci osvježeni prije ${seconds} s`}>
      <span className="live-dot" />
      UŽIVO
      <span className="live-age">{seconds}s</span>
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

  const { data: alarmObjects } = useQuery({
    queryKey: ['dashboard-attention-alarms'],
    queryFn: () => listObjects({ page: 1, page_size: 6, in_alarm: true }),
    refetchInterval: 60_000,
  });

  const { data: recentObjects } = useQuery({
    queryKey: ['dashboard-attention-silent'],
    queryFn: () => listObjects({ page: 1, page_size: 100, active: true }),
    refetchInterval: 60_000,
  });

  if (isLoading) return <div className="page-spinner"><div className="spinner" /></div>;
  if (error) return <div className="error-msg">Greška pri učitavanju operativnog pregleda</div>;

  const total = summaries?.reduce(
    (acc, s) => ({
      objects: acc.objects + (s.total_objects || 0),
      active: acc.active + (s.active_objects || 0),
      alarms: acc.alarms + (s.objects_in_alarm || 0),
      lowBatteries: acc.lowBatteries + (s.battery_low_count || 0) + (s.battery_flat_count || 0),
    }),
    { objects: 0, active: 0, alarms: 0, lowBatteries: 0 }
  );

  const silent = recentObjects?.data.filter((o) => o.is_silent).slice(0, 3) ?? [];
  const attention = [
    ...(alarmObjects?.data ?? []).map((o) => ({ obj: o, kind: 'alarm' as const })),
    ...silent
      .filter((s) => !(alarmObjects?.data ?? []).some((a) => a.id === s.id))
      .map((o) => ({ obj: o, kind: 'silent' as const })),
  ].slice(0, 6);

  const offlineCount = Math.max(0, (total?.objects ?? 0) - (total?.active ?? 0));

  return (
    <div className="dashboard">
      <header className="dashboard-header">
        <div>
          <div className="eyebrow">AtoN Watch</div>
          <h1>Operativni pregled</h1>
          <p>Nadzor navigacijskih objekata i sustava u stvarnom vremenu</p>
        </div>
        {dataUpdatedAt > 0 && <LiveStatus dataUpdatedAt={dataUpdatedAt} />}
      </header>

      <div className="stat-cards">
        <button className="stat-card card" onClick={() => navigate('/objects')}>
          <div className="stat-icon stat-icon-blue"><Radio size={21} /></div>
          <div className="stat-copy">
            <div className="stat-value"><AnimatedStat value={total?.objects} /></div>
            <div className="stat-label">Objekata</div>
            <div className="stat-detail">{total?.active ?? 0} operativno</div>
          </div>
        </button>

        <button className="stat-card card stat-attention" onClick={() => navigate('/alarms')}>
          <div className="stat-icon stat-icon-red"><AlertTriangle size={21} /></div>
          <div className="stat-copy">
            <div className="stat-value"><AnimatedStat value={total?.alarms} /></div>
            <div className="stat-label">Aktivnih alarma</div>
            <div className="stat-detail danger-copy">{(total?.alarms ?? 0) > 0 ? 'Zahtijeva provjeru' : 'Nema aktivnih alarma'}</div>
          </div>
        </button>

        <button className="stat-card card" onClick={() => navigate('/objects?active=false')}>
          <div className="stat-icon stat-icon-slate"><WifiOff size={21} /></div>
          <div className="stat-copy">
            <div className="stat-value"><AnimatedStat value={offlineCount} /></div>
            <div className="stat-label">Izvan pogona</div>
            <div className="stat-detail">Neaktivni objekti</div>
          </div>
        </button>

        <button className="stat-card card" onClick={() => navigate('/objects')}>
          <div className="stat-icon stat-icon-yellow"><Battery size={21} /></div>
          <div className="stat-copy">
            <div className="stat-value"><AnimatedStat value={total?.lowBatteries} /></div>
            <div className="stat-label">Baterije</div>
            <div className="stat-detail">Zahtijevaju pažnju</div>
          </div>
        </button>
      </div>

      <section className="dashboard-section attention-panel card">
        <div className="section-title-row">
          <div>
            <span className="section-kicker">Prioritet</span>
            <h2>Zahtijeva pažnju</h2>
          </div>
          <button className="text-action" onClick={() => navigate('/alarms')}>Prikaži sve <ChevronRight size={15} /></button>
        </div>

        {attention.length === 0 ? (
          <div className="all-clear">
            <CheckCircle2 size={20} />
            <div><strong>Nema otvorenih operativnih problema.</strong><span>Svi dohvaćeni objekti su bez aktivnog alarma i komunikacijskih upozorenja.</span></div>
          </div>
        ) : (
          <div className="attention-list">
            {attention.map(({ obj, kind }) => (
              <button key={obj.id} className="attention-row" onClick={() => navigate(`/objects/${obj.id}`)}>
                <span className={`attention-dot ${kind === 'alarm' ? 'attention-dot-danger' : 'attention-dot-offline'}`} />
                <div className="attention-main">
                  <strong>{obj.name}</strong>
                  <span>{obj.station_id}{obj.region_name ? ` · ${obj.region_name}` : ''}</span>
                </div>
                <div className="attention-reason">
                  <strong>{kind === 'alarm' ? `${obj.alarm_count} aktivnih alarma` : 'Nema komunikacije'}</strong>
                  <span>{obj.location_name || 'Lokacija nije unesena'}</span>
                </div>
                <span className={`attention-badge ${kind === 'alarm' ? 'critical' : 'offline'}`}>
                  {kind === 'alarm' ? 'ALARM' : 'OFFLINE'}
                </span>
                <ChevronRight size={17} className="attention-chevron" />
              </button>
            ))}
          </div>
        )}
      </section>

      <div className="regions-header">
        <div>
          <span className="section-kicker">Područja nadzora</span>
          <h2>Regije</h2>
        </div>
        {summaries && <span className="region-count-badge">{summaries.length}</span>}
      </div>

      <div className="region-grid">
        {summaries?.map((s) => {
          const availability = (s.total_objects ?? 0) > 0
            ? Math.round(((s.active_objects ?? 0) / (s.total_objects ?? 1)) * 100)
            : 0;
          return (
            <div key={s.region_id} className="region-card card">
              <div className="region-header">
                <div className="region-color-dot" style={{ background: s.region_color || '#64748b' }} />
                <div>
                  <div className="region-name">{s.region_name}</div>
                  <div className="region-code">{s.region_code}</div>
                </div>
                <div className="region-level"><AlarmLevel level={s.worst_alarm_level} /></div>
              </div>

              <div className="region-availability">
                <div>
                  <strong>{s.active_objects} / {s.total_objects}</strong>
                  <span>operativno</span>
                </div>
                <span>{availability}%</span>
              </div>
              <div className="region-progress">
                <div className="region-progress-fill" style={{ width: `${availability}%` }} />
              </div>

              <div className="region-signals">
                <span className={s.objects_in_alarm ? 'signal danger' : 'signal'}>
                  <AlertTriangle size={13} /> {s.objects_in_alarm ?? 0} alarma
                </span>
                <span className={(s.battery_low_count ?? 0) + (s.battery_flat_count ?? 0) > 0 ? 'signal warning' : 'signal'}>
                  <Battery size={13} /> {(s.battery_low_count ?? 0) + (s.battery_flat_count ?? 0)} baterija
                </span>
                <span className="signal"><MapPin size={13} /> {s.total_objects ?? 0} objekata</span>
              </div>

              <div className="region-actions">
                <button onClick={() => navigate(`/objects?region_id=${s.region_id}`)}>Objekti <ChevronRight size={13} /></button>
                {(s.objects_in_alarm ?? 0) > 0 && (
                  <button className="alarm-action" onClick={() => navigate(`/alarms?region_id=${s.region_id}`)}>Alarmi <ChevronRight size={13} /></button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
