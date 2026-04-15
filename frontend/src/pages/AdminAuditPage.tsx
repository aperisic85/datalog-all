import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { listAuditLog } from '../api/endpoints';
import type { AuditLogEntry } from '../types';
import {
  ClipboardList, Filter, ChevronLeft, ChevronRight, X, User, Clock,
} from 'lucide-react';
import { format, parseISO } from 'date-fns';
import './AdminAuditPage.css';

const ACTION_LABELS: Record<string, { label: string; cls: string }> = {
  LOGIN:               { label: 'Prijava',          cls: 'badge-neutral'  },
  CHANGE_PASSWORD:     { label: 'Promjena lozinke', cls: 'badge-neutral'  },
  CREATE_REGION:       { label: 'Kreirana regija',  cls: 'badge-success'  },
  CREATE_OBJECT:       { label: 'Kreiran objekt',   cls: 'badge-success'  },
  ACKNOWLEDGE_ALARM:   { label: 'Potvrda alarma',   cls: 'badge-warning'  },
  DELETE_ALARM:        { label: 'Brisanje alarma',  cls: 'badge-danger'   },
  DELETE_ALARMS:       { label: 'Brisanje alarma (bulk)', cls: 'badge-danger' },
  GRANT_REGION_ACCESS: { label: 'Dodijeljen pristup', cls: 'badge-accent' },
  REVOKE_REGION_ACCESS:{ label: 'Uklonjen pristup',   cls: 'badge-danger' },
};

const ACTION_OPTIONS = [
  'LOGIN', 'CHANGE_PASSWORD', 'CREATE_REGION', 'CREATE_OBJECT',
  'ACKNOWLEDGE_ALARM', 'DELETE_ALARM', 'DELETE_ALARMS',
  'GRANT_REGION_ACCESS', 'REVOKE_REGION_ACCESS',
];

function DetailsCell({ details }: { details?: Record<string, unknown> }) {
  if (!details) return <span className="audit-no-data">—</span>;
  return (
    <span className="audit-details" title={JSON.stringify(details, null, 2)}>
      {Object.entries(details)
        .map(([k, v]) => `${k}: ${v}`)
        .join(', ')}
    </span>
  );
}

function EntityCell({ type: et, id: ei }: { type?: string; id?: string }) {
  if (!et && !ei) return <span className="audit-no-data">—</span>;
  return (
    <span>
      {et && <span className="audit-entity-type">{et}</span>}
      {ei && <code className="audit-entity-id">{ei}</code>}
    </span>
  );
}

export default function AdminAuditPage() {
  const [action,   setAction]   = useState('');
  const [username, setUsername] = useState('');
  const [from,     setFrom]     = useState('');
  const [to,       setTo]       = useState('');
  const [page,     setPage]     = useState(1);

  const params = {
    action:   action   || undefined,
    username: username || undefined,
    from:     from     ? new Date(from).toISOString()  : undefined,
    to:       to       ? new Date(to + 'T23:59:59').toISOString() : undefined,
    page,
    page_size: 50,
  };

  const { data, isLoading, error } = useQuery({
    queryKey: ['audit-log', params],
    queryFn:  () => listAuditLog(params),
  });

  const items: AuditLogEntry[] = data?.data ?? [];
  const totalPages = data?.total_pages ?? 1;
  const total      = data?.total ?? 0;

  const clearFilters = () => {
    setAction(''); setUsername(''); setFrom(''); setTo(''); setPage(1);
  };
  const hasFilters = !!(action || username || from || to);

  return (
    <div className="audit-page">
      <div className="page-header">
        <div>
          <h2><ClipboardList size={20} style={{ verticalAlign: -4, marginRight: 8 }} />Audit Log</h2>
          <span className="text-muted">
            {total > 0
              ? `${total} ${total === 1 ? 'zapis' : 'zapisa'}`
              : 'Nema zapisa'}
          </span>
        </div>
      </div>

      {/* ── Filteri ── */}
      <div className="audit-filters card">
        <Filter size={14} style={{ color: 'var(--text2)', flexShrink: 0 }} />

        <select
          value={action}
          onChange={(e) => { setAction(e.target.value); setPage(1); }}
        >
          <option value="">Sve akcije</option>
          {ACTION_OPTIONS.map((a) => (
            <option key={a} value={a}>{ACTION_LABELS[a]?.label ?? a}</option>
          ))}
        </select>

        <input
          placeholder="Korisničko ime..."
          value={username}
          onChange={(e) => { setUsername(e.target.value); setPage(1); }}
          className="audit-filter-input"
        />

        <div className="audit-date-range">
          <input
            type="date"
            value={from}
            onChange={(e) => { setFrom(e.target.value); setPage(1); }}
            title="Od datuma"
          />
          <span className="audit-date-sep">—</span>
          <input
            type="date"
            value={to}
            onChange={(e) => { setTo(e.target.value); setPage(1); }}
            title="Do datuma"
          />
        </div>

        {hasFilters && (
          <button className="clear-filter-btn" onClick={clearFilters}>
            <X size={13} /> Poništi
          </button>
        )}
      </div>

      {/* ── Loading / error ── */}
      {isLoading && <div className="page-spinner"><div className="spinner" /></div>}
      {error    && <div className="error-msg">Greška pri učitavanju audit loga</div>}

      {/* ── Empty ── */}
      {!isLoading && items.length === 0 && (
        <div className="audit-empty card">
          <ClipboardList size={32} style={{ color: 'var(--text3)' }} />
          <div>Nema zapisa za odabrani filter</div>
        </div>
      )}

      {/* ── Tablica ── */}
      {items.length > 0 && (
        <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
          <div className="table-scroll">
            <table className="audit-table">
              <thead>
                <tr>
                  <th><Clock size={12} /> Datum/Vrijeme</th>
                  <th><User size={12} /> Korisnik</th>
                  <th>Akcija</th>
                  <th>Entitet</th>
                  <th>Detalji</th>
                  <th>IP adresa</th>
                </tr>
              </thead>
              <tbody>
                {items.map((entry) => {
                  const actionMeta = ACTION_LABELS[entry.action];
                  return (
                    <tr key={entry.id}>
                      <td className="audit-time">
                        <div>{format(parseISO(entry.created_at), 'dd.MM.yyyy')}</div>
                        <div className="audit-time-sub">{format(parseISO(entry.created_at), 'HH:mm:ss')}</div>
                      </td>
                      <td>
                        <div className="audit-user">
                          <div className="audit-user-avatar">
                            {(entry.username?.[0] ?? '?').toUpperCase()}
                          </div>
                          <span>{entry.username ?? <span className="audit-no-data">—</span>}</span>
                        </div>
                      </td>
                      <td>
                        <span className={`badge ${actionMeta?.cls ?? 'badge-neutral'}`}>
                          {actionMeta?.label ?? entry.action}
                        </span>
                      </td>
                      <td>
                        <EntityCell type={entry.entity_type} id={entry.entity_id} />
                      </td>
                      <td>
                        <DetailsCell details={entry.details} />
                      </td>
                      <td className="audit-ip">
                        {entry.ip_address ?? <span className="audit-no-data">—</span>}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* ── Paginacija ── */}
      {totalPages > 1 && (
        <div className="alarm-pagination">
          <button
            className="btn-secondary"
            disabled={page <= 1}
            onClick={() => setPage((p) => p - 1)}
          >
            <ChevronLeft size={16} />
          </button>
          <span className="page-info">Strana {page} od {totalPages}</span>
          <button
            className="btn-secondary"
            disabled={page >= totalPages}
            onClick={() => setPage((p) => p + 1)}
          >
            <ChevronRight size={16} />
          </button>
        </div>
      )}
    </div>
  );
}
