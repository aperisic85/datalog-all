import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useSearchParams, Link } from 'react-router-dom';
import { listObjects, listRegions, acknowledgeAlarm, deleteAlarms } from '../api/endpoints';
import type { ObjectView } from '../types';
import {
  AlertTriangle, Battery, Wifi, WifiOff,
  MapPin, Thermometer, Zap, Check, Trash2, ExternalLink, Filter,
} from 'lucide-react';
import './AlarmsPage.css';

// Mapiranje alarm polja → čitljivi opisi
const ALARM_LABELS: Record<string, { label: string; icon: React.ReactNode; severity: 'danger' | 'warning' }> = {
  alarm_battery_voltage_flat:   { label: 'Baterija prazna',         icon: <Battery size={14} />,    severity: 'danger' },
  alarm_battery_voltage_low:    { label: 'Baterija slaba',          icon: <Battery size={14} />,    severity: 'warning' },
  alarm_battery_other_error:    { label: 'Greška baterije',         icon: <Battery size={14} />,    severity: 'warning' },
  alarm_datalogger_high_temp:   { label: 'Visoka temp. datalogera', icon: <Thermometer size={14} />, severity: 'warning' },
  alarm_datalogger_high_voltage:{ label: 'Visoki napon datalogera', icon: <Zap size={14} />,        severity: 'warning' },
  alarm_datalogger_other_error: { label: 'Greška datalogera',       icon: <AlertTriangle size={14} />, severity: 'warning' },
  alarm_garmin_comm_failed:     { label: 'GPS komunikacija pala',   icon: <MapPin size={14} />,     severity: 'danger' },
  alarm_garmin_other_error:     { label: 'GPS greška',              icon: <MapPin size={14} />,     severity: 'warning' },
  alarm_station_out_of_radius:  { label: 'Stanica van radijusa',    icon: <MapPin size={14} />,     severity: 'danger' },
  alarm_lantern_night_light_off:{ label: 'Fenjer ugašen noću',      icon: <Zap size={14} />,        severity: 'danger' },
  alarm_lantern_day_light_on:   { label: 'Fenjer upaljen danju',    icon: <Zap size={14} />,        severity: 'warning' },
  alarm_lantern_comm_failed:    { label: 'Fenjer komunikacija pala',icon: <WifiOff size={14} />,    severity: 'danger' },
  alarm_lantern_other_error:    { label: 'Fenjer greška',           icon: <Zap size={14} />,        severity: 'warning' },
  alarm_modem_network_error:    { label: 'Greška mreže',            icon: <Wifi size={14} />,       severity: 'warning' },
  alarm_modem_other_error:      { label: 'Greška modema',           icon: <WifiOff size={14} />,    severity: 'warning' },
  alarm_station_other_error:    { label: 'Greška stanice',          icon: <AlertTriangle size={14} />, severity: 'warning' },
};

function alarmLevelLabel(level?: number | null) {
  if (!level) return null;
  if (level >= 4) return <span className="badge badge-danger badge-pulse">Kritično</span>;
  if (level >= 3) return <span className="badge badge-danger">Opasnost</span>;
  if (level >= 2) return <span className="badge badge-warning">Upozorenje</span>;
  return <span className="badge badge-neutral">Info</span>;
}

function ActiveAlarmTags({ summary }: { summary?: string | null }) {
  if (!summary) return null;
  const parts = summary.split(',').map(s => s.trim()).filter(Boolean);
  return (
    <div className="alarm-tag-list">
      {parts.map((p, i) => {
        const def = ALARM_LABELS[p];
        if (!def) return <span key={i} className={`alarm-tag alarm-tag-warning`}>{p}</span>;
        return (
          <span key={i} className={`alarm-tag alarm-tag-${def.severity}`}>
            {def.icon} {def.label}
          </span>
        );
      })}
    </div>
  );
}

function AlarmCard({ obj, onAcknowledge, onDelete }: {
  obj: ObjectView;
  onAcknowledge: (id: string) => void;
  onDelete: (id: string, name: string) => void;
}) {
  return (
    <div className={`alarm-card card ${(obj.alarm_worst_level ?? 0) >= 3 ? 'alarm-card-critical' : 'alarm-card-warning'}`}>
      <div className="alarm-card-header">
        <div className="alarm-card-title">
          <span className="status-dot status-dot-alarm" />
          <span className="alarm-obj-name">{obj.name}</span>
        </div>
        <div className="alarm-card-level">
          {alarmLevelLabel(obj.alarm_worst_level)}
        </div>
      </div>

      <div className="alarm-card-meta">
        <span className="region-tag">
          <span className="region-dot" style={{ background: obj.region_color }} />
          {obj.region_name}
        </span>
        {obj.location_name && (
          <span className="location-cell">
            <MapPin size={12} /> {obj.location_name}
          </span>
        )}
        <code className="station-id">{obj.station_id}</code>
      </div>

      {obj.alarm_summary && (
        <ActiveAlarmTags summary={obj.alarm_summary} />
      )}

      {obj.alarm_last_seen_at && (
        <div className="alarm-time">
          Zadnji alarm: {new Date(obj.alarm_last_seen_at).toLocaleString('bs-BA')}
        </div>
      )}

      <div className="alarm-card-actions">
        <Link to={`/objects/${obj.id}`} className="btn-secondary alarm-action-btn">
          <ExternalLink size={14} /> Pregledaj
        </Link>
        <button className="btn-secondary alarm-action-btn" onClick={() => onAcknowledge(obj.id)}>
          <Check size={14} /> Potvrdi
        </button>
        <button className="btn-danger alarm-action-btn" onClick={() => onDelete(obj.id, obj.name)}>
          <Trash2 size={14} /> Briši alarme
        </button>
      </div>
    </div>
  );
}

export default function AlarmsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [regionFilter, setRegionFilter] = useState(searchParams.get('region_id') || '');
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [, setActionLoading] = useState<string | null>(null);
  const qc = useQueryClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ['objects-alarms', regionFilter],
    queryFn: () => listObjects({ in_alarm: true, page_size: 100, region_id: regionFilter || undefined }),
    refetchInterval: 60_000,
  });

  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const handleRegionFilter = (val: string) => {
    setRegionFilter(val);
    if (val) setSearchParams({ region_id: val });
    else setSearchParams({});
  };

  const handleAcknowledge = async (id: string) => {
    setActionLoading(id + '-ack');
    try {
      await acknowledgeAlarm(id);
      qc.invalidateQueries({ queryKey: ['objects-alarms'] });
      qc.invalidateQueries({ queryKey: ['region-summary'] });
    } finally {
      setActionLoading(null);
    }
  };

  const handleDeleteConfirm = async () => {
    if (!confirmDelete) return;
    setActionLoading(confirmDelete.id + '-del');
    setConfirmDelete(null);
    try {
      await deleteAlarms(confirmDelete.id);
      qc.invalidateQueries({ queryKey: ['objects-alarms'] });
      qc.invalidateQueries({ queryKey: ['region-summary'] });
    } finally {
      setActionLoading(null);
    }
  };

  const alarmed = data?.data ?? [];

  return (
    <div className="alarms-page">
      {/* Confirm delete dialog */}
      {confirmDelete && (
        <div className="modal-overlay" onClick={() => setConfirmDelete(null)}>
          <div className="modal-box card" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>Brisanje alarma</h3>
            </div>
            <p style={{ margin: '0 0 20px', color: 'var(--text2)' }}>
              Sigurno želiš obrisati sve alarm zapise za objekt <strong style={{ color: 'var(--text)' }}>{confirmDelete.name}</strong>?
              Ova radnja je nepovratna.
            </p>
            <div className="modal-actions">
              <button className="btn-secondary" onClick={() => setConfirmDelete(null)}>Odustani</button>
              <button className="btn-danger" onClick={handleDeleteConfirm}>
                <Trash2 size={14} /> Briši alarme
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="page-header">
        <div>
          <h2>Alarmi</h2>
          <span className="text-muted">
            {alarmed.length > 0 ? `${alarmed.length} ${alarmed.length === 1 ? 'objekt u alarmu' : 'objekata u alarmu'}` : 'Nema aktivnih alarma'}
          </span>
        </div>
      </div>

      {/* Filters */}
      <div className="alarms-filters card">
        <Filter size={15} style={{ color: 'var(--text2)', flexShrink: 0 }} />
        <select value={regionFilter} onChange={(e) => handleRegionFilter(e.target.value)}>
          <option value="">Sve regije</option>
          {regions?.map((r) => (
            <option key={r.id} value={r.id}>{r.name}</option>
          ))}
        </select>
        {regionFilter && (
          <button className="btn-secondary" style={{ padding: '6px 12px', fontSize: 13 }} onClick={() => handleRegionFilter('')}>
            Sve regije
          </button>
        )}
      </div>

      {isLoading && <div className="page-spinner"><div className="spinner" /></div>}
      {error && <div className="error-msg">Greška pri učitavanju alarma</div>}

      {!isLoading && alarmed.length === 0 && (
        <div className="alarms-empty">
          <div className="alarms-empty-icon"><AlertTriangle size={40} /></div>
          <div className="alarms-empty-title">Nema aktivnih alarma</div>
          <div className="alarms-empty-sub">Svi objekti rade normalno</div>
        </div>
      )}

      {alarmed.length > 0 && (
        <>
          <div className="alarms-summary">
            <span className="badge badge-danger badge-pulse">
              <AlertTriangle size={12} />
              {alarmed.filter(o => (o.alarm_worst_level ?? 0) >= 3).length} kritičnih
            </span>
            <span className="badge badge-warning">
              <AlertTriangle size={12} />
              {alarmed.filter(o => (o.alarm_worst_level ?? 0) < 3).length} upozorenja
            </span>
            <span style={{ fontSize: 12, color: 'var(--text2)' }}>
              Osvježava se svake minute
            </span>
          </div>

          <div className="alarm-list">
            {alarmed
              .sort((a, b) => (b.alarm_worst_level ?? 0) - (a.alarm_worst_level ?? 0))
              .map((obj) => (
                <AlarmCard
                  key={obj.id}
                  obj={obj}
                  onAcknowledge={handleAcknowledge}
                  onDelete={(id, name) => setConfirmDelete({ id, name })}
                />
              ))}
          </div>
        </>
      )}
    </div>
  );
}
