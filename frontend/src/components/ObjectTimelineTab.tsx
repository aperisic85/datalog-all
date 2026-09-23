import { useQuery } from '@tanstack/react-query';
import {
  Activity, AlertTriangle, CheckCircle2, Clock3, Radio, Settings, UserRound,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { hr } from 'date-fns/locale';
import { getObjectTimeline } from '../api/endpoints';
import type { ObjectTimelineItem } from '../types';
import './ObjectTimelineTab.css';

function TimelineIcon({ item }: { item: ObjectTimelineItem }) {
  if (item.kind === 'alarm_raised') return <AlertTriangle size={16} />;
  if (item.kind === 'alarm_cleared') return <CheckCircle2 size={16} />;
  if (item.kind === 'event_log') return <Radio size={16} />;
  if (item.kind === 'audit') return <Settings size={16} />;
  return <Activity size={16} />;
}

function detailText(item: ObjectTimelineItem) {
  if (!item.details) return null;
  if (item.title === 'Poslana upravljačka naredba') {
    const table = item.details.table;
    const field = item.details.field;
    const value = item.details.value;
    if (table || field || value) return `${table ?? ''}.${field ?? ''} = ${value ?? ''}`;
  }
  if (item.title === 'Alarm odložen') {
    const duration = item.details.duration_minutes;
    const reason = item.details.reason;
    if (duration) return `${duration} min${reason ? ` · ${reason}` : ''}`;
  }
  return null;
}

export default function ObjectTimelineTab({ objectId }: { objectId: string }) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['object-timeline', objectId],
    queryFn: () => getObjectTimeline(objectId, 100),
    refetchInterval: 60_000,
  });

  if (isLoading) return <div className="page-spinner"><div className="spinner" /></div>;
  if (error) return <div className="error-msg">Greška pri učitavanju timelinea</div>;

  const items = data ?? [];

  return (
    <div className="object-timeline card">
      <div className="timeline-header">
        <div>
          <span className="timeline-kicker">Povijest objekta</span>
          <h3>Timeline</h3>
        </div>
        <span className="timeline-count">{items.length} događaja</span>
      </div>

      {items.length === 0 ? (
        <div className="timeline-empty">
          <Clock3 size={20} />
          <div>
            <strong>Nema zabilježenih događaja</strong>
            <span>Alarmi, logovi i operativne akcije pojavit će se ovdje.</span>
          </div>
        </div>
      ) : (
        <div className="timeline-list">
          {items.map((item) => {
            const extra = detailText(item);
            return (
              <div key={item.id} className={`timeline-item timeline-${item.severity}`}>
                <div className="timeline-rail">
                  <div className="timeline-icon"><TimelineIcon item={item} /></div>
                  <span className="timeline-line" />
                </div>
                <div className="timeline-body">
                  <div className="timeline-title-row">
                    <strong>{item.title}</strong>
                    <time title={new Date(item.occurred_at).toLocaleString('hr-HR')}>
                      {formatDistanceToNow(new Date(item.occurred_at), { addSuffix: true, locale: hr })}
                    </time>
                  </div>
                  {item.message && <p>{item.message}</p>}
                  {extra && <code className="timeline-detail">{extra}</code>}
                  <div className="timeline-meta">
                    <span><Clock3 size={11} />{new Date(item.occurred_at).toLocaleString('hr-HR')}</span>
                    {item.actor && <span><UserRound size={11} />{item.actor}</span>}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}