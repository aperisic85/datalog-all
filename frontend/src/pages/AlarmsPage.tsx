import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useSearchParams, Link } from 'react-router-dom';
import { listAlarmHistory, listRegions, acknowledgeAlarm, deleteAlarm } from '../api/endpoints';
import type { AlarmListItem } from '../types';
import {
  AlertTriangle, Battery, Wifi, WifiOff,
  MapPin, Thermometer, Zap, Check, Trash2,
  ExternalLink, Filter, ChevronLeft, ChevronRight,
  Clock, CheckCircle, X, Wind, Eye,
} from 'lucide-react';
import { formatDistanceToNow, format } from 'date-fns';
import { bs } from 'date-fns/locale';
import './AlarmsPage.css';

// ── Alarm tip definicije ────────────────────────────────────────────────────
type AlarmKey = keyof Pick<AlarmListItem,
  'alarm_battery_voltage_flat' | 'alarm_battery_voltage_low' | 'alarm_battery_other_error' |
  'alarm_datalogger_high_temp' | 'alarm_datalogger_high_voltage' | 'alarm_datalogger_other_error' |
  'alarm_garmin_comm_failed' | 'alarm_garmin_other_error' | 'alarm_station_out_of_radius' |
  'alarm_lantern_night_light_off' | 'alarm_lantern_day_light_on' |
  'alarm_lantern_comm_failed' | 'alarm_lantern_other_error' |
  'alarm_modem_network_error' | 'alarm_modem_other_error' | 'alarm_station_other_error' |
  'alarm_visibility_comm_failed' | 'alarm_visibility_error' |
  'alarm_fog_signal_off_during_fog' | 'alarm_fog_signal_on_while_no_fog'
>;

const ALARM_DEFS: { key: AlarmKey; label: string; icon: React.ReactNode; severity: 'danger' | 'warning' }[] = [
  { key: 'alarm_battery_voltage_flat',    label: 'Baterija prazna',          icon: <Battery size={12} />,       severity: 'danger' },
  { key: 'alarm_battery_voltage_low',     label: 'Baterija slaba',           icon: <Battery size={12} />,       severity: 'warning' },
  { key: 'alarm_battery_other_error',     label: 'Greška baterije',          icon: <Battery size={12} />,       severity: 'warning' },
  { key: 'alarm_datalogger_high_temp',    label: 'Visoka temp.',             icon: <Thermometer size={12} />,   severity: 'warning' },
  { key: 'alarm_datalogger_high_voltage', label: 'Visoki napon',             icon: <Zap size={12} />,           severity: 'warning' },
  { key: 'alarm_datalogger_other_error',  label: 'Greška datalogera',        icon: <AlertTriangle size={12} />, severity: 'warning' },
  { key: 'alarm_garmin_comm_failed',      label: 'GPS komunikacija pala',    icon: <MapPin size={12} />,        severity: 'danger' },
  { key: 'alarm_garmin_other_error',      label: 'GPS greška',               icon: <MapPin size={12} />,        severity: 'warning' },
  { key: 'alarm_station_out_of_radius',   label: 'Van radijusa',             icon: <MapPin size={12} />,        severity: 'danger' },
  { key: 'alarm_lantern_night_light_off', label: 'Svjetlo ugašeno noću',     icon: <Zap size={12} />,           severity: 'danger' },
  { key: 'alarm_lantern_day_light_on',    label: 'Svjetlo upaljeno danju',   icon: <Zap size={12} />,           severity: 'warning' },
  { key: 'alarm_lantern_comm_failed',     label: 'Svjetlo komun. pala',      icon: <WifiOff size={12} />,       severity: 'danger' },
  { key: 'alarm_lantern_other_error',     label: 'Svjetlo greška',           icon: <Zap size={12} />,           severity: 'warning' },
  { key: 'alarm_modem_network_error',          label: 'Greška mreže',               icon: <Wifi size={12} />,          severity: 'warning' },
  { key: 'alarm_modem_other_error',            label: 'Greška modema',              icon: <WifiOff size={12} />,       severity: 'warning' },
  { key: 'alarm_station_other_error',          label: 'Greška stanice',             icon: <AlertTriangle size={12} />, severity: 'warning' },
  { key: 'alarm_visibility_comm_failed',       label: 'Vidljivost: greška veze',    icon: <Eye size={12} />,           severity: 'danger'  },
  { key: 'alarm_visibility_error',             label: 'Vidljivost: greška senzora', icon: <Eye size={12} />,           severity: 'warning' },
  { key: 'alarm_fog_signal_off_during_fog',    label: 'Sirena: nije aktivna u magli', icon: <Wind size={12} />,        severity: 'danger'  },
  { key: 'alarm_fog_signal_on_while_no_fog',   label: 'Sirena: aktivna bez magle',  icon: <Wind size={12} />,          severity: 'warning' },
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

function isCriticalAlarm(item: AlarmListItem) {
  return item.alarm_battery_voltage_flat > 0 ||
    item.alarm_garmin_comm_failed > 0 ||
    item.alarm_lantern_night_light_off > 0 ||
    item.alarm_lantern_comm_failed > 0 ||
    item.alarm_station_out_of_radius > 0 ||
    item.alarm_fog_signal_off_during_fog > 0 ||
    item.alarm_visibility_comm_failed > 0;
}

// ── Modalna forma za potvrdu ────────────────────────────────────────────────
function ConfirmModal({ title, message, danger, confirmLabel, onConfirm, onCancel }: {
  title: string;
  message: React.ReactNode;
  danger?: boolean;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal-box card" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{title}</h3>
          <button className="modal-close-btn" onClick={onCancel}><X size={18} /></button>
        </div>
        <p className="modal-body">{message}</p>
        <div className="modal-actions">
          <button className="btn-secondary" onClick={onCancel}>Odustani</button>
          <button className={danger ? 'btn-danger' : 'btn-primary'} onClick={onConfirm}>
            {danger ? <Trash2 size={14} /> : <Check size={14} />} {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Status tab definicije ──────────────────────────────────────────────────
type Status = 'active' | 'acknowledged' | 'all';
const STATUS_TABS: { value: Status; label: string; icon: React.ReactNode }[] = [
  { value: 'active',       label: 'Aktivni',   icon: <AlertTriangle size={14} /> },
  { value: 'acknowledged', label: 'Potvrđeni', icon: <CheckCircle size={14} /> },
  { value: 'all',          label: 'Svi',       icon: <Clock size={14} /> },
];

// ── Alarm kartica ──────────────────────────────────────────────────────────
function AlarmCard({ item, onAcknowledge, onDelete, isAcking }: {
  item: AlarmListItem;
  onAcknowledge: () => void;
  onDelete: () => void;
  isAcking: boolean;
}) {
  const isAcknowledged = !!item.acknowledged_at;
  const critical = isCriticalAlarm(item);

  return (
    <div className={`alarm-card card ${isAcknowledged ? 'alarm-card-ack' : critical ? 'alarm-card-critical' : 'alarm-card-warning'}`}>
      {/* Header: naziv + status badge */}
      <div className="alarm-card-header">
        <div className="alarm-card-title">
          <span className={`status-dot ${isAcknowledged ? 'status-dot-inactive' : 'status-dot-alarm'}`} />
          <div className="alarm-card-title-text">
            <span className="alarm-obj-name">{item.object_name}</span>
            <code className="station-id">{item.station_id}</code>
          </div>
        </div>
        <div>
          {isAcknowledged
            ? <span className="badge badge-neutral"><CheckCircle size={11} /> Potvrđen</span>
            : critical
              ? <span className="badge badge-danger badge-pulse"><AlertTriangle size={11} /> Kritično</span>
              : <span className="badge badge-warning"><AlertTriangle size={11} /> Upozorenje</span>
          }
        </div>
      </div>

      {/* Meta: regija + lokacija */}
      <div className="alarm-card-meta">
        <span className="region-tag">
          <span className="region-dot" style={{ background: item.region_color }} />
          {item.region_name}
        </span>
        {item.location_name && (
          <span className="location-cell"><MapPin size={12} />{item.location_name}</span>
        )}
      </div>

      {/* Alarm tagovi */}
      <AlarmTags item={item} />

      {/* Vremena */}
      <div className="alarm-times">
        <span><Clock size={11} />
          {format(new Date(item.recorded_at), 'dd.MM.yyyy HH:mm')}
          {' · '}
          {formatDistanceToNow(new Date(item.recorded_at), { addSuffix: true, locale: bs })}
        </span>
        {isAcknowledged && item.acknowledged_at && (
          <span className="alarm-ack-info">
            <CheckCircle size={11} />
            Potvrdio: <strong>{item.acknowledged_by || '—'}</strong>
            {' · '}{format(new Date(item.acknowledged_at), 'dd.MM.yyyy HH:mm')}
          </span>
        )}
      </div>

      {/* Akcijski gumbi */}
      <div className="alarm-card-actions">
        <Link to={`/objects/${item.object_id}`} className="btn-secondary alarm-action-btn">
          <ExternalLink size={13} /> Pregledaj
        </Link>
        {!isAcknowledged && (
          <button className="btn-secondary alarm-action-btn" onClick={onAcknowledge} disabled={isAcking}>
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

// ── Glavna stranica ────────────────────────────────────────────────────────
export default function AlarmsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [status, setStatus] = useState<Status>(
    (searchParams.get('status') as Status) || 'active'
  );
  const [regionFilter, setRegionFilter] = useState(searchParams.get('region_id') || '');
  const [page, setPage] = useState(1);

  // Modalne potvrde
  const [ackTarget, setAckTarget]       = useState<AlarmListItem | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<AlarmListItem | null>(null);

  // Loading stanja po kartici
  const [pendingAck, setPendingAck]     = useState<Set<string>>(new Set());
  const [actionError, setActionError]   = useState('');

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

  // Potvrdi alarm
  const doAcknowledge = async () => {
    if (!ackTarget) return;
    setAckTarget(null);
    setActionError('');
    setPendingAck(prev => new Set(prev).add(ackTarget.object_id));
    try {
      await acknowledgeAlarm(ackTarget.object_id);
      await qc.invalidateQueries({ queryKey: ['alarms-history'] });
      await qc.invalidateQueries({ queryKey: ['region-summary'] });
    } catch {
      setActionError(`Greška pri potvrdi alarma za "${ackTarget.object_name}". Pokušaj ponovo.`);
    } finally {
      setPendingAck(prev => { const s = new Set(prev); s.delete(ackTarget.object_id); return s; });
    }
  };

  // Briši jedan alarm zapis
  const doDelete = async () => {
    if (!deleteTarget) return;
    setDeleteTarget(null);
    setActionError('');
    try {
      await deleteAlarm(deleteTarget.id);
      await qc.invalidateQueries({ queryKey: ['alarms-history'] });
      await qc.invalidateQueries({ queryKey: ['region-summary'] });
    } catch {
      setActionError(`Greška pri brisanju alarma. Pokušaj ponovo.`);
    }
  };

  const items = data?.data ?? [];
  const totalPages = data?.total_pages ?? 1;
  const total = data?.total ?? 0;

  return (
    <div className="alarms-page">
      {/* Confirm acknowledge */}
      {ackTarget && (
        <ConfirmModal
          title="Potvrdi alarm"
          message={<>Potvrđuješ alarm za <strong>{ackTarget.object_name}</strong>?
            <br /><span style={{ color: 'var(--text2)', fontSize: 13 }}>
              Alarm će biti označen kao potvrđen s tvojim imenom i vremenom.
            </span></>}
          confirmLabel="Potvrdi alarm"
          onConfirm={doAcknowledge}
          onCancel={() => setAckTarget(null)}
        />
      )}

      {/* Confirm delete */}
      {deleteTarget && (
        <ConfirmModal
          title="Brisanje alarma"
          danger
          message={<>Brišeš alarm od <strong>{format(new Date(deleteTarget.recorded_at), 'dd.MM.yyyy HH:mm')}</strong>{' '}
            za objekt <strong>{deleteTarget.object_name}</strong>.
            <br /><span style={{ color: 'var(--text2)', fontSize: 13 }}>
              Briše se samo ovaj zapis, ne svi alarmi objekta.
            </span></>}
          confirmLabel="Briši ovaj alarm"
          onConfirm={doDelete}
          onCancel={() => setDeleteTarget(null)}
        />
      )}

      {/* Header */}
      <div className="page-header">
        <div>
          <h2>Alarmi</h2>
          <span className="text-muted">
            {total > 0 ? `${total} ${total === 1 ? 'zapis' : 'zapisa'}` : 'Nema zapisa'}
          </span>
        </div>
      </div>

      {/* Greška akcije */}
      {actionError && (
        <div className="error-msg" style={{ marginBottom: 12, display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          {actionError}
          <button style={{ background: 'none', padding: 4, color: 'inherit' }} onClick={() => setActionError('')}>
            <X size={14} />
          </button>
        </div>
      )}

      {/* Status tabs */}
      <div className="alarm-status-tabs">
        {STATUS_TABS.map(t => (
          <button key={t.value}
            className={`alarm-status-tab${status === t.value ? ' active' : ''}`}
            onClick={() => handleStatus(t.value)}
          >
            {t.icon} {t.label}
          </button>
        ))}
      </div>

      {/* Region filter */}
      <div className="alarms-filters card">
        <Filter size={14} style={{ color: 'var(--text2)', flexShrink: 0 }} />
        <select value={regionFilter} onChange={e => handleRegion(e.target.value)}>
          <option value="">Sve regije</option>
          {regions?.map(r => (
            <option key={r.id} value={r.id}>{r.name}</option>
          ))}
        </select>
        {regionFilter && (
          <button className="clear-filter-btn" onClick={() => handleRegion('')}>
            <X size={14} /> Sve regije
          </button>
        )}
      </div>

      {/* Loading / error */}
      {isLoading && <div className="page-spinner"><div className="spinner" /></div>}
      {error && <div className="error-msg">Greška pri učitavanju alarma</div>}

      {/* Empty state */}
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

      {/* Lista */}
      {items.length > 0 && (
        <div className="alarm-list">
          {items.map(item => (
            <AlarmCard
              key={item.id}
              item={item}
              isAcking={pendingAck.has(item.object_id)}
              onAcknowledge={() => setAckTarget(item)}
              onDelete={() => setDeleteTarget(item)}
            />
          ))}
        </div>
      )}

      {/* Paginacija */}
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
