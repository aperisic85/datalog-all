import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  startOfWeek, subDays, format, parseISO,
  eachDayOfInterval, getMonth,
} from 'date-fns';
import { bs } from 'date-fns/locale';
import { getAlarmHeatmap } from '../api/endpoints';
import type { AlarmHeatmapDay } from '../types';
import './AlarmHeatmapTab.css';

// ── Konstante ────────────────────────────────────────────────────────────────

const DOW_LABELS = ['Pon', 'Uto', 'Sri', 'Čet', 'Pet', 'Sub', 'Ned'];
const HOUR_LABELS = Array.from({ length: 24 }, (_, i) => `${String(i).padStart(2, '0')}h`);
const MONTH_LABELS = ['Sij', 'Velj', 'Ožu', 'Tra', 'Svi', 'Lip', 'Srp', 'Kol', 'Ruj', 'Lis', 'Stu', 'Pro'];

type View = 'yearly' | 'hourly';

// ── Boja ćelije ──────────────────────────────────────────────────────────────

/** Vraća CSS boju za godišnji heatmap (0 = mirno, visoki = puno alarma) */
function dayColor(count: number): string {
  if (count === 0) return 'var(--hm-empty)';
  if (count <= 3)  return 'var(--hm-l1)';
  if (count <= 10) return 'var(--hm-l2)';
  if (count <= 30) return 'var(--hm-l3)';
  return 'var(--hm-l4)';
}

/** Vraća CSS boju za hour-of-day heatmap (0.0–1.0 udio) */
function hourColor(frac: number): string {
  if (frac === 0)      return 'var(--hm-empty)';
  if (frac <= 0.05)    return 'var(--hm-l1)';
  if (frac <= 0.15)    return 'var(--hm-l2)';
  if (frac <= 0.35)    return 'var(--hm-l3)';
  return 'var(--hm-l4)';
}

// ── Godišnji kalendarski heatmap ─────────────────────────────────────────────

function YearlyHeatmap({ daily }: { daily: AlarmHeatmapDay[] }) {
  const [tooltip, setTooltip] = useState<{ x: number; y: number; text: string } | null>(null);

  const { weeks, monthPositions, maxCount } = useMemo(() => {
    const today = new Date();
    const yearAgo = subDays(today, 364);

    // Počnemo od ponedjeljka tjedna koji sadrži yearAgo
    const gridStart = startOfWeek(yearAgo, { weekStartsOn: 1 });

    // Sve dane od gridStart do today
    const allDays = eachDayOfInterval({ start: gridStart, end: today });

    // Mapa datum → count
    const countMap = new Map<string, number>();
    for (const d of daily) {
      countMap.set(d.date, d.count);
    }

    let maxCount = 0;
    for (const d of daily) {
      if (d.count > maxCount) maxCount = d.count;
    }

    // Grupiraj po tjednima (stupci), svaki tjedan ima 7 dana (redovi = pon–ned)
    const weeks: { date: Date; count: number; inRange: boolean }[][] = [];
    for (let i = 0; i < allDays.length; i += 7) {
      const week = allDays.slice(i, i + 7).map(d => ({
        date: d,
        count: countMap.get(format(d, 'yyyy-MM-dd')) ?? 0,
        inRange: d >= yearAgo && d <= today,
      }));
      weeks.push(week);
    }

    // Pozicije etiketa mjeseci — prvi tjedan koji počinje novim mjesecom
    const seen = new Set<number>();
    const monthPositions: { weekIdx: number; label: string }[] = [];
    for (let wi = 0; wi < weeks.length; wi++) {
      const firstDay = weeks[wi][0]; // ponedjeljak tog tjedna
      const m = getMonth(firstDay.date);
      if (!seen.has(m)) {
        seen.add(m);
        monthPositions.push({ weekIdx: wi, label: MONTH_LABELS[m] });
      }
    }

    return { weeks, monthPositions, maxCount };
  }, [daily]);

  const CELL = 13; // px — veličina jedne ćelije
  const GAP  =  2; // px — razmak

  return (
    <div className="hm-yearly-wrap">
      {/* Legenda */}
      <div className="hm-legend-row">
        <span className="hm-legend-label">Manje alarma</span>
        <span className="hm-swatch" style={{ background: 'var(--hm-empty)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l1)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l2)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l3)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l4)' }} />
        <span className="hm-legend-label">Više alarma</span>
        {maxCount > 0 && (
          <span className="hm-legend-label" style={{ marginLeft: 'auto', opacity: 0.6 }}>
            maks. {maxCount} perioda/dan
          </span>
        )}
      </div>

      <div className="hm-scroll">
        {/* Etikete dana u tjednu (lijeva os) */}
        <div className="hm-dow-col" style={{ '--cell': `${CELL}px`, '--gap': `${GAP}px` } as React.CSSProperties}>
          <div className="hm-dow-spacer" /> {/* Prostor za etikete mjeseci */}
          {DOW_LABELS.map((d, i) => (
            <div key={i} className={`hm-dow-label ${i % 2 === 1 ? 'visible' : ''}`}
              style={{ height: CELL, marginBottom: GAP }}>
              {i % 2 === 1 ? d : ''}
            </div>
          ))}
        </div>

        {/* Stupci tjedana */}
        <div className="hm-grid">
          {/* Etikete mjeseci */}
          <div className="hm-month-row" style={{ width: weeks.length * (CELL + GAP) }}>
            {monthPositions.map(({ weekIdx, label }) => (
              <span
                key={label}
                className="hm-month-label"
                style={{ left: weekIdx * (CELL + GAP) }}
              >
                {label}
              </span>
            ))}
          </div>

          {/* Ćelije */}
          <div className="hm-cells-row">
            {weeks.map((week, wi) => (
              <div key={wi} className="hm-week-col">
                {week.map((cell, di) => (
                  <div
                    key={di}
                    className={`hm-cell ${!cell.inRange ? 'hm-cell-out' : ''}`}
                    style={{
                      width: CELL,
                      height: CELL,
                      marginBottom: GAP,
                      background: cell.inRange ? dayColor(cell.count) : 'transparent',
                    }}
                    onMouseEnter={e => {
                      const r = (e.target as HTMLDivElement).getBoundingClientRect();
                      const wrap = (e.target as HTMLDivElement).closest('.hm-scroll')!.getBoundingClientRect();
                      setTooltip({
                        x: r.left - wrap.left + CELL / 2,
                        y: r.top - wrap.top - 8,
                        text: cell.inRange
                          ? `${format(cell.date, 'dd. MMMM yyyy.', { locale: bs })} — ${cell.count === 0 ? 'nema alarma' : `${cell.count} perioda s alarmom`}`
                          : '',
                      });
                    }}
                    onMouseLeave={() => setTooltip(null)}
                  />
                ))}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Tooltip */}
      {tooltip && tooltip.text && (
        <div
          className="hm-tooltip"
          style={{ left: tooltip.x, top: tooltip.y }}
          aria-hidden
        >
          {tooltip.text}
        </div>
      )}
    </div>
  );
}

// ── Hour-of-day × Day-of-week heatmap ────────────────────────────────────────

function HourlyHeatmap({ hourly }: { hourly: { hour: number; dow: number; count: number }[] }) {
  const [tooltip, setTooltip] = useState<{ x: number; y: number; text: string } | null>(null);

  // Postavi u 24×7 matricu
  const matrix = useMemo(() => {
    const m: number[][] = Array.from({ length: 24 }, () => Array(7).fill(0));
    for (const h of hourly) {
      m[h.hour][h.dow] = h.count;
    }
    return m;
  }, [hourly]);

  const hasData = hourly.length > 0;

  return (
    <div className="hm-hourly-wrap">
      <p className="hm-hourly-subtitle">
        Prosječna učestalost alarma po satu i danu (zadnjih 90 dana)
      </p>

      {/* Legenda */}
      <div className="hm-legend-row">
        <span className="hm-legend-label">Rijetko</span>
        <span className="hm-swatch" style={{ background: 'var(--hm-empty)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l1)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l2)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l3)' }} />
        <span className="hm-swatch" style={{ background: 'var(--hm-l4)' }} />
        <span className="hm-legend-label">Često</span>
      </div>

      {!hasData ? (
        <div className="hm-nodata">Nema dovoljno podataka za hour-of-day analizu (zadnjih 90 dana)</div>
      ) : (
        <div className="hm-hourly-grid-wrap">
          {/* Zaglavlje — dani u tjednu */}
          <div className="hm-hourly-grid">
            <div className="hm-h-corner" />
            {DOW_LABELS.map(d => (
              <div key={d} className="hm-h-dow">{d}</div>
            ))}

            {/* Redovi po satu */}
            {matrix.map((rowCounts, hour) => (
              <>
                <div key={`lbl-${hour}`} className="hm-h-hour">{HOUR_LABELS[hour]}</div>
                {rowCounts.map((count, dow) => (
                  <div
                    key={`${hour}-${dow}`}
                    className="hm-h-cell"
                    style={{ background: hourColor(count) }}
                    onMouseEnter={e => {
                      const r = (e.target as HTMLDivElement).getBoundingClientRect();
                      const wrap = (e.target as HTMLDivElement).closest('.hm-hourly-grid-wrap')!.getBoundingClientRect();
                      setTooltip({
                        x: r.left - wrap.left + r.width / 2,
                        y: r.top - wrap.top - 8,
                        text: `${DOW_LABELS[dow]}, ${HOUR_LABELS[hour]} — ${(count * 100).toFixed(1)}% perioda s alarmom`,
                      });
                    }}
                    onMouseLeave={() => setTooltip(null)}
                  />
                ))}
              </>
            ))}
          </div>

          {/* Tooltip */}
          {tooltip && (
            <div
              className="hm-tooltip"
              style={{ left: tooltip.x, top: tooltip.y }}
              aria-hidden
            >
              {tooltip.text}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Statistike uz heatmap ────────────────────────────────────────────────────

function HeatmapStats({ daily }: { daily: AlarmHeatmapDay[] }) {
  const stats = useMemo(() => {
    if (daily.length === 0) return null;

    const total = daily.reduce((s, d) => s + d.count, 0);
    const alarmDays = daily.filter(d => d.count > 0).length;
    const maxDay = daily.reduce((a, b) => b.count > a.count ? b : a, daily[0]);
    const last30 = daily.filter(d => {
      const dt = parseISO(d.date);
      return dt >= subDays(new Date(), 30);
    });
    const last30Total = last30.reduce((s, d) => s + d.count, 0);

    return { total, alarmDays, maxDay, last30Total, totalDays: daily.length };
  }, [daily]);

  if (!stats) return null;

  return (
    <div className="hm-stats">
      <div className="hm-stat">
        <span className="hm-stat-val">{stats.alarmDays}</span>
        <span className="hm-stat-lbl">dana s alarmom / {stats.totalDays} ukupno</span>
      </div>
      <div className="hm-stat">
        <span className="hm-stat-val">{stats.total.toLocaleString()}</span>
        <span className="hm-stat-lbl">alarm perioda (365 dana)</span>
      </div>
      <div className="hm-stat">
        <span className="hm-stat-val">{stats.last30Total.toLocaleString()}</span>
        <span className="hm-stat-lbl">alarm perioda (zadnjih 30 dana)</span>
      </div>
      {stats.maxDay.count > 0 && (
        <div className="hm-stat">
          <span className="hm-stat-val">{stats.maxDay.count}</span>
          <span className="hm-stat-lbl">
            maks. perioda — {format(parseISO(stats.maxDay.date), 'dd. MMM yyyy.', { locale: bs })}
          </span>
        </div>
      )}
    </div>
  );
}

// ── Glavni export ─────────────────────────────────────────────────────────────

export default function AlarmHeatmapTab({ objectId }: { objectId: string }) {
  const [view, setView] = useState<View>('yearly');

  const { data, isLoading, isError } = useQuery({
    queryKey: ['alarm-heatmap', objectId],
    queryFn: () => getAlarmHeatmap(objectId),
    staleTime: 10 * 60_000,
  });

  if (isLoading) {
    return <div className="page-spinner"><div className="spinner" /></div>;
  }

  if (isError || !data) {
    return <div className="no-data">Greška pri učitavanju heatmap podataka</div>;
  }

  const hasAnyData = data.daily.length > 0 || data.hourly.length > 0;

  return (
    <div className="hm-tab">
      {/* Odabir prikaza */}
      <div className="hm-view-selector range-selector" style={{ marginBottom: 16 }}>
        <button
          className={`filter-tab ${view === 'yearly' ? 'active' : ''}`}
          onClick={() => setView('yearly')}
        >
          Godišnji pregled
        </button>
        <button
          className={`filter-tab ${view === 'hourly' ? 'active' : ''}`}
          onClick={() => setView('hourly')}
        >
          Doba dana
        </button>
      </div>

      {!hasAnyData ? (
        <div className="card hm-empty-card">
          <div className="no-data" style={{ padding: '32px 0' }}>
            Nema dovoljno alarm podataka za prikaz heatmapa
          </div>
        </div>
      ) : (
        <>
          {/* Statistike */}
          {data.daily.length > 0 && <HeatmapStats daily={data.daily} />}

          {/* Heatmap prikaz */}
          <div className="card" style={{ padding: 16, marginTop: 12 }}>
            {view === 'yearly' ? (
              <>
                <div className="hm-section-title">
                  Kalendarski prikaz — zadnjih 365 dana
                </div>
                <YearlyHeatmap daily={data.daily} />
              </>
            ) : (
              <>
                <div className="hm-section-title">
                  Heatmap po dobu dana i danu u tjednu
                </div>
                <HourlyHeatmap hourly={data.hourly} />
              </>
            )}
          </div>
        </>
      )}
    </div>
  );
}
