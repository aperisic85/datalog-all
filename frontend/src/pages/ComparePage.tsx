import { useState, useMemo, useRef, useEffect } from 'react';
import { useQuery, useQueries } from '@tanstack/react-query';
import { listObjects, getMeasurements10min, getMeasurements1h } from '../api/endpoints';
import type { ObjectView, Measurement10min, Measurement1h } from '../types';
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
import { format, parseISO } from 'date-fns';
import { GitCompare, X, Search, Info } from 'lucide-react';
import './ComparePage.css';

type Range = '6h' | '24h' | '7d';

const STATION_COLORS = [
  { stroke: 'var(--accent)',   bg: 'rgba(59,130,246,0.15)' },
  { stroke: 'var(--success)',  bg: 'rgba(34,197,94,0.15)'  },
  { stroke: 'var(--warning)',  bg: 'rgba(245,158,11,0.15)' },
  { stroke: 'var(--danger)',   bg: 'rgba(239,68,68,0.15)'  },
];

const MAX_STATIONS = 4;
const RANGES: Range[] = ['6h', '24h', '7d'];

// ────────────────────────────────────────────────────────────────────────────
// Station chip
// ────────────────────────────────────────────────────────────────────────────
function StationChip({
  obj,
  colorIdx,
  onRemove,
}: {
  obj: ObjectView;
  colorIdx: number;
  onRemove: () => void;
}) {
  const { stroke, bg } = STATION_COLORS[colorIdx];
  return (
    <div className="station-chip" style={{ '--chip-stroke': stroke, '--chip-bg': bg } as React.CSSProperties}>
      <span className="chip-dot" style={{ background: stroke }} />
      <span className="chip-name">{obj.name}</span>
      <span className="chip-id">{obj.station_id}</span>
      <button className="chip-remove" onClick={onRemove} title="Ukloni">
        <X size={12} />
      </button>
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Station search dropdown
// ────────────────────────────────────────────────────────────────────────────
function StationSearch({
  all,
  selected,
  onAdd,
}: {
  all: ObjectView[];
  selected: ObjectView[];
  onAdd: (obj: ObjectView) => void;
}) {
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);

  const selectedIds = new Set(selected.map((o) => o.id));
  const filtered = all.filter(
    (o) =>
      !selectedIds.has(o.id) &&
      (query === '' ||
        o.name.toLowerCase().includes(query.toLowerCase()) ||
        o.station_id.toLowerCase().includes(query.toLowerCase()) ||
        (o.location_name ?? '').toLowerCase().includes(query.toLowerCase())),
  );

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  return (
    <div className="station-search-wrap" ref={wrapRef}>
      <div className="station-search-input-row">
        <Search size={14} className="search-icon" />
        <input
          className="station-search-input"
          type="text"
          placeholder="Pretraži stanicu…"
          value={query}
          onChange={(e) => { setQuery(e.target.value); setOpen(true); }}
          onFocus={() => setOpen(true)}
        />
      </div>
      {open && filtered.length > 0 && (
        <div className="station-dropdown">
          {filtered.slice(0, 20).map((obj) => (
            <button
              key={obj.id}
              className="station-dropdown-item"
              onMouseDown={(e) => { e.preventDefault(); onAdd(obj); setQuery(''); setOpen(false); }}
            >
              <span
                className="dd-region-dot"
                style={{ background: obj.region_color || 'var(--border)' }}
              />
              <span className="dd-name">{obj.name}</span>
              <span className="dd-meta">{obj.station_id} · {obj.region_name}</span>
              {obj.alarm_active && <span className="dd-alarm">!</span>}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Merged chart data
// ────────────────────────────────────────────────────────────────────────────
type MergedPoint = { time: string; [key: string]: number | string | undefined };

function mergeData(
  dataList: (Measurement10min[] | Measurement1h[])[],
  ids: string[],
  range: Range,
): MergedPoint[] {
  const timeMap = new Map<string, MergedPoint>();
  const timeFmt = range === '7d' ? 'dd.MM HH:mm' : 'HH:mm';

  dataList.forEach((data, idx) => {
    const id = ids[idx];
    (data as Measurement10min[]).forEach((m) => {
      const t = format(parseISO(m.recorded_at), timeFmt);
      if (!timeMap.has(t)) timeMap.set(t, { time: t });
      const pt = timeMap.get(t)!;
      pt[`bv_${id}`] = m.battery_voltage_avg ?? undefined;
      pt[`sv_${id}`] = m.solar_voltage_avg ?? undefined;
      pt[`tp_${id}`] = m.datalogger_temp_avg ?? undefined;
      pt[`bc_${id}`] = m.battery_current_avg ?? undefined;
    });
  });

  // Sort by original recorded_at order (map preserves insertion order for first station)
  const allTimes = Array.from(timeMap.keys());
  return allTimes.map((t) => timeMap.get(t)!);
}

// ────────────────────────────────────────────────────────────────────────────
// Compare chart card
// ────────────────────────────────────────────────────────────────────────────
function CompareChart({
  title,
  data,
  metric,
  stations,
}: {
  title: string;
  data: MergedPoint[];
  metric: 'bv' | 'sv' | 'tp' | 'bc';
  stations: ObjectView[];
}) {
  if (data.length === 0) return null;

  return (
    <div className="compare-chart-card card">
      <h4>{title}</h4>
      <ResponsiveContainer width="100%" height={200}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
          <XAxis dataKey="time" tick={{ fontSize: 10, fill: 'var(--text2)' }} interval="preserveStartEnd" />
          <YAxis tick={{ fontSize: 10, fill: 'var(--text2)' }} width={40} />
          <Tooltip
            contentStyle={{ background: 'var(--bg2)', border: '1px solid var(--border)', borderRadius: 6, fontSize: 12 }}
          />
          <Legend wrapperStyle={{ fontSize: 11 }} />
          {stations.map((obj, idx) => (
            <Line
              key={obj.id}
              type="monotone"
              dataKey={`${metric}_${obj.id}`}
              stroke={STATION_COLORS[idx].stroke}
              dot={false}
              name={obj.name}
              connectNulls
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Main page
// ────────────────────────────────────────────────────────────────────────────
export default function ComparePage() {
  const [selected, setSelected] = useState<ObjectView[]>([]);
  const [range, setRange] = useState<Range>('24h');

  // Load all objects for the selector (up to 500)
  const { data: page, isLoading: loadingObjects } = useQuery({
    queryKey: ['objects-compare'],
    queryFn: () => listObjects({ page_size: 500 }),
    staleTime: 60_000,
  });
  const allObjects = page?.data ?? [];

  // Fetch measurements for each selected station in parallel
  const measurementResults = useQueries({
    queries: selected.map((obj) => ({
      queryKey: ['compare-measurements', obj.id, range],
      queryFn: () =>
        range === '7d'
          ? getMeasurements1h(obj.id, { limit: 168 })
          : getMeasurements10min(obj.id, {
              limit: range === '24h' ? 144 : 36,
            }),
      enabled: selected.length >= 2,
      staleTime: 60_000,
    })),
  });

  const isLoading = measurementResults.some((r) => r.isLoading);

  const chartData = useMemo(() => {
    if (selected.length < 2 || isLoading) return [];
    const dataList = measurementResults.map((r) => r.data ?? []) as (
      | Measurement10min[]
      | Measurement1h[]
    )[];
    return mergeData(dataList, selected.map((o) => o.id), range);
  }, [measurementResults, selected, range, isLoading]);

  const addStation = (obj: ObjectView) => {
    if (selected.length >= MAX_STATIONS) return;
    if (selected.some((s) => s.id === obj.id)) return;
    setSelected((prev) => [...prev, obj]);
  };

  const removeStation = (id: string) =>
    setSelected((prev) => prev.filter((s) => s.id !== id));

  return (
    <div className="compare-page">
      <div className="compare-header">
        <div className="compare-title-row">
          <GitCompare size={20} />
          <h2>Usporedi stanice</h2>
        </div>
        <p className="compare-desc">
          Odaberi 2–4 stanice i promatraj napona, solarni napon, temperaturu i struju baterije
          u jednom pogledu.
        </p>
      </div>

      {/* Selector panel */}
      <div className="compare-selector card">
        <div className="compare-chips-row">
          {selected.map((obj, idx) => (
            <StationChip
              key={obj.id}
              obj={obj}
              colorIdx={idx}
              onRemove={() => removeStation(obj.id)}
            />
          ))}
          {selected.length < MAX_STATIONS && !loadingObjects && (
            <StationSearch all={allObjects} selected={selected} onAdd={addStation} />
          )}
        </div>

        {selected.length >= 2 && (
          <div className="compare-controls">
            <div className="range-selector">
              {RANGES.map((r) => (
                <button
                  key={r}
                  className={`filter-tab${range === r ? ' active' : ''}`}
                  onClick={() => setRange(r)}
                >
                  {r}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Empty state */}
      {selected.length < 2 && (
        <div className="compare-empty">
          <Info size={32} />
          <p>Odaberi barem <strong>2 stanice</strong> za usporedbu grafova.</p>
          <p className="compare-empty-sub">Možeš odabrati do {MAX_STATIONS} stanica.</p>
        </div>
      )}

      {/* Loading */}
      {selected.length >= 2 && isLoading && (
        <div className="page-spinner">
          <div className="spinner" />
        </div>
      )}

      {/* Charts */}
      {selected.length >= 2 && !isLoading && chartData.length === 0 && (
        <div className="no-data">Nema podataka za odabrani period.</div>
      )}

      {selected.length >= 2 && !isLoading && chartData.length > 0 && (
        <div className="compare-charts-grid">
          <CompareChart
            title="Napon baterije (V)"
            data={chartData}
            metric="bv"
            stations={selected}
          />
          <CompareChart
            title="Solarni napon (V)"
            data={chartData}
            metric="sv"
            stations={selected}
          />
          <CompareChart
            title="Temperatura datalogera (°C)"
            data={chartData}
            metric="tp"
            stations={selected}
          />
          <CompareChart
            title="Struja baterije (A)"
            data={chartData}
            metric="bc"
            stations={selected}
          />
        </div>
      )}
    </div>
  );
}
