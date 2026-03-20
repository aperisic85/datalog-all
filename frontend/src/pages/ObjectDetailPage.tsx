import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { MapContainer, TileLayer, CircleMarker, Circle, Popup } from 'react-leaflet';
import {
  getObject,
  getLatestMeasurement,
  getMeasurements10min,
  getMeasurements1h,
  getActiveAlarms,
  getEventLogs,
  pollObject,
} from '../api/endpoints';
import 'leaflet/dist/leaflet.css';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
  Legend,
} from 'recharts';
import { format, parseISO, subHours, subDays } from 'date-fns';
import {
  ArrowLeft,
  Battery,
  Zap,
  Thermometer,
  Wifi,
  AlertTriangle,
  MapPin,
  Radio,
  Sun,
  RefreshCw,
} from 'lucide-react';
import './ObjectDetailPage.css';

type Tab = 'overview' | 'charts' | 'alarms' | 'events';
type Range = '6h' | '24h' | '7d';

function MetricCard({
  icon,
  label,
  value,
  unit,
  color,
}: {
  icon: React.ReactNode;
  label: string;
  value?: number | null;
  unit?: string;
  color?: string;
}) {
  return (
    <div className="metric-card card">
      <div className="metric-icon" style={{ color: color || 'var(--accent)' }}>{icon}</div>
      <div className="metric-label">{label}</div>
      <div className="metric-value">
        {value != null ? (
          <>
            <span>{typeof value === 'number' ? value.toFixed(2) : value}</span>
            {unit && <span className="metric-unit">{unit}</span>}
          </>
        ) : (
          <span className="metric-na">N/A</span>
        )}
      </div>
    </div>
  );
}

const ALARM_LABELS: Record<string, string> = {
  alarm_datalogger_high_temp: 'Datalogger visoka temp.',
  alarm_datalogger_high_voltage: 'Datalogger visoki napon',
  alarm_datalogger_other_error: 'Datalogger ostala greška',
  alarm_battery_voltage_low: 'Baterija nizak napon',
  alarm_battery_voltage_flat: 'Baterija prazna',
  alarm_battery_other_error: 'Baterija ostala greška',
  alarm_garmin_comm_failed: 'Garmin komunikacija pala',
  alarm_garmin_other_error: 'Garmin ostala greška',
  alarm_station_out_of_radius: 'Stanica van radijusa',
  alarm_lantern_night_light_off: 'Fenjer noću ugašen',
  alarm_lantern_day_light_on: 'Fenjer danju upaljen',
  alarm_lantern_comm_failed: 'Fenjer komunikacija pala',
  alarm_lantern_other_error: 'Fenjer ostala greška',
  alarm_modem_network_error: 'Modem mrežna greška',
  alarm_modem_other_error: 'Modem ostala greška',
  alarm_station_other_error: 'Stanica ostala greška',
};

const LOG_LEVELS: Record<number, { label: string; cls: string }> = {
  1: { label: 'Debug', cls: 'badge-neutral' },
  2: { label: 'Info', cls: 'badge-neutral' },
  3: { label: 'Upozorenje', cls: 'badge-warning' },
  4: { label: 'Greška', cls: 'badge-danger' },
  5: { label: 'Kritično', cls: 'badge-danger' },
};

export default function ObjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const qc = useQueryClient();
  const [tab, setTab] = useState<Tab>('overview');
  const [range, setRange] = useState<Range>('24h');
  const [polling, setPolling] = useState(false);
  const [pollResult, setPollResult] = useState<string | null>(null);

  const { data: obj, isLoading: loadingObj } = useQuery({
    queryKey: ['object', id],
    queryFn: () => getObject(id!),
    enabled: !!id,
  });

  const { data: latest } = useQuery({
    queryKey: ['latest', id],
    queryFn: () => getLatestMeasurement(id!),
    enabled: !!id,
    refetchInterval: 60_000,
  });

  const { data: recentPositions } = useQuery({
    queryKey: ['positions', id],
    queryFn: () => getMeasurements10min(id!, { limit: 3 }),
    enabled: !!id,
    refetchInterval: 60_000,
  });

  const rangeParam = {
    '6h': { from: subHours(new Date(), 6).toISOString(), limit: 500 },
    '24h': { from: subHours(new Date(), 24).toISOString(), limit: 500 },
    '7d': { from: subDays(new Date(), 7).toISOString(), limit: 500 },
  }[range];

  const { data: measurements10min, isLoading: loadingM } = useQuery({
    queryKey: ['measurements-10min', id, range],
    queryFn: () => getMeasurements10min(id!, rangeParam),
    enabled: !!id && tab === 'charts',
  });

  const { data: measurements1h } = useQuery({
    queryKey: ['measurements-1h', id, range],
    queryFn: () => getMeasurements1h(id!, rangeParam),
    enabled: !!id && tab === 'charts' && range === '7d',
  });

  const { data: activeAlarms, isLoading: loadingAlarms } = useQuery({
    queryKey: ['alarms-active', id],
    queryFn: () => getActiveAlarms(id!),
    enabled: !!id && tab === 'alarms',
    refetchInterval: 60_000,
  });

  const { data: events, isLoading: loadingEvents } = useQuery({
    queryKey: ['events', id],
    queryFn: () => getEventLogs(id!, { limit: 100 }),
    enabled: !!id && tab === 'events',
  });

  const handlePoll = async () => {
    if (!id) return;
    setPolling(true);
    setPollResult(null);
    try {
      const res = await pollObject(id);
      const total = res.results.reduce((s, r) => s + (r.records ?? 0), 0);
      const errors = res.results.filter((r) => r.error);
      if (errors.length > 0) {
        setPollResult(`Greška: ${errors.map((e) => e.error).join(', ')}`);
      } else {
        setPollResult(`Dohvaćeno ${total} novih zapisa`);
        qc.invalidateQueries({ queryKey: ['latest', id] });
        qc.invalidateQueries({ queryKey: ['measurements-10min', id] });
      }
    } catch {
      setPollResult('Greška — datalogger nije dostupan');
    } finally {
      setPolling(false);
    }
  };

  if (loadingObj) return <div className="page-spinner"><div className="spinner" /></div>;
  if (!obj) return <div className="error-msg">Objekt nije pronađen</div>;

  const chartData = (range === '7d' ? measurements1h : measurements10min)?.map((m) => ({
    ...m,
    time: format(parseISO(m.recorded_at), range === '7d' ? 'dd.MM HH:mm' : 'HH:mm'),
  })) ?? [];

  return (
    <div className="object-detail">
      <div className="detail-header">
        <Link to="/objects" className="back-link">
          <ArrowLeft size={16} /> Objekti
        </Link>
        <div className="detail-title">
          <div className="detail-name-row">
            <Radio size={18} />
            <h2>{obj.name}</h2>
            {obj.alarm_active
              ? <span className="badge badge-danger"><AlertTriangle size={11} /> Alarm</span>
              : <span className="badge badge-success">OK</span>
            }
            {!obj.is_active && <span className="badge badge-neutral">Neaktivan</span>}
          </div>
          <div className="detail-meta">
            <code className="station-id">{obj.station_id}</code>
            {obj.region_name && (
              <span className="region-tag">
                <span className="region-dot" style={{ background: obj.region_color }} />
                {obj.region_name}
              </span>
            )}
            {obj.location_name && (
              <span className="detail-location">
                <MapPin size={12} /> {obj.location_name}
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="detail-tabs">
        {(['overview', 'charts', 'alarms', 'events'] as Tab[]).map((t) => (
          <button
            key={t}
            className={`tab-btn ${tab === t ? 'active' : ''}`}
            onClick={() => setTab(t)}
          >
            {{
              overview: 'Pregled',
              charts: 'Grafovi',
              alarms: 'Alarmi',
              events: 'Log',
            }[t]}
          </button>
        ))}
      </div>

      {tab === 'overview' && (
        <div className="overview-tab">
          <div className="metrics-grid">
            <MetricCard icon={<Battery size={20} />} label="Napon baterije" value={latest?.battery_voltage_avg} unit="V" color="var(--success)" />
            <MetricCard icon={<Battery size={20} />} label="Struja baterije" value={latest?.battery_current_avg} unit="A" />
            <MetricCard icon={<Sun size={20} />} label="Napon solarnog" value={latest?.solar_voltage_avg} unit="V" color="var(--warning)" />
            <MetricCard icon={<Thermometer size={20} />} label="Temp. datalogera" value={latest?.datalogger_temp_avg} unit="°C" color="var(--danger)" />
            <MetricCard icon={<Wifi size={20} />} label="Internet" value={latest?.internet_ok_avg != null ? latest.internet_ok_avg * 100 : null} unit="%" color="var(--accent)" />
            <MetricCard icon={<Zap size={20} />} label="Fenjer aktivan" value={latest?.lantern_light_active_avg != null ? latest.lantern_light_active_avg * 100 : null} unit="%" color="var(--warning)" />
            <MetricCard icon={<Zap size={20} />} label="Struja fenjera" value={latest?.lantern_current_avg} unit="A" />
            <MetricCard icon={<Radio size={20} />} label="Garmin sateliti" value={latest?.garmin_satellites_avg} />
          </div>

          <div className="poll-row">
            {latest?.recorded_at && (
              <span className="last-update">
                Zadnje mjerenje: {format(parseISO(latest.recorded_at), 'dd.MM.yyyy HH:mm:ss')}
              </span>
            )}
            {obj.datalogger_url && (
              <button className="btn-secondary poll-btn" onClick={handlePoll} disabled={polling}>
                <RefreshCw size={14} className={polling ? 'spin' : ''} />
                {polling ? 'Dohvaćam...' : 'Dohvati podatke'}
              </button>
            )}
          </div>
          {pollResult && (
            <div className={`poll-result ${pollResult.startsWith('Greška') ? 'error-msg' : 'success-msg'}`}>
              {pollResult}
            </div>
          )}

          <div className="info-section card" style={{ marginTop: 16 }}>
            <h3>Informacije o objektu</h3>
            <div className="info-grid">
              {obj.type_name && <div><span>Tip:</span> {obj.type_name}</div>}
              {obj.commissioned_at && <div><span>Puštanje u rad:</span> {obj.commissioned_at}</div>}
              {obj.latitude && obj.longitude && (
                <div>
                  <span>Koordinate:</span>{' '}
                  <a
                    href={`https://www.google.com/maps?q=${obj.latitude},${obj.longitude}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {obj.latitude.toFixed(5)}, {obj.longitude.toFixed(5)}
                  </a>
                </div>
              )}
              {obj.allowed_radius_m != null && obj.allowed_radius_m > 0 && (
                <div><span>Dozvoljeni radijus:</span> {obj.allowed_radius_m} m</div>
              )}
              {(obj.allowed_radius_m == null || obj.allowed_radius_m === 0) && (
                <div><span>Tip pozicije:</span> Fiksni objekt</div>
              )}
              <div><span>Polling:</span> {obj.polling_enabled ? `${obj.poll_interval_sec}s` : 'isključen'}</div>
              {obj.description && <div className="info-full"><span>Opis:</span> {obj.description}</div>}
            </div>
          </div>

          {obj.latitude && obj.longitude && (() => {
            const gpsPoints = (recentPositions ?? [])
              .filter((m) => m.garmin_latitude_avg != null && m.garmin_longitude_avg != null)
              .slice(0, 3);
            const posColors = ['#f97316', '#fb923c', '#fdba74'];
            return (
              <div className="location-map-section card" style={{ marginTop: 16 }}>
                <div className="location-map-header">
                  <h3>Lokacija objekta</h3>
                  <div className="location-map-legend">
                    <span className="loc-legend-item"><span className="loc-dot default" />Zadana pozicija</span>
                    {obj.allowed_radius_m != null && obj.allowed_radius_m > 0 && (
                      <span className="loc-legend-item"><span className="loc-dot radius" />Dozvoljeni radijus</span>
                    )}
                    {gpsPoints.length > 0 && (
                      <span className="loc-legend-item"><span className="loc-dot gps" />Zadnje GPS pozicije</span>
                    )}
                  </div>
                </div>
                <div className="location-map-wrap">
                  <MapContainer
                    center={[obj.latitude, obj.longitude]}
                    zoom={15}
                    style={{ height: '100%', width: '100%' }}
                    scrollWheelZoom={false}
                  >
                    <TileLayer
                      attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                      url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
                    />
                    {/* Radius circle */}
                    {obj.allowed_radius_m != null && obj.allowed_radius_m > 0 && (
                      <Circle
                        center={[obj.latitude, obj.longitude]}
                        radius={obj.allowed_radius_m}
                        pathOptions={{ color: '#3b82f6', fillColor: '#3b82f6', fillOpacity: 0.08, weight: 2, dashArray: '6 4' }}
                      />
                    )}
                    {/* Default position */}
                    <CircleMarker
                      center={[obj.latitude, obj.longitude]}
                      radius={9}
                      pathOptions={{ color: '#2563eb', fillColor: '#2563eb', fillOpacity: 0.9, weight: 2 }}
                    >
                      <Popup>
                        <strong>Zadana pozicija</strong><br />
                        {obj.latitude.toFixed(5)}, {obj.longitude.toFixed(5)}
                      </Popup>
                    </CircleMarker>
                    {/* Last 3 GPS positions */}
                    {gpsPoints.map((m, i) => (
                      <CircleMarker
                        key={m.id}
                        center={[m.garmin_latitude_avg!, m.garmin_longitude_avg!]}
                        radius={6}
                        pathOptions={{ color: posColors[i], fillColor: posColors[i], fillOpacity: 0.85, weight: 2 }}
                      >
                        <Popup>
                          <strong>GPS pozicija #{i + 1}</strong><br />
                          {m.garmin_latitude_avg!.toFixed(5)}, {m.garmin_longitude_avg!.toFixed(5)}<br />
                          <span style={{ fontSize: 11, color: '#6b7280' }}>{m.recorded_at.replace('T', ' ').slice(0, 16)}</span>
                        </Popup>
                      </CircleMarker>
                    ))}
                  </MapContainer>
                </div>
              </div>
            );
          })()}
        </div>
      )}

      {tab === 'charts' && (
        <div className="charts-tab">
          <div className="range-selector">
            {(['6h', '24h', '7d'] as Range[]).map((r) => (
              <button
                key={r}
                className={`filter-tab ${range === r ? 'active' : ''}`}
                onClick={() => setRange(r)}
              >
                {r}
              </button>
            ))}
          </div>

          {loadingM ? (
            <div className="page-spinner"><div className="spinner" /></div>
          ) : chartData.length === 0 ? (
            <div className="no-data">Nema podataka za odabrani period</div>
          ) : (
            <div className="charts-grid">
              <div className="chart-card card">
                <h4>Napon baterije (V)</h4>
                <ResponsiveContainer width="100%" height={180}>
                  <LineChart data={chartData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                    <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                    <Line type="monotone" dataKey="battery_voltage_avg" stroke="var(--success)" dot={false} name="Napon (V)" />
                  </LineChart>
                </ResponsiveContainer>
              </div>

              <div className="chart-card card">
                <h4>Struja baterije (A)</h4>
                <ResponsiveContainer width="100%" height={180}>
                  <LineChart data={chartData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                    <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                    <Line type="monotone" dataKey="battery_current_avg" stroke="var(--accent)" dot={false} name="Struja (A)" />
                  </LineChart>
                </ResponsiveContainer>
              </div>

              <div className="chart-card card">
                <h4>Solarna ploča (V)</h4>
                <ResponsiveContainer width="100%" height={180}>
                  <LineChart data={chartData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                    <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                    <Line type="monotone" dataKey="solar_voltage_avg" stroke="var(--warning)" dot={false} name="Solar (V)" />
                  </LineChart>
                </ResponsiveContainer>
              </div>

              <div className="chart-card card">
                <h4>Temperatura datalogera (°C)</h4>
                <ResponsiveContainer width="100%" height={180}>
                  <LineChart data={chartData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                    <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                    <Line type="monotone" dataKey="datalogger_temp_avg" stroke="var(--danger)" dot={false} name="Temp. (°C)" />
                  </LineChart>
                </ResponsiveContainer>
              </div>

              <div className="chart-card card chart-wide">
                <h4>Fenjer</h4>
                <ResponsiveContainer width="100%" height={180}>
                  <LineChart data={chartData}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                    <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                    <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                    <Legend />
                    <Line type="monotone" dataKey="lantern_light_active_avg" stroke="var(--warning)" dot={false} name="Svjetlo aktivno" />
                    <Line type="monotone" dataKey="lantern_current_avg" stroke="var(--accent)" dot={false} name="Struja (A)" />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </div>
          )}
        </div>
      )}

      {tab === 'alarms' && (
        <div className="alarms-tab">
          {loadingAlarms ? (
            <div className="page-spinner"><div className="spinner" /></div>
          ) : (() => {
            const latest = activeAlarms?.[0];
            const activeKeys = latest
              ? Object.keys(ALARM_LABELS).filter((k) => (latest as unknown as Record<string, number>)[k] > 0)
              : [];
            if (!latest || activeKeys.length === 0) {
              return (
                <div className="alarm-ok card">
                  <span className="badge badge-success" style={{ fontSize: 14, padding: '6px 16px' }}>OK — nema aktivnih alarma</span>
                  {latest && (
                    <div style={{ marginTop: 8, fontSize: 12, color: 'var(--text2)' }}>
                      Zadnja provjera: {format(parseISO(latest.recorded_at), 'dd.MM.yyyy HH:mm')}
                    </div>
                  )}
                </div>
              );
            }
            return (
              <div className="alarm-current card">
                <div className="alarm-current-header">
                  <span className="badge badge-danger" style={{ fontSize: 13 }}>
                    <AlertTriangle size={13} /> Aktivni alarmi
                  </span>
                  <span style={{ fontSize: 12, color: 'var(--text2)' }}>
                    od {format(parseISO(latest.recorded_at), 'dd.MM.yyyy HH:mm')}
                  </span>
                </div>
                <div className="alarm-tags" style={{ marginTop: 12 }}>
                  {activeKeys.map((k) => (
                    <div key={k} className="alarm-item">
                      <AlertTriangle size={14} className="alarm-item-icon" />
                      <span>{ALARM_LABELS[k]}</span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })()}
        </div>
      )}

      {tab === 'events' && (
        <div className="events-tab">
          {loadingEvents ? (
            <div className="page-spinner"><div className="spinner" /></div>
          ) : !events?.length ? (
            <div className="no-data">Nema event log zapisa</div>
          ) : (
            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
              <table>
                <thead>
                  <tr>
                    <th>Zabilježeno</th>
                    <th>Nivo</th>
                    <th>Poruka</th>
                  </tr>
                </thead>
                <tbody>
                  {events.map((e) => {
                    const lvl = LOG_LEVELS[e.log_level] || { label: `L${e.log_level}`, cls: 'badge-neutral' };
                    return (
                      <tr key={e.id}>
                        <td style={{ whiteSpace: 'nowrap', fontSize: 12 }}>
                          {format(parseISO(e.recorded_at), 'dd.MM.yyyy HH:mm:ss')}
                        </td>
                        <td><span className={`badge ${lvl.cls}`}>{lvl.label}</span></td>
                        <td style={{ fontSize: 13 }}>{e.log_message}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
