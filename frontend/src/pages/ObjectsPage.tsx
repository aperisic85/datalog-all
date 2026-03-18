import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { listObjects, listRegions } from '../api/endpoints';
import { AlertTriangle, Search, ChevronLeft, ChevronRight, MapPin, Radio } from 'lucide-react';
import './ObjectsPage.css';

function AlarmBadge({ active, count }: { active: boolean; count: number }) {
  if (!active) return <span className="badge badge-success">OK</span>;
  return <span className="badge badge-danger"><AlertTriangle size={11} />{count} alarm{count !== 1 ? 'a' : ''}</span>;
}

export default function ObjectsPage() {
  const [search, setSearch] = useState('');
  const [regionFilter, setRegionFilter] = useState('');
  const [activeFilter, setActiveFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [alarmFilter, setAlarmFilter] = useState(false);
  const [page, setPage] = useState(1);
  const PAGE_SIZE = 20;

  const { data, isLoading } = useQuery({
    queryKey: ['objects', search, regionFilter, activeFilter, alarmFilter, page],
    queryFn: () => listObjects({
      page,
      page_size: PAGE_SIZE,
      search: search || undefined,
      region_id: regionFilter || undefined,
      active: activeFilter === 'all' ? undefined : activeFilter === 'active',
      in_alarm: alarmFilter || undefined,
    }),
    placeholderData: (prev) => prev,
  });

  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const handleSearch = (val: string) => { setSearch(val); setPage(1); };
  const handleRegion = (val: string) => { setRegionFilter(val); setPage(1); };
  const handleActive = (val: 'all' | 'active' | 'inactive') => { setActiveFilter(val); setPage(1); };

  return (
    <div className="objects-page">
      <div className="page-header">
        <h2>Objekti</h2>
        <span className="text-muted">Prikaz stanica i datalogera</span>
      </div>

      <div className="objects-filters card">
        <div className="filter-search">
          <Search size={15} className="filter-search-icon" />
          <input
            type="text"
            placeholder="Pretraži po nazivu, ID stanice..."
            value={search}
            onChange={(e) => handleSearch(e.target.value)}
          />
        </div>

        <select value={regionFilter} onChange={(e) => handleRegion(e.target.value)}>
          <option value="">Sve regije</option>
          {regions?.map((r) => (
            <option key={r.id} value={r.id}>{r.name}</option>
          ))}
        </select>

        <div className="filter-tabs">
          {(['all', 'active', 'inactive'] as const).map((v) => (
            <button
              key={v}
              className={`filter-tab ${activeFilter === v ? 'active' : ''}`}
              onClick={() => handleActive(v)}
            >
              {v === 'all' ? 'Svi' : v === 'active' ? 'Aktivni' : 'Neaktivni'}
            </button>
          ))}
        </div>

        <label className="filter-checkbox">
          <input
            type="checkbox"
            checked={alarmFilter}
            onChange={(e) => { setAlarmFilter(e.target.checked); setPage(1); }}
            style={{ width: 'auto' }}
          />
          Samo alarmi
        </label>
      </div>

      {isLoading ? (
        <div className="page-spinner"><div className="spinner" /></div>
      ) : (
        <>
          <div className="objects-table card">
            <table>
              <thead>
                <tr>
                  <th>Naziv</th>
                  <th>ID Stanice</th>
                  <th>Regija</th>
                  <th>Lokacija</th>
                  <th>Status</th>
                  <th>Alarm</th>
                </tr>
              </thead>
              <tbody>
                {data?.data.length === 0 && (
                  <tr><td colSpan={6} style={{ textAlign: 'center', color: 'var(--text2)', padding: 32 }}>Nema rezultata</td></tr>
                )}
                {data?.data.map((obj) => (
                  <tr key={obj.id}>
                    <td>
                      <Link to={`/objects/${obj.id}`} className="obj-name">
                        <Radio size={14} />
                        {obj.name}
                      </Link>
                      {obj.short_name && <div className="obj-sub">{obj.short_name}</div>}
                    </td>
                    <td><code className="station-id">{obj.station_id}</code></td>
                    <td>
                      <div className="region-tag">
                        <span className="region-dot" style={{ background: obj.region_color }} />
                        {obj.region_name}
                      </div>
                    </td>
                    <td>
                      {obj.location_name ? (
                        <div className="location-cell">
                          <MapPin size={12} />
                          <span>{obj.location_name}</span>
                        </div>
                      ) : (
                        <span className="text-muted">—</span>
                      )}
                    </td>
                    <td>
                      {obj.is_active
                        ? <span className="badge badge-success">Aktivan</span>
                        : <span className="badge badge-neutral">Neaktivan</span>
                      }
                    </td>
                    <td>
                      <AlarmBadge active={obj.alarm_active} count={obj.alarm_count} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {data && data.total_pages > 1 && (
            <div className="pagination">
              <button
                className="btn-secondary"
                disabled={page <= 1}
                onClick={() => setPage((p) => p - 1)}
              >
                <ChevronLeft size={16} />
              </button>
              <span className="page-info">
                Strana {data.page} od {data.total_pages} ({data.total} ukupno)
              </span>
              <button
                className="btn-secondary"
                disabled={page >= data.total_pages}
                onClick={() => setPage((p) => p + 1)}
              >
                <ChevronRight size={16} />
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
