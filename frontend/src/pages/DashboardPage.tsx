import { useQuery } from '@tanstack/react-query';
import { regionSummary } from '../api/endpoints';
import { AlertTriangle, Battery, Zap, Radio, CheckCircle } from 'lucide-react';
import './DashboardPage.css';

function AlarmLevel({ level }: { level?: number | null }) {
  if (!level && level !== 0) return null;
  if (level >= 3) return <span className="badge badge-danger">Kritično</span>;
  if (level >= 2) return <span className="badge badge-warning">Upozorenje</span>;
  return <span className="badge badge-success">OK</span>;
}

export default function DashboardPage() {
  const { data: summaries, isLoading, error } = useQuery({
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
        <h2>Dashboard</h2>
        <span className="text-muted">Pregled sistema u realnom vremenu</span>
      </div>

      <div className="stat-cards">
        <div className="stat-card card">
          <div className="stat-icon stat-icon-blue">
            <Radio size={20} />
          </div>
          <div>
            <div className="stat-value">{total?.objects ?? '—'}</div>
            <div className="stat-label">Ukupno objekata</div>
          </div>
        </div>
        <div className="stat-card card">
          <div className="stat-icon stat-icon-green">
            <CheckCircle size={20} />
          </div>
          <div>
            <div className="stat-value">{total?.active ?? '—'}</div>
            <div className="stat-label">Aktivnih</div>
          </div>
        </div>
        <div className="stat-card card">
          <div className="stat-icon stat-icon-red">
            <AlertTriangle size={20} />
          </div>
          <div>
            <div className="stat-value">{total?.alarms ?? '—'}</div>
            <div className="stat-label">U alarmu</div>
          </div>
        </div>
        <div className="stat-card card">
          <div className="stat-icon stat-icon-yellow">
            <Zap size={20} />
          </div>
          <div>
            <div className="stat-value">{total?.lanterns ?? '—'}</div>
            <div className="stat-label">Fenjeri uključeni</div>
          </div>
        </div>
      </div>

      <h3 style={{ margin: '24px 0 12px' }}>Regije</h3>
      <div className="region-grid">
        {summaries?.map((s) => (
          <div key={s.region_id} className="region-card card">
            <div className="region-header">
              <div
                className="region-color-dot"
                style={{ background: s.region_color || '#666' }}
              />
              <div>
                <div className="region-name">{s.region_name}</div>
                <div className="region-code">{s.region_code}</div>
              </div>
              <div style={{ marginLeft: 'auto' }}>
                <AlarmLevel level={s.worst_alarm_level} />
              </div>
            </div>

            <div className="region-stats">
              <div className="region-stat">
                <Radio size={14} />
                <span>{s.active_objects} / {s.total_objects} aktivnih</span>
              </div>
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
                  <span>{s.lanterns_on_count} fenjera uključeno</span>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
