import { useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { useSearchParams, Link } from 'react-router-dom';
import { listAlarmHistory, listRegions, acknowledgeAlarm, deleteAlarm } from '../api/endpoints';
import type { AlarmListItem } from '../types';
import {
  AlertTriangle, Battery, Wifi, WifiOff,
  MapPin, Thermometer, Zap, Check, Trash2,
  ExternalLink, Filter, ChevronLeft, ChevronRight,
  Clock, CheckCircle, X, Wind, Eye,
  Volume2, VolumeX, BellOff, RefreshCw,
} from 'lucide-react';
import { formatDistanceToNow, format } from 'date-fns';
import { hr } from 'date-fns/locale';
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

// Kritičnost se izvodi iz ALARM_DEFS — jedan izvor istine za severity
const CRITICAL_KEYS = ALARM_DEFS.filter(d => d.severity === 'danger').map(d => d.key);

function isCriticalAlarm(item: AlarmListItem) {
  return CRITICAL_KEYS.some(k => item[k] > 0);
}

type Severity = 'critical' | 'warning' | 'ack';

function severityOf(item: AlarmListItem): Severity {
  if (item.acknowledged_at) return 'ack';
  return isCriticalAlarm(item) ? 'critical' : 'warning';
}

function AlarmTags({ item }: { item: AlarmListItem }) {
  const active = ALARM_DEFS.filter(d => item[d.key] > 0);
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

// ── Zvučna signalizacija (annunciator horn) ────────────────────────────────
// Klasična SCADA sirena: svira dok postoji nepotvrđeni kritični alarm,
// "Utišaj" je gasi do pojave NOVOG kritičnog alarma (re-annunciation).
function useAlarmHorn(criticalCount: number) {
  const [enabled, setEnabled] = useState(() => localStorage.getItem('alarm-horn') === '1');
  const [silenced, setSilenced] = useState(false);
  const [prevCount, setPrevCount] = useState(criticalCount);

  // Porast broja kritičnih alarma poništava utišanje (re-annunciation)
  if (criticalCount !== prevCount) {
    setPrevCount(criticalCount);
    if (criticalCount > prevCount) setSilenced(false);
  }

  const sounding = enabled && !silenced && criticalCount > 0;

  useEffect(() => {
    if (!sounding) return;
    const ctx = new AudioContext();
    const beep = () => {
      if (ctx.state !== 'running') { ctx.resume(); return; }
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'square';
      osc.frequency.value = 880;
      gain.gain.setValueAtTime(0.05, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.25);
      osc.connect(gain).connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.25);
    };
    beep();
    const timer = setInterval(beep, 2500);
    return () => { clearInterval(timer); ctx.close(); };
  }, [sounding]);

  const toggle = () => setEnabled(e => {
    localStorage.setItem('alarm-horn', e ? '0' : '1');
    return !e;
  });

  return { enabled, toggle, sounding, silence: () => setSilenced(true) };
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

// ── SCADA statusna traka (annunciator panel) ───────────────────────────────
function ScadaBanner({ critical, warning, activeTotal, updatedAt, onRefresh, isFetching, horn }: {
  critical: number;
  warning: number;
  activeTotal: number;
  updatedAt: number;
  onRefresh: () => void;
  isFetching: boolean;
  horn: ReturnType<typeof useAlarmHorn>;
}) {
  return (
    <div className="scada-banner card">
      <div className="scada-tiles">
        <div className={`scada-tile scada-tile-critical${critical > 0 ? ' lit' : ''}`}>
          <span className="scada-tile-count">{critical}</span>
          <span className="scada-tile-label">Kritično</span>
        </div>
        <div className={`scada-tile scada-tile-warning${warning > 0 ? ' lit' : ''}`}>
          <span className="scada-tile-count">{warning}</span>
          <span className="scada-tile-label">Upozorenje</span>
        </div>
        <div className={`scada-tile scada-tile-total${activeTotal === 0 ? ' ok' : ''}`}>
          <span className="scada-tile-count">{activeTotal}</span>
          <span className="scada-tile-label">Aktivnih</span>
        </div>
      </div>
      <div className="scada-banner-right">
        {updatedAt > 0 && (
          <span className="scada-updated" title="Vrijeme zadnjeg osvježavanja podataka">
            <Clock size={12} /> {format(updatedAt, 'HH:mm:ss')}
          </span>
        )}
        <button className="btn-secondary scada-btn" onClick={onRefresh} disabled={isFetching}
          title="Osvježi podatke">
          <RefreshCw size={13} className={isFetching ? 'spin' : undefined} /> Osvježi
        </button>
        <button
          className={`btn-secondary scada-btn${horn.enabled ? ' scada-btn-on' : ''}`}
          onClick={horn.toggle}
          title={horn.enabled ? 'Isključi zvučnu signalizaciju' : 'Uključi zvučnu signalizaciju za kritične alarme'}
        >
          {horn.enabled ? <Volume2 size={13} /> : <VolumeX size={13} />} Sirena
        </button>
        {horn.sounding && (
          <button className="btn-danger scada-btn scada-btn-silence" onClick={horn.silence}
            title="Utišaj sirenu do pojave novog kritičnog alarma">
            <BellOff size={13} /> Utišaj
          </button>
        )}
      </div>
    </div>
  );
}

// ── Redak SCADA tablice ────────────────────────────────────────────────────
function AlarmRow({ item, onAcknowledge, onDelete, isAcking, selected, onToggleSelect }: {
  item: AlarmListItem;
  onAcknowledge: () => void;
  onDelete: () => void;
  isAcking: boolean;
  selected: boolean;
  onToggleSelect: () => void;
}) {
  const sev = severityOf(item);
  return (
    <tr className={`alarm-row alarm-row-${sev}${selected ? ' alarm-row-selected' : ''}`}>
      <td className="col-cb">
        <input
          type="checkbox"
          className="alarm-select-cb"
          checked={selected}
          onChange={onToggleSelect}
          aria-label={`Označi alarm za ${item.object_name}`}
        />
      </td>
      <td className="col-state">
        {sev === 'ack'
          ? <span className="alarm-state alarm-state-ack">POTV</span>
          : <span className={`alarm-state alarm-state-${sev}`}>AKT</span>}
      </td>
      <td className="col-time" title={formatDistanceToNow(new Date(item.recorded_at), { addSuffix: true, locale: hr })}>
        {format(new Date(item.recorded_at), 'dd.MM.yyyy HH:mm:ss')}
      </td>
      <td className="col-object">
        <Link to={`/objects/${item.object_id}`} className="alarm-obj-link">{item.object_name}</Link>
        <code className="station-id">{item.station_id}</code>
        {item.location_name && <span className="alarm-row-loc"><MapPin size={11} />{item.location_name}</span>}
      </td>
      <td className="col-region">
        <span className="region-tag">
          <span className="region-dot" style={{ background: item.region_color }} />
          {item.region_name}
        </span>
      </td>
      <td className="col-alarms"><AlarmTags item={item} /></td>
      <td className="col-ackby">
        {item.acknowledged_at
          ? <span className="alarm-ack-info" title={format(new Date(item.acknowledged_at), 'dd.MM.yyyy HH:mm:ss')}>
              <CheckCircle size={11} /> {item.acknowledged_by || '—'}
              <span className="alarm-ack-time">{format(new Date(item.acknowledged_at), 'dd.MM. HH:mm')}</span>
            </span>
          : <span className="text-muted">—</span>}
      </td>
      <td className="col-actions">
        <div className="alarm-row-actions">
          <Link to={`/objects/${item.object_id}`} className="btn-secondary alarm-icon-btn" title="Pregledaj objekt">
            <ExternalLink size={13} />
          </Link>
          {!item.acknowledged_at && (
            <button className="btn-secondary alarm-icon-btn alarm-icon-btn-ack" onClick={onAcknowledge}
              disabled={isAcking} title="Potvrdi alarm">
              {isAcking ? <span className="spinner" style={{ width: 13, height: 13 }} /> : <Check size={13} />}
            </button>
          )}
          <button className="btn-danger alarm-icon-btn" onClick={onDelete} title="Briši zapis">
            <Trash2 size={13} />
          </button>
        </div>
      </td>
    </tr>
  );
}

// ── Alarm kartica (mobilni prikaz) ─────────────────────────────────────────
function AlarmCard({ item, onAcknowledge, onDelete, isAcking, selected, onToggleSelect }: {
  item: AlarmListItem;
  onAcknowledge: () => void;
  onDelete: () => void;
  isAcking: boolean;
  selected: boolean;
  onToggleSelect: () => void;
}) {
  const isAcknowledged = !!item.acknowledged_at;
  const critical = isCriticalAlarm(item);

  return (
    <div className={`alarm-card card ${selected ? 'alarm-card-selected' : ''} ${isAcknowledged ? 'alarm-card-ack' : critical ? 'alarm-card-critical' : 'alarm-card-warning'}`}>
      {/* Header: naziv + status badge */}
      <div className="alarm-card-header">
        <div className="alarm-card-title">
          <input
            type="checkbox"
            className="alarm-select-cb"
            checked={selected}
            onChange={onToggleSelect}
            aria-label={`Označi alarm za ${item.object_name}`}
          />
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
          {format(new Date(item.recorded_at), 'dd.MM.yyyy HH:mm:ss')}
          {' · '}
          {formatDistanceToNow(new Date(item.recorded_at), { addSuffix: true, locale: hr })}
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

  // Bulk odabir + potvrde
  const [selected, setSelected]         = useState<Set<number>>(new Set());
  const [bulkConfirm, setBulkConfirm]   = useState<null | 'ack' | 'delete'>(null);
  const [bulkBusy, setBulkBusy]         = useState(false);

  // Loading stanja po kartici
  const [pendingAck, setPendingAck]     = useState<Set<string>>(new Set());
  const [actionError, setActionError]   = useState('');

  const qc = useQueryClient();

  const { data, isLoading, error, isFetching, refetch, dataUpdatedAt } = useQuery({
    queryKey: ['alarms-history', status, regionFilter, page],
    queryFn: () => listAlarmHistory({
      status,
      region_id: regionFilter || undefined,
      page,
      page_size: 30,
    }),
    // SCADA: prikaz se osvježava uvijek, aktivni alarmi češće
    refetchInterval: status === 'active' ? 30_000 : 60_000,
    placeholderData: keepPreviousData,
  });

  // Zaseban upit za annunciator brojače — neovisan o filterima i paginaciji
  const { data: activeSummary } = useQuery({
    queryKey: ['alarms-active-summary'],
    queryFn: () => listAlarmHistory({ status: 'active', page: 1, page_size: 200 }),
    refetchInterval: 30_000,
  });

  const activeAlarms = activeSummary?.data ?? [];
  const criticalCount = activeAlarms.filter(isCriticalAlarm).length;
  const warningCount = activeAlarms.length - criticalCount;
  const activeTotal = activeSummary?.total ?? 0;

  const horn = useAlarmHorn(criticalCount);

  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const invalidateAlarmQueries = () => Promise.all([
    qc.invalidateQueries({ queryKey: ['alarms-history'] }),
    qc.invalidateQueries({ queryKey: ['alarms-active-summary'] }),
    qc.invalidateQueries({ queryKey: ['region-summary'] }),
  ]);

  const syncParams = (s: Status, r: string) => {
    const p: Record<string, string> = {};
    if (s !== 'active') p.status = s;
    if (r) p.region_id = r;
    setSearchParams(p);
  };

  const clearSelection = () => setSelected(new Set());
  const handleStatus = (s: Status) => { setStatus(s); setPage(1); clearSelection(); syncParams(s, regionFilter); };
  const handleRegion = (r: string) => { setRegionFilter(r); setPage(1); clearSelection(); syncParams(status, r); };

  // Potvrdi alarm
  const doAcknowledge = async () => {
    if (!ackTarget) return;
    setAckTarget(null);
    setActionError('');
    setPendingAck(prev => new Set(prev).add(ackTarget.object_id));
    try {
      await acknowledgeAlarm(ackTarget.object_id);
      await invalidateAlarmQueries();
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
      await invalidateAlarmQueries();
    } catch {
      setActionError(`Greška pri brisanju alarma. Pokušaj ponovo.`);
    }
  };

  const items = useMemo(() => data?.data ?? [], [data]);
  const totalPages = data?.total_pages ?? 1;
  const total = data?.total ?? 0;

  // Ako nakon brisanja/promjene filtera stranica ispadne iz raspona, vrati je u raspon
  useEffect(() => {
    if (data && page > Math.max(1, data.total_pages)) {
      setPage(Math.max(1, data.total_pages));
    }
  }, [data, page]);

  // Odabir ne smije preživjeti zapise koji više nisu na ekranu (refetch, paginacija)
  useEffect(() => {
    setSelected(prev => {
      const visible = new Set(items.map(i => i.id));
      const next = new Set([...prev].filter(id => visible.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [items]);

  // ── Bulk odabir ────────────────────────────────────────────────────────────
  const toggleSelect = (id: number) =>
    setSelected(prev => {
      const s = new Set(prev);
      if (s.has(id)) s.delete(id); else s.add(id);
      return s;
    });

  const allSelected = items.length > 0 && items.every(i => selected.has(i.id));
  const toggleSelectAll = () =>
    setSelected(allSelected ? new Set() : new Set(items.map(i => i.id)));

  const selectedItems = items.filter(i => selected.has(i.id));
  // Za potvrdu su relevantni samo još nepotvrđeni alarmi (ack ide po objektu)
  const selectedUnacked = selectedItems.filter(i => !i.acknowledged_at);

  // Masovna potvrda — dedupe po objektu jer ack potvrđuje sve alarme objekta
  const doBulkAcknowledge = async () => {
    setBulkConfirm(null);
    setActionError('');
    setBulkBusy(true);
    const objectIds = [...new Set(selectedUnacked.map(i => i.object_id))];
    const results = await Promise.allSettled(objectIds.map(id => acknowledgeAlarm(id)));
    const failed = results.filter(r => r.status === 'rejected').length;
    await invalidateAlarmQueries();
    setBulkBusy(false);
    clearSelection();
    if (failed > 0) setActionError(`${failed} od ${objectIds.length} potvrda nije uspjelo. Pokušaj ponovo.`);
  };

  // Masovno brisanje — po pojedinom zapisu alarma
  const doBulkDelete = async () => {
    setBulkConfirm(null);
    setActionError('');
    setBulkBusy(true);
    const ids = selectedItems.map(i => i.id);
    const results = await Promise.allSettled(ids.map(id => deleteAlarm(id)));
    const failed = results.filter(r => r.status === 'rejected').length;
    await invalidateAlarmQueries();
    setBulkBusy(false);
    clearSelection();
    if (failed > 0) setActionError(`${failed} od ${ids.length} brisanja nije uspjelo. Pokušaj ponovo.`);
  };

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

      {/* Bulk potvrda */}
      {bulkConfirm === 'ack' && (
        <ConfirmModal
          title="Masovna potvrda alarma"
          message={<>Potvrđuješ <strong>{selectedUnacked.length}</strong>{' '}
            {selectedUnacked.length === 1 ? 'nepotvrđeni alarm' : 'nepotvrđenih alarma'}.
            <br /><span style={{ color: 'var(--text2)', fontSize: 13 }}>
              Potvrda se primjenjuje po objektu i označava sve njegove aktivne alarme.
            </span></>}
          confirmLabel="Potvrdi označene"
          onConfirm={doBulkAcknowledge}
          onCancel={() => setBulkConfirm(null)}
        />
      )}
      {bulkConfirm === 'delete' && (
        <ConfirmModal
          title="Masovno brisanje alarma"
          danger
          message={<>Brišeš <strong>{selected.size}</strong>{' '}
            {selected.size === 1 ? 'označeni zapis' : 'označenih zapisa'} alarma.
            <br /><span style={{ color: 'var(--text2)', fontSize: 13 }}>
              Brišu se samo odabrani zapisi. Ovu akciju nije moguće poništiti.
            </span></>}
          confirmLabel="Briši označene"
          onConfirm={doBulkDelete}
          onCancel={() => setBulkConfirm(null)}
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

      {/* SCADA annunciator traka */}
      <ScadaBanner
        critical={criticalCount}
        warning={warningCount}
        activeTotal={activeTotal}
        updatedAt={dataUpdatedAt}
        onRefresh={() => { refetch(); qc.invalidateQueries({ queryKey: ['alarms-active-summary'] }); }}
        isFetching={isFetching}
        horn={horn}
      />

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

      {/* Bulk akcijska traka */}
      {items.length > 0 && (
        <div className="alarm-bulk-bar card">
          <label className="alarm-bulk-selectall">
            <input
              type="checkbox"
              checked={allSelected}
              ref={el => { if (el) el.indeterminate = selected.size > 0 && !allSelected; }}
              onChange={toggleSelectAll}
            />
            {selected.size > 0
              ? `${selected.size} ${selected.size === 1 ? 'odabran' : 'odabranih'}`
              : 'Označi sve'}
          </label>
          <div className="alarm-bulk-actions">
            <button
              className="btn-secondary alarm-action-btn"
              disabled={bulkBusy || selectedUnacked.length === 0}
              onClick={() => setBulkConfirm('ack')}
            >
              {bulkBusy
                ? <><span className="spinner" style={{ width: 13, height: 13 }} /> Obrada...</>
                : <><Check size={13} /> Potvrdi označene{selectedUnacked.length > 0 ? ` (${selectedUnacked.length})` : ''}</>}
            </button>
            <button
              className="btn-danger alarm-action-btn"
              disabled={bulkBusy || selected.size === 0}
              onClick={() => setBulkConfirm('delete')}
            >
              <Trash2 size={13} /> Briši označene{selected.size > 0 ? ` (${selected.size})` : ''}
            </button>
            {selected.size > 0 && (
              <button className="clear-filter-btn" onClick={clearSelection}>
                <X size={14} /> Poništi odabir
              </button>
            )}
          </div>
        </div>
      )}

      {/* SCADA tablica (desktop) */}
      {items.length > 0 && (
        <div className="alarm-table-wrap card">
          <table className="alarm-table">
            <thead>
              <tr>
                <th className="col-cb">
                  <input
                    type="checkbox"
                    className="alarm-select-cb"
                    checked={allSelected}
                    ref={el => { if (el) el.indeterminate = selected.size > 0 && !allSelected; }}
                    onChange={toggleSelectAll}
                    aria-label="Označi sve alarme"
                  />
                </th>
                <th className="col-state">Stanje</th>
                <th className="col-time">Vrijeme</th>
                <th className="col-object">Objekt</th>
                <th className="col-region">Regija</th>
                <th className="col-alarms">Alarmi</th>
                <th className="col-ackby">Potvrdio</th>
                <th className="col-actions">Akcije</th>
              </tr>
            </thead>
            <tbody>
              {items.map(item => (
                <AlarmRow
                  key={item.id}
                  item={item}
                  selected={selected.has(item.id)}
                  onToggleSelect={() => toggleSelect(item.id)}
                  isAcking={pendingAck.has(item.object_id)}
                  onAcknowledge={() => setAckTarget(item)}
                  onDelete={() => setDeleteTarget(item)}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Kartice (mobilni) */}
      {items.length > 0 && (
        <div className="alarm-list">
          {items.map(item => (
            <AlarmCard
              key={item.id}
              item={item}
              selected={selected.has(item.id)}
              onToggleSelect={() => toggleSelect(item.id)}
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
