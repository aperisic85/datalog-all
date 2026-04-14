import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { MapContainer, TileLayer, CircleMarker, Circle, Popup, Polyline } from 'react-leaflet';
import {
  getObject,
  getLatestMeasurement,
  getMeasurements10min,
  getMeasurements1h,
  getActiveAlarms,
  getEventLogs,
  getBatteryPrediction,
  pollObject,
  updateObject,
  listRegions,
  listStationTypes,
} from '../api/endpoints';
import { useAuth } from '../context/AuthContext';
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
  Pencil,
  X,
  TrendingUp,
  TrendingDown,
  Eye,
  Wind,
  Cpu,
} from 'lucide-react';
import './ObjectDetailPage.css';
import './ObjectsPage.css';
import AlarmHeatmapTab from '../components/AlarmHeatmapTab';

type Tab = 'overview' | 'charts' | 'alarms' | 'events' | 'heatmap';
type Range = '6h' | '24h' | '7d';
type DriftRange = '1h' | '6h' | '24h' | '7d';

function haversineDistance(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const R = 6371000;
  const φ1 = (lat1 * Math.PI) / 180;
  const φ2 = (lat2 * Math.PI) / 180;
  const Δφ = ((lat2 - lat1) * Math.PI) / 180;
  const Δλ = ((lon2 - lon1) * Math.PI) / 180;
  const a = Math.sin(Δφ / 2) ** 2 + Math.cos(φ1) * Math.cos(φ2) * Math.sin(Δλ / 2) ** 2;
  return R * 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
}

function MetricCard({
  icon,
  label,
  value,
  unit,
  color,
  prev,
}: {
  icon: React.ReactNode;
  label: string;
  value?: number | null;
  unit?: string;
  color?: string;
  prev?: number | null;
}) {
  let trend: 'up' | 'down' | null = null;
  if (value != null && prev != null) {
    const threshold = Math.max(Math.abs(prev), 0.01) * 0.02; // 2% relative threshold
    if (value - prev > threshold) trend = 'up';
    else if (prev - value > threshold) trend = 'down';
  }

  return (
    <div className="metric-card card">
      <div className="metric-icon" style={{ color: color || 'var(--accent)' }}>{icon}</div>
      <div className="metric-label">{label}</div>
      <div className="metric-value">
        {value != null ? (
          <>
            <span>{typeof value === 'number' ? value.toFixed(2) : value}</span>
            {unit && <span className="metric-unit">{unit}</span>}
            {trend === 'up' && <TrendingUp size={13} style={{ color: 'var(--success)', marginLeft: 3 }} />}
            {trend === 'down' && <TrendingDown size={13} style={{ color: 'var(--danger)', marginLeft: 3 }} />}
          </>
        ) : (
          <span className="metric-na">N/A</span>
        )}
      </div>
    </div>
  );
}

// ─── Battery section (napon + struja + predikcija) ──────────────────────────

const TREND_CONFIG: Record<string, { label: string; color: string }> = {
  stable:            { label: 'Stabilan',            color: 'var(--success)' },
  charging:          { label: 'Puni se',              color: 'var(--success)' },
  degrading:         { label: 'Pada',                 color: 'var(--warning)' },
  warning:           { label: 'Upozorenje',           color: 'var(--warning)' },
  critical:          { label: 'KRITIČNO',             color: 'var(--danger)'  },
  insufficient_data: { label: 'Nedovoljno podataka',  color: 'var(--text2)'   },
};

function formatDays(days: number): string {
  if (days < 1) {
    const h = Math.round(days * 24);
    return `~${h} sat${h === 1 ? '' : 'a'}`;
  }
  return `~${days.toFixed(1)} dan${days < 2 ? '' : 'a'}`;
}

function BatterySection({
  objectId,
  voltage,
  current,
  prevVoltage,
  prevCurrent,
}: {
  objectId: string;
  voltage?: number | null;
  current?: number | null;
  prevVoltage?: number | null;
  prevCurrent?: number | null;
}) {
  const { data } = useQuery({
    queryKey: ['battery-prediction', objectId],
    queryFn: () => getBatteryPrediction(objectId),
    refetchInterval: 5 * 60_000,
  });

  const cfg = data ? (TREND_CONFIG[data.trend] ?? TREND_CONFIG.insufficient_data) : null;

  // Inline delta helper (zamjena za MetricCard trend strelicu)
  const delta = (cur?: number | null, prev?: number | null) => {
    if (cur == null || prev == null) return null;
    const thr = Math.max(Math.abs(prev), 0.01) * 0.02;
    if (cur - prev > thr) return <TrendingUp size={13} style={{ color: 'var(--success)', marginLeft: 3 }} />;
    if (prev - cur > thr) return <TrendingDown size={13} style={{ color: 'var(--danger)', marginLeft: 3 }} />;
    return null;
  };

  return (
    <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
      {/* Zaglavlje */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: 7,
        padding: '8px 14px', borderBottom: '1px solid var(--border)',
      }}>
        <Battery size={14} style={{ color: 'var(--success)' }} />
        <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--text2)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          Baterija
        </span>
      </div>

      {/* Napon + Struja — inline, bez ugniježđenih kartica */}
      <div style={{ display: 'flex', gap: 0, borderBottom: '1px solid var(--border)' }}>
        {/* Napon */}
        <div style={{ flex: 1, padding: '14px', borderRight: '1px solid var(--border)' }}>
          <div className="metric-label">Napon</div>
          <div className="metric-value" style={{ marginTop: 4 }}>
            {voltage != null ? (
              <>
                <span style={{ color: 'var(--success)' }}>{voltage.toFixed(2)}</span>
                <span className="metric-unit">V</span>
                {delta(voltage, prevVoltage)}
              </>
            ) : <span className="metric-na">N/A</span>}
          </div>
        </div>
        {/* Struja */}
        <div style={{ flex: 1, padding: '14px' }}>
          <div className="metric-label">Struja</div>
          <div className="metric-value" style={{ marginTop: 4 }}>
            {current != null ? (
              <>
                <span>{current.toFixed(2)}</span>
                <span className="metric-unit">A</span>
                {delta(current, prevCurrent)}
              </>
            ) : <span className="metric-na">N/A</span>}
          </div>
        </div>
      </div>

      {/* Predikcija */}
      {data && cfg && (
        <div style={{
          padding: '9px 14px',
          display: 'flex',
          flexWrap: 'wrap',
          gap: '5px 18px',
          alignItems: 'center',
        }}>
          <span style={{ fontSize: 12, fontWeight: 600, color: cfg.color }}>
            {cfg.label}
          </span>

          {data.trend !== 'insufficient_data' && (
            <span style={{ fontSize: 12, color: 'var(--text2)' }}>
              {data.slope_v_per_hour >= 0 ? '+' : ''}
              {data.slope_v_per_hour.toFixed(4)} V/h
            </span>
          )}

          {data.days_to_warning != null && data.days_to_warning > 0 && (
            <span style={{ fontSize: 12, color: 'var(--warning)', fontWeight: 500 }}>
              ⚠ upoz. za {formatDays(data.days_to_warning)}
            </span>
          )}

          {data.days_to_critical != null && data.days_to_critical > 0 && (
            <span style={{ fontSize: 12, color: 'var(--danger)', fontWeight: 600 }}>
              ⛔ kritično za {formatDays(data.days_to_critical)}
            </span>
          )}

          {data.trend === 'insufficient_data' && (
            <span style={{ fontSize: 11, color: 'var(--text2)' }}>
              Potrebno min. 6 satnih mjerenja
            </span>
          )}

          {data.r_squared != null && data.trend !== 'insufficient_data' && (
            <span style={{ fontSize: 11, color: 'var(--text3)', marginLeft: 'auto' }}>
              R²={data.r_squared.toFixed(2)} · {data.sample_count} uzoraka
            </span>
          )}
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────

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
  alarm_lantern_night_light_off: 'Svjetlo noću ugašeno',
  alarm_lantern_day_light_on: 'Svjetlo danju upaljeno',
  alarm_lantern_comm_failed: 'Svjetlo komunikacija pala',
  alarm_lantern_other_error: 'Svjetlo ostala greška',
  alarm_modem_network_error: 'Modem mrežna greška',
  alarm_modem_other_error: 'Modem ostala greška',
  alarm_station_other_error: 'Stanica ostala greška',
  // Novi alarmi — modularni program (Tip 2)
  alarm_visibility_comm_failed: 'Vidljivost: greška veze',
  alarm_visibility_error: 'Vidljivost: greška senzora',
  alarm_fog_signal_off_during_fog: 'Sirena: nije aktivna u magli',
  alarm_fog_signal_on_while_no_fog: 'Sirena: aktivna bez magle',
};

const LOG_LEVELS: Record<number, { label: string; cls: string }> = {
  1: { label: 'Debug', cls: 'badge-neutral' },
  2: { label: 'Info', cls: 'badge-neutral' },
  3: { label: 'Upozorenje', cls: 'badge-warning' },
  4: { label: 'Greška', cls: 'badge-danger' },
  5: { label: 'Kritično', cls: 'badge-danger' },
};

function EditObjectModal({ obj, onClose }: { obj: import('../types').ObjectView; onClose: () => void }) {
  const qc = useQueryClient();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const pf = obj.program_features;
  const [form, setForm] = useState({
    name: obj.name,
    short_name: obj.short_name ?? '',
    region_id: obj.region_id,
    station_type_id: '',
    datalogger_url: obj.datalogger_url ?? '',
    location_name: obj.location_name ?? '',
    latitude: obj.latitude != null ? String(obj.latitude) : '',
    longitude: obj.longitude != null ? String(obj.longitude) : '',
    allowed_radius_m: obj.allowed_radius_m != null ? String(obj.allowed_radius_m) : '0',
    poll_interval_sec: String(obj.poll_interval_sec),
    polling_enabled: obj.polling_enabled,
    is_active: obj.is_active,
    description: obj.description ?? '',
    // Program tip
    is_modular: pf != null,
    program_version: obj.program_version ?? '',
    pf_sealite: pf?.sealite ?? false,
    pf_navlite: pf?.navlite ?? false,
    pf_modem: pf?.modem ?? false,
    pf_modem_on_other: pf?.modem_on_other_station ?? false,
    pf_vaisala: pf?.vaisala_pwd20 ?? false,
    pf_visibility_other: pf?.visibility_on_other_station ?? false,
    pf_fog: pf?.fog_signal ?? false,
  });

  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });
  const { data: types } = useQuery({ queryKey: ['station-types'], queryFn: listStationTypes });

  const set = (k: string, v: string | boolean) => setForm((f) => ({ ...f, [k]: v }));

  const update = useMutation({
    mutationFn: (data: Record<string, unknown>) => updateObject(obj.id, data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['object', obj.id] });
      qc.invalidateQueries({ queryKey: ['objects'] });
      onClose();
    },
    onError: (err: unknown) => {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(msg || 'Greška pri spremanju');
      setSaving(false);
    },
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError('');
    update.mutate({
      name: form.name,
      short_name: form.short_name || undefined,
      region_id: form.region_id,
      station_type_id: form.station_type_id ? Number(form.station_type_id) : undefined,
      datalogger_url: form.datalogger_url || undefined,
      location_name: form.location_name || undefined,
      latitude: form.latitude ? Number(form.latitude) : undefined,
      longitude: form.longitude ? Number(form.longitude) : undefined,
      allowed_radius_m: Number(form.allowed_radius_m) || 0,
      poll_interval_sec: Number(form.poll_interval_sec) || 60,
      polling_enabled: form.polling_enabled,
      is_active: form.is_active,
      description: form.description || undefined,
      program_version: form.is_modular && form.program_version ? form.program_version : undefined,
      program_features: form.is_modular ? {
        sealite: form.pf_sealite,
        navlite: form.pf_navlite,
        modem: form.pf_modem,
        modem_on_other_station: form.pf_modem_on_other,
        vaisala_pwd20: form.pf_vaisala,
        visibility_on_other_station: form.pf_visibility_other,
        fog_signal: form.pf_fog,
      } : null,
    });
  };

  return (
    <div className="modal-overlay" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal-box card">
        <div className="modal-header">
          <h3>Uredi objekt</h3>
          <button className="modal-close" onClick={onClose}><X size={18} /></button>
        </div>
        <form onSubmit={handleSubmit} className="modal-form">
          {error && <div className="error-msg">{error}</div>}

          <div className="form-row">
            <div className="form-group">
              <label>Naziv *</label>
              <input value={form.name} onChange={(e) => set('name', e.target.value)} required />
            </div>
            <div className="form-group">
              <label>Kratki naziv</label>
              <input value={form.short_name} onChange={(e) => set('short_name', e.target.value)} />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Regija *</label>
              <select value={form.region_id} onChange={(e) => set('region_id', e.target.value)} required>
                <option value="">Odaberi regiju...</option>
                {regions?.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label>Tip stanice</label>
              <select value={form.station_type_id} onChange={(e) => set('station_type_id', e.target.value)}>
                <option value="">Bez tipa</option>
                {types?.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
              </select>
            </div>
          </div>

          <div className="form-group">
            <label>Datalogger URL</label>
            <input value={form.datalogger_url} onChange={(e) => set('datalogger_url', e.target.value)} placeholder="http://192.168.1.100" />
          </div>

          <div className="form-group">
            <label>Lokacija</label>
            <input value={form.location_name} onChange={(e) => set('location_name', e.target.value)} />
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Latitude</label>
              <input type="number" step="any" value={form.latitude} onChange={(e) => set('latitude', e.target.value)} />
            </div>
            <div className="form-group">
              <label>Longitude</label>
              <input type="number" step="any" value={form.longitude} onChange={(e) => set('longitude', e.target.value)} />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Dozvoljeni radijus (m)</label>
              <input type="number" min="0" value={form.allowed_radius_m} onChange={(e) => set('allowed_radius_m', e.target.value)} />
            </div>
            <div className="form-group">
              <label>Poll interval (s)</label>
              <input type="number" min={10} value={form.poll_interval_sec} onChange={(e) => set('poll_interval_sec', e.target.value)} />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group" style={{ flexDirection: 'row', alignItems: 'center', gap: 16, marginTop: 8 }}>
              <label className="filter-checkbox">
                <input type="checkbox" checked={form.polling_enabled} onChange={(e) => set('polling_enabled', e.target.checked)} style={{ width: 'auto' }} />
                Polling uključen
              </label>
              <label className="filter-checkbox">
                <input type="checkbox" checked={form.is_active} onChange={(e) => set('is_active', e.target.checked)} style={{ width: 'auto' }} />
                Aktivan
              </label>
            </div>
          </div>

          <div className="form-group">
            <label>Opis</label>
            <input value={form.description} onChange={(e) => set('description', e.target.value)} />
          </div>

          {/* Program tip */}
          <div className="form-group" style={{ marginTop: 8 }}>
            <label>Program tip</label>
            <select
              value={form.is_modular ? 'modular' : 'galija'}
              onChange={(e) => set('is_modular', e.target.value === 'modular')}
            >
              <option value="galija">Tip 1 — Galija (stari program)</option>
              <option value="modular">Tip 2 — Modularni program</option>
            </select>
          </div>

          {form.is_modular && (
            <>
              <div className="form-group">
                <label>Verzija programa</label>
                <input
                  value={form.program_version}
                  onChange={(e) => set('program_version', e.target.value)}
                  placeholder="npr. 0.05"
                />
              </div>
              <div className="form-group">
                <label>Instalirani moduli</label>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '6px 16px', marginTop: 4 }}>
                  {[
                    ['pf_sealite',        'SeaLite fenjer (SL serija)'],
                    ['pf_navlite',        'NavLite fenjer'],
                    ['pf_modem',          'Lokalni modem'],
                    ['pf_modem_on_other', 'Modem na drugoj stanici'],
                    ['pf_vaisala',        'Vaisala PWD20 vidljivost'],
                    ['pf_visibility_other','Vidljivost s druge stanice'],
                    ['pf_fog',            'Sirena SFH'],
                  ].map(([key, label]) => (
                    <label key={key} className="filter-checkbox" style={{ fontSize: 13 }}>
                      <input
                        type="checkbox"
                        checked={!!form[key as keyof typeof form]}
                        onChange={(e) => set(key, e.target.checked)}
                        style={{ width: 'auto' }}
                      />
                      {label}
                    </label>
                  ))}
                </div>
              </div>
            </>
          )}

          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>Odustani</button>
            <button type="submit" className="btn-primary" disabled={saving}>
              {saving ? <><span className="spinner" style={{ width: 14, height: 14 }} /> Sprema...</> : 'Spremi'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export default function ObjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const qc = useQueryClient();
  const { isAdmin } = useAuth();
  const [tab, setTab] = useState<Tab>('overview');
  const [range, setRange] = useState<Range>('24h');
  const [polling, setPolling] = useState(false);
  const [pollResult, setPollResult] = useState<string | null>(null);
  const [showEdit, setShowEdit] = useState(false);
  const [driftRange, setDriftRange] = useState<DriftRange>('24h');

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

  const { data: driftMeasurements, isLoading: loadingDrift } = useQuery({
    queryKey: ['drift', id, driftRange],
    queryFn: () => {
      const hoursMap: Record<string, number> = { '1h': 1, '6h': 6, '24h': 24 };
      const from = driftRange === '7d'
        ? subDays(new Date(), 7).toISOString()
        : subHours(new Date(), hoursMap[driftRange]).toISOString();
      return getMeasurements10min(id!, { from, limit: driftRange === '7d' ? 1000 : 500 });
    },
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
      {showEdit && <EditObjectModal obj={obj} onClose={() => setShowEdit(false)} />}
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
            {isAdmin && (
              <button className="btn-secondary" style={{ marginLeft: 8, padding: '4px 10px', fontSize: 13 }} onClick={() => setShowEdit(true)}>
                <Pencil size={13} /> Uredi
              </button>
            )}
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
            {obj.program_features != null
              ? <span className="badge" style={{ background: 'var(--accent)', color: '#fff', fontSize: 11 }}><Cpu size={10} /> Tip 2 — Modularni</span>
              : <span className="badge badge-neutral" style={{ fontSize: 11 }}><Cpu size={10} /> Tip 1 — Galija</span>
            }
          </div>
        </div>
      </div>

      <div className="detail-tabs">
        {(['overview', 'charts', 'alarms', 'heatmap', 'events'] as Tab[]).map((t) => (
          <button
            key={t}
            className={`tab-btn ${tab === t ? 'active' : ''}`}
            onClick={() => setTab(t)}
          >
            {{
              overview: 'Pregled',
              charts: 'Grafovi',
              alarms: 'Alarmi',
              heatmap: 'Heatmap',
              events: 'Log',
            }[t]}
          </button>
        ))}
      </div>

      {tab === 'overview' && (
        <div className="overview-tab">
          <BatterySection
            objectId={id!}
            voltage={latest?.battery_voltage_avg}
            current={latest?.battery_current_avg}
            prevVoltage={recentPositions?.[1]?.battery_voltage_avg}
            prevCurrent={recentPositions?.[1]?.battery_current_avg}
          />
          <div className="metrics-grid" style={{ marginTop: 10 }}>
            <MetricCard icon={<Sun size={20} />} label="Napon solarnog" value={latest?.solar_voltage_avg} unit="V" color="var(--warning)"
              prev={recentPositions?.[1]?.solar_voltage_avg} />
            <MetricCard icon={<Thermometer size={20} />} label="Temp. datalogera" value={latest?.datalogger_temp_avg} unit="°C" color="var(--danger)"
              prev={recentPositions?.[1]?.datalogger_temp_avg} />
            <MetricCard icon={<Wifi size={20} />} label="Internet"
              value={latest?.internet_ok_avg != null ? latest.internet_ok_avg * 100 : null} unit="%" color="var(--accent)"
              prev={recentPositions?.[1]?.internet_ok_avg != null ? recentPositions[1].internet_ok_avg! * 100 : null} />
            <MetricCard icon={<Zap size={20} />} label="Svjetlo aktivno"
              value={latest?.lantern_light_active_avg != null ? latest.lantern_light_active_avg * 100 : null} unit="%" color="var(--warning)"
              prev={recentPositions?.[1]?.lantern_light_active_avg != null ? recentPositions[1].lantern_light_active_avg! * 100 : null} />
            <MetricCard icon={<Zap size={20} />} label="Struja svjetla" value={latest?.lantern_current_avg} unit="A"
              prev={recentPositions?.[1]?.lantern_current_avg} />
            {/* Tip 1 — Galija: GPS sateliti */}
            {!obj.program_features && (
              <MetricCard icon={<Radio size={20} />} label="Garmin sateliti" value={latest?.garmin_satellites_avg}
                prev={recentPositions?.[1]?.garmin_satellites_avg} />
            )}
            {/* Tip 1 — Galija: GPS udaljenost od zadane pozicije */}
            {!obj.program_features && (
              <MetricCard
                icon={<MapPin size={20} />}
                label="GPS udaljenost"
                value={latest?.garmin_distance_avg}
                unit="m"
                color={
                  latest?.garmin_distance_avg == null ? undefined :
                  obj.allowed_radius_m && obj.allowed_radius_m > 0
                    ? (latest.garmin_distance_avg <= obj.allowed_radius_m ? 'var(--success)' : 'var(--danger)')
                    : 'var(--accent)'
                }
                prev={recentPositions?.[1]?.garmin_distance_avg}
              />
            )}
            {/* Tip 2 — Modularni: udaljenost lanterne/modema od zadane pozicije */}
            {obj.program_features?.modem && (
              <MetricCard
                icon={<MapPin size={20} />}
                label="Udaljenost od pozicije"
                value={latest?.lantern_distance_avg}
                unit="m"
                color={
                  latest?.lantern_distance_avg == null ? undefined :
                  obj.allowed_radius_m && obj.allowed_radius_m > 0
                    ? (latest.lantern_distance_avg <= obj.allowed_radius_m ? 'var(--success)' : 'var(--danger)')
                    : 'var(--accent)'
                }
                prev={recentPositions?.[1]?.lantern_distance_avg}
              />
            )}
            {/* Tip 2 — Modularni: vidljivost i sirena */}
            {(obj.program_features?.vaisala_pwd20 || obj.program_features?.visibility_on_other_station) && (
              <MetricCard
                icon={<Eye size={20} />}
                label="Vidljivost"
                value={latest?.visibility_value_avg}
                unit="m"
                color={
                  latest?.visibility_value_avg == null ? undefined :
                  latest.visibility_value_avg < 200 ? 'var(--danger)' :
                  latest.visibility_value_avg < 1000 ? 'var(--warning)' : 'var(--success)'
                }
                prev={recentPositions?.[1]?.visibility_value_avg}
              />
            )}
            {obj.program_features?.fog_signal && (
              <MetricCard
                icon={<Wind size={20} />}
                label="Sirena aktivna"
                value={latest?.fog_signal_active_avg != null ? latest.fog_signal_active_avg * 100 : null}
                unit="%"
                color="var(--accent)"
                prev={recentPositions?.[1]?.fog_signal_active_avg != null ? recentPositions[1].fog_signal_active_avg! * 100 : null}
              />
            )}
            {obj.program_features?.fog_signal && (
              <MetricCard icon={<Wind size={20} />} label="Struja sirene" value={latest?.fog_signal_current_avg} unit="A"
                prev={recentPositions?.[1]?.fog_signal_current_avg} />
            )}
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
              {obj.type_name && <div><span>Fizički tip:</span> {obj.type_name}</div>}
              <div>
                <span>Program tip:</span>{' '}
                {obj.program_features != null ? 'Tip 2 — Modularni' : 'Tip 1 — Galija'}
              </div>
              {obj.program_version && <div><span>Verzija programa:</span> {obj.program_version}</div>}
              {obj.program_features && (
                <div className="info-full">
                  <span>Instalirani moduli:</span>{' '}
                  {[
                    obj.program_features.sealite && 'SeaLite',
                    obj.program_features.navlite && 'NavLite',
                    obj.program_features.modem && 'Modem',
                    obj.program_features.modem_on_other_station && 'Modem (druga stanica)',
                    obj.program_features.vaisala_pwd20 && 'Vaisala PWD20',
                    obj.program_features.visibility_on_other_station && 'Vidljivost (druga stanica)',
                    obj.program_features.fog_signal && 'Sirena SFH',
                  ].filter(Boolean).join(', ') || '—'}
                </div>
              )}
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
            const isModular = !!(obj.program_features?.modem || obj.program_features?.navlite || obj.program_features?.sealite);

            // GPS trail in chronological order (API returns DESC → reverse to ASC)
            const trailPoints = (driftMeasurements ?? [])
              .filter((m) =>
                isModular
                  ? m.lantern_latitude_avg != null && m.lantern_longitude_avg != null
                  : m.garmin_latitude_avg != null && m.garmin_longitude_avg != null
              )
              .map((m) => ({
                id: m.id,
                lat: isModular ? m.lantern_latitude_avg! : m.garmin_latitude_avg!,
                lng: isModular ? m.lantern_longitude_avg! : m.garmin_longitude_avg!,
                dist: isModular ? m.lantern_distance_avg : m.garmin_distance_avg,
                time: m.recorded_at,
              }))
              .reverse();

            const polylinePositions = trailPoints.map((p) => [p.lat, p.lng] as [number, number]);

            // Stats
            const maxDrift = trailPoints.reduce((max, p) =>
              p.dist != null ? Math.max(max, p.dist) : max, 0
            );
            let trailLength = 0;
            for (let i = 1; i < trailPoints.length; i++) {
              trailLength += haversineDistance(
                trailPoints[i - 1].lat, trailPoints[i - 1].lng,
                trailPoints[i].lat, trailPoints[i].lng
              );
            }
            const driftedOutside = obj.allowed_radius_m != null && obj.allowed_radius_m > 0 && maxDrift > obj.allowed_radius_m;

            return (
              <div className="location-map-section card" style={{ marginTop: 16 }}>
                <div className="location-map-header">
                  <div className="drift-title-row">
                    <h3>GPS drift analiza</h3>
                    {driftedOutside && (
                      <span className="badge badge-danger" style={{ fontSize: 11 }}>
                        <AlertTriangle size={11} /> Van radijusa
                      </span>
                    )}
                  </div>
                  <div className="drift-controls">
                    <div className="drift-range-selector">
                      {(['1h', '6h', '24h', '7d'] as DriftRange[]).map((r) => (
                        <button
                          key={r}
                          className={`filter-tab ${driftRange === r ? 'active' : ''}`}
                          onClick={() => setDriftRange(r)}
                        >
                          {r}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="location-map-legend">
                  <span className="loc-legend-item"><span className="loc-dot default" />Zadana pozicija</span>
                  {obj.allowed_radius_m != null && obj.allowed_radius_m > 0 && (
                    <span className="loc-legend-item"><span className="loc-dot radius" />Dozvoljeni radijus</span>
                  )}
                  {trailPoints.length > 0 && (
                    <span className="loc-legend-item"><span className="loc-dot drift-trail" />GPS trag</span>
                  )}
                  {trailPoints.length > 0 && (
                    <span className="loc-legend-item"><span className="loc-dot drift-current" />Trenutna pozicija</span>
                  )}
                </div>

                <div className="location-map-wrap location-map-drift">
                  {loadingDrift ? (
                    <div className="drift-loading"><div className="spinner" /></div>
                  ) : (
                    <MapContainer
                      center={[obj.latitude, obj.longitude]}
                      zoom={15}
                      style={{ height: '100%', width: '100%' }}
                      scrollWheelZoom={true}
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
                      {/* GPS trail polyline */}
                      {polylinePositions.length > 1 && (
                        <Polyline
                          positions={polylinePositions}
                          pathOptions={{ color: '#f97316', weight: 3, opacity: 0.75 }}
                        />
                      )}
                      {/* Trail points — all intermediate points */}
                      {trailPoints.slice(1, -1).map((p) => (
                        <CircleMarker
                          key={p.id}
                          center={[p.lat, p.lng]}
                          radius={3}
                          pathOptions={{ color: '#f97316', fillColor: '#f97316', fillOpacity: 0.55, weight: 1 }}
                        >
                          <Popup>
                            <strong>GPS točka</strong><br />
                            {p.lat.toFixed(5)}, {p.lng.toFixed(5)}<br />
                            {p.dist != null && <><span>Udaljenost: </span>{p.dist.toFixed(0)} m<br /></>}
                            <span style={{ fontSize: 11, color: '#6b7280' }}>
                              {format(parseISO(p.time), 'dd.MM.yyyy HH:mm')}
                            </span>
                          </Popup>
                        </CircleMarker>
                      ))}
                      {/* Oldest point */}
                      {trailPoints.length > 1 && (
                        <CircleMarker
                          center={[trailPoints[0].lat, trailPoints[0].lng]}
                          radius={5}
                          pathOptions={{ color: '#64748b', fillColor: '#64748b', fillOpacity: 0.8, weight: 1.5 }}
                        >
                          <Popup>
                            <strong>Početna točka traga</strong><br />
                            {trailPoints[0].lat.toFixed(5)}, {trailPoints[0].lng.toFixed(5)}<br />
                            {trailPoints[0].dist != null && <><span>Udaljenost: </span>{trailPoints[0].dist.toFixed(0)} m<br /></>}
                            <span style={{ fontSize: 11, color: '#6b7280' }}>
                              {format(parseISO(trailPoints[0].time), 'dd.MM.yyyy HH:mm')}
                            </span>
                          </Popup>
                        </CircleMarker>
                      )}
                      {/* Newest / current point */}
                      {trailPoints.length > 0 && (
                        <CircleMarker
                          center={[trailPoints[trailPoints.length - 1].lat, trailPoints[trailPoints.length - 1].lng]}
                          radius={8}
                          pathOptions={{ color: '#ef4444', fillColor: '#ef4444', fillOpacity: 0.95, weight: 2 }}
                        >
                          <Popup>
                            <strong>Trenutna GPS pozicija</strong><br />
                            {trailPoints[trailPoints.length - 1].lat.toFixed(5)}, {trailPoints[trailPoints.length - 1].lng.toFixed(5)}<br />
                            {trailPoints[trailPoints.length - 1].dist != null && (
                              <><span>Udaljenost: </span>{trailPoints[trailPoints.length - 1].dist!.toFixed(0)} m<br /></>
                            )}
                            <span style={{ fontSize: 11, color: '#6b7280' }}>
                              {format(parseISO(trailPoints[trailPoints.length - 1].time), 'dd.MM.yyyy HH:mm')}
                            </span>
                          </Popup>
                        </CircleMarker>
                      )}
                      {/* Home / anchor position */}
                      <CircleMarker
                        center={[obj.latitude, obj.longitude]}
                        radius={9}
                        pathOptions={{ color: '#2563eb', fillColor: '#2563eb', fillOpacity: 0.9, weight: 2 }}
                      >
                        <Popup>
                          <strong>Zadana pozicija (sidro)</strong><br />
                          {obj.latitude.toFixed(5)}, {obj.longitude.toFixed(5)}
                        </Popup>
                      </CircleMarker>
                    </MapContainer>
                  )}
                </div>

                {/* Drift stats */}
                <div className="drift-stats">
                  <div className="drift-stat">
                    <span className="drift-stat-label">Maks. drift</span>
                    <span
                      className="drift-stat-value"
                      style={{ color: driftedOutside ? 'var(--danger)' : 'var(--text)' }}
                    >
                      {trailPoints.length > 0 ? `${maxDrift.toFixed(0)} m` : '—'}
                    </span>
                  </div>
                  <div className="drift-stat">
                    <span className="drift-stat-label">Duljina traga</span>
                    <span className="drift-stat-value">
                      {trailLength >= 1000
                        ? `${(trailLength / 1000).toFixed(2)} km`
                        : trailLength > 0 ? `${trailLength.toFixed(0)} m` : '—'}
                    </span>
                  </div>
                  <div className="drift-stat">
                    <span className="drift-stat-label">GPS točaka</span>
                    <span className="drift-stat-value">{trailPoints.length}</span>
                  </div>
                  {trailPoints.length > 0 && (
                    <div className="drift-stat">
                      <span className="drift-stat-label">Posljednje GPS</span>
                      <span className="drift-stat-value" style={{ fontSize: 12 }}>
                        {format(parseISO(trailPoints[trailPoints.length - 1].time), 'dd.MM. HH:mm')}
                      </span>
                    </div>
                  )}
                  {trailPoints.length === 0 && !loadingDrift && (
                    <div className="drift-stat drift-stat-nodata">
                      <span className="drift-stat-label">Nema GPS podataka za odabrani period</span>
                    </div>
                  )}
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
                <h4>Solarni panel (V)</h4>
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
                <h4>Svjetlo</h4>
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

              {/* Vidljivost — samo ako je Vaisala ili vidljivost s druge stanice */}
              {(obj.program_features?.vaisala_pwd20 || obj.program_features?.visibility_on_other_station) && (
                <div className="chart-card card chart-wide">
                  <h4>Vidljivost (m)</h4>
                  <ResponsiveContainer width="100%" height={180}>
                    <LineChart data={chartData}>
                      <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                      <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                      <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                      <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                      <Legend />
                      <Line type="monotone" dataKey="visibility_value_avg" stroke="var(--accent)" dot={false} name="Vidljivost (m)" />
                      <Line type="monotone" dataKey="visibility_alarm_avg" stroke="var(--danger)" dot={false} name="Alarm vidljivosti" />
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              )}

              {/* Sirena — samo ako je instalirana */}
              {obj.program_features?.fog_signal && (
                <div className="chart-card card chart-wide">
                  <h4>Sirena</h4>
                  <ResponsiveContainer width="100%" height={180}>
                    <LineChart data={chartData}>
                      <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                      <XAxis dataKey="time" tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                      <YAxis tick={{ fontSize: 11, fill: 'var(--text2)' }} />
                      <Tooltip contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6 }} />
                      <Legend />
                      <Line type="monotone" dataKey="fog_signal_active_avg" stroke="var(--warning)" dot={false} name="Aktivan (0/1)" />
                      <Line type="monotone" dataKey="fog_signal_current_avg" stroke="var(--accent)" dot={false} name="Struja (A)" />
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              )}
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

      {tab === 'heatmap' && (
        <AlarmHeatmapTab objectId={id!} />
      )}

      {tab === 'events' && (
        <div className="events-tab">
          {loadingEvents ? (
            <div className="page-spinner"><div className="spinner" /></div>
          ) : !events?.length ? (
            <div className="no-data">Nema event log zapisa</div>
          ) : (
            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
              <div className="table-scroll">
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
            </div>
          )}
        </div>
      )}
    </div>
  );
}
