import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Activity,
  AlertTriangle,
  Bell,
  CheckCircle2,
  Clock3,
  Database,
  Radio,
  Server,
  XCircle,
} from 'lucide-react';
import { getSystemHealth } from '../api/endpoints';
import './SystemHealthStatus.css';

function formatDate(value?: string) {
  if (!value) return '—';
  return new Date(value).toLocaleString('hr-HR');
}

function statusLabel(status: string) {
  switch (status) {
    case 'ok': return 'Radi';
    case 'degraded': return 'Provjeri';
    case 'critical':
    case 'error': return 'Greška';
    case 'not_configured': return 'Nije konfigurirano';
    default: return 'Nepoznato';
  }
}

function HealthIcon({ status }: { status: string }) {
  if (status === 'ok') return <CheckCircle2 size={15} />;
  if (status === 'degraded') return <AlertTriangle size={15} />;
  if (status === 'critical' || status === 'error') return <XCircle size={15} />;
  return <Clock3 size={15} />;
}

export default function SystemHealthStatus() {
  const [open, setOpen] = useState(false);

  const { data, isLoading, isError, isFetching, refetch } = useQuery({
    queryKey: ['system-health'],
    queryFn: getSystemHealth,
    refetchInterval: 60_000,
    staleTime: 30_000,
    retry: 1,
  });

  const overall = isError ? 'critical' : data?.status ?? 'unknown';
  const label = isLoading
    ? 'Provjera sustava...'
    : isError
      ? 'Sustav nedostupan'
      : overall === 'ok'
        ? 'Sustav uredan'
        : overall === 'critical'
          ? 'Kritična greška'
          : 'Sustav zahtijeva pažnju';

  return (
    <div className="system-health-shell">
      <div className="system-health-wrap">
        <button
          type="button"
          className={`system-health-trigger health-${overall}`}
          onClick={() => setOpen((value) => !value)}
          aria-expanded={open}
          title="Globalno stanje sustava"
        >
          <span className="system-health-dot" />
          <Activity size={14} />
          <span>{label}</span>
          {isFetching && !isLoading && <span className="system-health-pulse" />}
        </button>

        {open && (
          <div className="system-health-popover card">
            <div className="system-health-header">
              <div>
                <span className="system-health-kicker">AtoN Watch</span>
                <strong>Stanje sustava</strong>
              </div>
              <button
                type="button"
                className="system-health-refresh"
                onClick={() => refetch()}
                disabled={isFetching}
              >
                Osvježi
              </button>
            </div>

            {isError || !data ? (
              <div className="system-health-error">
                <XCircle size={18} />
                Backend health endpoint nije dostupan.
              </div>
            ) : (
              <div className="system-health-list">
                <div className={`system-health-row status-${data.api.status}`}>
                  <Server size={16} />
                  <div>
                    <strong>Backend API</strong>
                    <span>{data.api.message}</span>
                  </div>
                  <span className="health-value"><HealthIcon status={data.api.status} />{statusLabel(data.api.status)}</span>
                </div>

                <div className={`system-health-row status-${data.database.status}`}>
                  <Database size={16} />
                  <div>
                    <strong>Baza podataka</strong>
                    <span>{data.database.message}</span>
                  </div>
                  <span className="health-value"><HealthIcon status={data.database.status} />{statusLabel(data.database.status)}</span>
                </div>

                <div className={`system-health-row ${data.pollers.late > 0 || data.pollers.offline > 0 ? 'status-degraded' : 'status-ok'}`}>
                  <Radio size={16} />
                  <div>
                    <strong>Polleri</strong>
                    <span>{data.pollers.online}/{data.pollers.total} online · {data.pollers.late} kasni</span>
                  </div>
                  <span className="health-value">
                    <HealthIcon status={data.pollers.late > 0 || data.pollers.offline > 0 ? 'degraded' : 'ok'} />
                    {data.pollers.offline > 0 ? `${data.pollers.offline} offline` : 'Uredno'}
                  </span>
                </div>

                <div className="system-health-row status-neutral">
                  <Clock3 size={16} />
                  <div>
                    <strong>Zadnji uspješni poll</strong>
                    <span>Posljednji potvrđeni dohvat nekog periodičnog pollera</span>
                  </div>
                  <span className="health-value health-time">{formatDate(data.pollers.last_successful_poll)}</span>
                </div>

                <div className={`system-health-row ${data.data.stale_objects > 0 ? 'status-degraded' : 'status-ok'}`}>
                  <AlertTriangle size={16} />
                  <div>
                    <strong>Svježina podataka</strong>
                    <span>{data.data.active_objects} aktivnih objekata</span>
                  </div>
                  <span className="health-value">
                    <HealthIcon status={data.data.stale_objects > 0 ? 'degraded' : 'ok'} />
                    {data.data.stale_objects} bez svježih podataka
                  </span>
                </div>

                <div className={`system-health-row status-${data.notifications.status}`}>
                  <Bell size={16} />
                  <div>
                    <strong>Obavijesti</strong>
                    <span>
                      {data.notifications.enabled_channels} aktivnih kanala · {data.notifications.failed_last_24h} grešaka / 24 h
                    </span>
                  </div>
                  <span className="health-value">
                    <HealthIcon status={data.notifications.status} />
                    {statusLabel(data.notifications.status)}
                  </span>
                </div>
              </div>
            )}

            {data && (
              <div className="system-health-footer">
                <span>Provjereno: {formatDate(data.checked_at)}</span>
                {data.notifications.last_success_at && (
                  <span>Zadnja obavijest: {formatDate(data.notifications.last_success_at)}</span>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
