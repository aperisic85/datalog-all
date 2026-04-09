import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useSearchParams, Link } from 'react-router-dom';
import { listAlarmHistory, listRegions, acknowledgeAlarm, deleteAlarms } from '../api/endpoints';
import type { AlarmListItem } from '../types';
import {
  AlertTriangle, Battery, Wifi, WifiOff,
  MapPin, Thermometer, Zap, Check, Trash2,
  ExternalLink, Filter, ChevronLeft, ChevronRight,
  Clock, CheckCircle,
} from 'lucide-react';
import { formatDistanceToNow, format } from 'date-fns';
import { bs } from 'date-fns/locale';
import './AlarmsPage.css';

// ── Mapiranje polja na čitljive opise ──────────────────────────────────────
type AlarmKey = keyof Pick<AlarmListItem,
  'alarm_battery_voltage_flat' | 'alarm_battery_voltage_low' | 'alarm_battery_other_error' |
  'alarm_datalogger_high_temp' | 'alarm_datalogger_high_voltage' | 'alarm_datalogger_other_error' |
  'alarm_garmin_comm_failed' | 'alarm_garmin_other_error' | 'alarm_station_out_of_radius' |
  'alarm_lantern_night_light_off' | 'alarm_lantern_day_light_on' |
  'alarm_lantern_comm_failed' | 'alarm_lantern_other_error' |
  'alarm_modem_network_error' | 'alarm_modem_other_error' | 'alarm_station_other_error'
>;

const ALARM_DEFS: { key: AlarmKey; label: string; icon: React.ReactNode; severity: 'danger' | 'warning' }[] = [
  { key: 'alarm_battery_voltage_flat',    label: 'Baterija prazna',          icon: <Battery size={12} />,     severity: 'danger' },
  { key: 'alarm_battery_voltage_low',     label: 'Baterija slaba',           icon: <Battery size={12} />,     severity: 'warning' },
  { key: 'alarm_battery_other_error',     label: 'Greška baterije',          icon: <Battery size={12} />,     severity: 'warning' },
  { key: 'alarm_datalogger_high_temp',    label: 'Visoka temp.',             icon: <Thermometer size={12} />, severity: 'warning' },
  { key: 'alarm_datalogger_high_voltage', label: 'Visoki napon',             icon: <Zap size={12} />,         severity: 'warning' },
  { key: 'alarm_datalogger_other_error',  label: 'Greška datalogera',        icon: <AlertTriangle size={12} />, severity: 'warning' },
  { key: 'alarm_garmin_comm_failed',      label: 'GPS komunikacija pala',    icon: <MapPin size={12} />,      severity: 'danger' },
  { key: 'alarm_garmin_other_error',      label: 'GPS greška',               icon: <MapPin size={12} />,      severity: 'warning' },
  { key: 'alarm_station_out_of_radius',   label: 'Van radijusa',             icon: <MapPin size={12} />,      severity: 'danger' },
  { key: 'alarm_lantern_night_light_off', label: 'Fenjer ugašen noću',       icon: <Zap size={12} />,         severity: 'danger' },
  { key: 'alarm_lantern_day_light_on',    label: 'Fenjer upaljen danju',     icon: <Zap size={12} />,         severity: 'warning' },
  { key: 'alarm_lantern_comm_failed',     label: 'Fenjer komunikacija pala', icon: <WifiOff size={12} />,     severity: 'danger' },
  { key: 'alarm_lantern_other_error',     label: 'Fenjer greška',            icon: <Zap size={12} />,         severity: 'warning' },
  { key: 'alarm_modem_network_error',     label: 'Greška mreže',             icon: <Wifi size={12} />,        severity: 'warning' },
  { key: 'alarm_modem_other_error',       label: 'Greška modema',            icon: <WifiOff size={12} />,     severity: 'warning' },
  { key: 'alarm_station_other_error',     label: 'Greška stanice',           icon: <AlertTriangle size={12} />, severity: 'warning' },
];

function AlarmTags({ item }: { item: AlarmListItem }) {
  const active = ALARM_DEFS.filter(d => (item[d.key] as number) > 0);
  if (!active.length) return null;
  return (
    <div className="alarm-tag-list">
      {active.map(d => (
        <span key={d.key} className={`alarm-tag alarm-tag-${d.severity}`}>
          {d.icon} {d.label}
        </span>
      ))}
    </div>
  );
}

type Status = 'active' | 'acknowledged' | 'all';

const STATUS_TABS: { value: Status; label: string; icon: React.ReactNode }[] = [
  { value: 'active',       label: 'Aktivni',    icon: <AlertTriangle size={14} /> },
  { value: 'acknowledged', label: 'Potvrđeni',  icon: <CheckCircle size={14} /> },
  { value: 'all',          label: 'Svi',        icon: <Clock size={14} /> },
];

export default function AlarmsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [status, setStatus] = useState<Status>(
    (searchParams.get('status') as Status) || 'active'
  );
  const [regionFilter, setRegionFilter] = useState(searchParams.get('region_id') || '');
  const [page, setPage] = useState(1);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [pendingAck, setPendingAck] = useState<Set<string>>(new Set());
  const qc = useQueryClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ['alarms-history', status, regionFilter, page],
    queryFn: () => listAlarmHistory({
      status,
      region_id: regionFilter || undefined,
      page,
      page_size: 30,
    }),
    refetchInterval: status === 'active' ? 60_000 : undefined,
  });

  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const syncParams = (s: Status, r: string) => {
    const p: Record<string, string> = {};
    if (s !== 'active') p.status = s;
    if (r) p.region_id = r;
    setSearchParams(p);
  };

  const handleStatus = (s: Status) => { setStatus(s); setPage(1); syncParams(s, regionFilter); };
  const handleRegion = (r: string) => { setRegionFilter(r); setPage(1); syncParams(status, r); };

  const handleAcknowledge = async (objectId: string) => {
    setPendingAck(prev => new Set(prev).add(objectId));
    try {
      await acknowledgeAlarm(objectId);
      qc.invalidateQueries({ queryKey: ['alarms-history'] });
      qc.invalidateQueries({ queryKey: ['region-summary'] });
    } finally {
      setPendingAck(prev => { const s = new Set(prev); s.delete(objectId); return s; });
    }
  };

  const handleDeleteConfirm = async () => {
    if (!confirmDelete) return;
    const { id } = confirmDelete;
    setConfirmDelete(null);
    try {
      await deleteAlarms(id);
      qc.invalidateQueries({ queryKey: ['alarms-history'] });
      qc.invalidateQueries({ queryKey: ['region-summary'] });
    } catch { /* ignore */ }
  };

  const items = data?.data ?? [];
  const totalPages = data?.total_pages ?? 1;
  const total = data?.total ?? 0;

  return (
    <div className="alarms-page">
      {/* Delete confirm dialog */}
      {confirmDelete && (
        <div className="modal-overlay" onClick={() => setConfirmDelete(null)}>
          <div className="modal-box card" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h3>Brisanje alarma</h3>
            </div>
            <p style={{ margin: '0 0 20px', color: 'var(--text2)' }}>
              Sigurno želiš obrisati sve alarm zapise za <strong style={{ color: 'var(--text)' }}>{confirmDelete.name}</strong>?
              Ova radnja je nepovratna.
            </p>
            <div className="modal-actions">
              <button className="btn-secondary" onClick={() => setConfirmDelete(null)}>Odustani</button>
              <button className="btn-danger" onClick={handleDeleteConfirm}>
                <Trash2 size={14} /> Briši sve alarme
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="page-header">
        <div>
          <h2>Alarmi</h2>
          <span className="text-muted">
            {total > 0 ? `${total} ${total === 1 ? 'zapis' : total < 5 ? 'zapisa' : 'zapisa'}` : 'Nema zapisa'}
          </span>
        </div>
      </div>

      {/* Status tabs */}
      <div className="alarm-status-tabs">
        {STATUS_TABS.map(t => (
          <button
            key={t.value}
            className={`alarm-status-tab${status === t.value ? ' active' : ''}`}
            onClick={() => handleStatus(t.value)}
          >
            {t.icon} {t.label}
          </button>
        ))}
      </div>

      {/* Filters */}
      <div className="alarms-filters card">
        <Filter size={14} style={{ color: 'var(--text2)', flexShrink: 0 }} />
        <select value={regionFilter} onChange={e => handleRegion(e.target.value)}>
          <option value="">Sve regije</option>
          {regions?.map(r => (
            <option key={r.id} value={r.id}>{r.name}</option>
          ))}
        </select>
      </div>

      {/* Content */}
      {isLoading && <div className="page-spinner"><div className="spinner" /></div>}
      {error && <div className="error-msg">Greška pri učitavanju alarma</div>}

      {!isLoading && items.length === 0 && (
        <div className="alarms-empty">
          <div className="alarms-empty-icon">
            {status === 'active' ? <AlertTriangle size={36} /> : <CheckCircle size={36} />}
          </div>
          <div className="alarms-empty-title">
            {status === 'active' ? 'Nema aktivnih alarma' :
             status === 'acknowledged' ? 'Nema potvrđenih alarma' : 'Nema alarma'}
          </div>
          <div className="alarms-empty-sub">
            {status === 'active' ? 'Svi objekti rade normalno' : 'Nema zapisa za odabrani filter'}
          </div>
        </div>
      )}

      {items.length > 0 && (
        <div className="alarm-list">
          {items.map(item => (
            <AlarmRow
              key={item.id}
              item={item}
              isAcking={pendingAck.has(item.object_id)}
              onAcknowledge={() => handleAcknowledge(item.object_id)}
              onDelete={() => setConfirmDelete({ id: item.object_id, name: item.object_name })}
            />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="alarm-pagination">
          <button className="btn-secondary" disabled={page <= 1} onClick={() => setPage(p => p - 1)}>
            <ChevronLeft size={16} />
          </button>
          <span className="page-info">Strana {page} od {totalPages}</span>
          <button className="btn-secondary" disabled={page >= totalPages} onClick={() => setPage(p => p + 1)}>
            <ChevronRight size={16} />
          </button>
        </div>
      )}
    </div>
  );
}

// ── Alarm row component ────────────────────────────────────────────────────
function AlarmRow({ item, isAcking, onAcknowledge, onDelete }: {
  item: AlarmListItem;
  isAcking: boolean;
  onAcknowledge: () => void;
  onDelete: () => void;
}) {
  const isCritical = item.alarm_battery_voltage_flat > 0 ||
    item.alarm_garmin_comm_failed > 0 ||
    item.alarm_lantern_night_light_off > 0 ||
    item.alarm_lantern_comm_failed > 0 ||
    item.alarm_station_out_of_radius > 0;

  const isAcknowledged = !!item.acknowledged_at;

  return (
    <div className={`alarm-card card ${isAcknowledged ? 'alarm-card-ack' : isCritical ? 'alarm-card-critical' : 'alarm-card-warning'}`}>
      <div className="alarm-card-header">
        <div className="alarm-card-title">
          <span className={`status-dot ${isAcknowledged ? 'status-dot-inactive' : isCritical ? 'status-dot-alarm' : 'status-dot-alarm'}`} />
          <div>
            <div className="alarm-obj-name">{item.object_name}</div>
            <code className="station-id" style={{ marginTop: 2, display: 'block' }}>{item.station_id}</code>
          </div>
        </div>
        <div className="alarm-card-badges">
          {isAcknowledged
            ? <span className="badge badge-neutral"><CheckCircle size={11} /> Potvrđen</span>
            : isCritical
              ? <span className="badge badge-danger badge-pulse"><AlertTriangle size={11} /> Kritično</span>
              : <span className="badge badge-warning"><AlertTriangle size={11} /> Upozorenje</span>
          }
        </div>
      </div>

      <div className="alarm-card-meta">
        <span className="region-tag">
          <span className="region-dot" style={{ background: item.region_color }} />
          {item.region_name}
        </span>
        {item.location_name && (
          <span className="location-cell"><MapPin size={12} /> {item.location_name}</span>
        )}
      </div>

      <AlarmTags item={item} />

      <div className="alarm-times">
        <span>
          <Clock size={11} />
          {format(new Date(item.recorded_at), 'dd.MM.yyyy HH:mm')}
          {' · '}
          {formatDistanceToNow(new Date(item.recorded_at), { addSuffix: true, locale: bs })}
        </span>
        {isAcknowledged && item.acknowledged_by && (
          <span className="alarm-ack-info">
            <CheckCircle size={11} />
            Potvrdio: <strong>{item.acknowledged_by}</strong>
            {' · '}{format(new Date(item.acknowledged_at!), 'dd.MM.yyyy HH:mm')}
          </span>
        )}
      </div>

      <div className="alarm-card-actions">
        <Link to={`/objects/${item.object_id}`} className="btn-secondary alarm-action-btn">
          <ExternalLink size={13} /> Pregledaj
        </Link>
        {!isAcknowledged && (
          <button
            className="btn-secondary alarm-action-btn"
            onClick={onAcknowledge}
            disabled={isAcking}
          >
            {isAcking
              ? <><span className="spinner" style={{ width: 13, height: 13 }} /> Potvrđuje...</>
              : <><Check size={13} /> Potvrdi</>
            }
          </button>
        )}
        <button className="btn-danger alarm-action-btn" onClick={onDelete}>
          <Trash2 size={13} /> Briši
        </button>
      </div>
    </div>
  );
}
