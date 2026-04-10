import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Link, useSearchParams } from 'react-router-dom';
import { listObjects, listRegions, listStationTypes, createObject } from '../api/endpoints';
import { useAuth } from '../context/AuthContext';
import { AlertTriangle, Search, ChevronLeft, ChevronRight, MapPin, Radio, Plus, X, LayoutGrid, List } from 'lucide-react';
import './ObjectsPage.css';

function AlarmBadge({ active, count }: { active: boolean; count: number }) {
  if (!active) return <span className="badge badge-success">OK</span>;
  return <span className="badge badge-danger"><AlertTriangle size={11} />{count} alarm{count !== 1 ? 'a' : ''}</span>;
}

function CreateObjectModal({ onClose }: { onClose: () => void }) {
  const qc = useQueryClient();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [form, setForm] = useState({
    station_id: '', name: '', short_name: '', region_id: '',
    station_type_id: '', datalogger_url: '', location_name: '',
    latitude: '', longitude: '', allowed_radius_m: '0', poll_interval_sec: '60',
    polling_enabled: false, description: '',
  });

  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });
  const { data: types }   = useQuery({ queryKey: ['station-types'], queryFn: listStationTypes });

  const set = (k: string, v: string | boolean) => setForm((f) => ({ ...f, [k]: v }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError('');
    try {
      await createObject({
        station_id:      form.station_id,
        name:            form.name,
        short_name:      form.short_name || undefined,
        region_id:       form.region_id,
        station_type_id: form.station_type_id ? Number(form.station_type_id) : undefined,
        datalogger_url:  form.datalogger_url || undefined,
        location_name:   form.location_name || undefined,
        latitude:        form.latitude ? Number(form.latitude) : undefined,
        longitude:       form.longitude ? Number(form.longitude) : undefined,
        allowed_radius_m: Number(form.allowed_radius_m) || 0,
        poll_interval_sec: Number(form.poll_interval_sec) || 60,
        polling_enabled: form.polling_enabled,
        description:     form.description || undefined,
      });
      qc.invalidateQueries({ queryKey: ['objects'] });
      onClose();
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(msg || 'Greška pri kreiranju objekta');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal-box card">
        <div className="modal-header">
          <h3>Novi objekt</h3>
          <button className="modal-close" onClick={onClose}><X size={18} /></button>
        </div>
        <form onSubmit={handleSubmit} className="modal-form">
          {error && <div className="error-msg">{error}</div>}

          <div className="form-row">
            <div className="form-group">
              <label>ID Stanice *</label>
              <input value={form.station_id} onChange={(e) => set('station_id', e.target.value)} required placeholder="npr. Galija_01" />
            </div>
            <div className="form-group">
              <label>Naziv *</label>
              <input value={form.name} onChange={(e) => set('name', e.target.value)} required placeholder="Naziv objekta" />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Kratki naziv</label>
              <input value={form.short_name} onChange={(e) => set('short_name', e.target.value)} placeholder="Skraćenica" />
            </div>
            <div className="form-group">
              <label>Regija *</label>
              <select value={form.region_id} onChange={(e) => set('region_id', e.target.value)} required>
                <option value="">Odaberi regiju...</option>
                {regions?.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
              </select>
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Tip stanice</label>
              <select value={form.station_type_id} onChange={(e) => set('station_type_id', e.target.value)}>
                <option value="">Bez tipa</option>
                {types?.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
              </select>
            </div>
            <div className="form-group">
              <label>Lokacija</label>
              <input value={form.location_name} onChange={(e) => set('location_name', e.target.value)} placeholder="Naziv lokacije" />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Latitude</label>
              <input type="number" step="any" value={form.latitude} onChange={(e) => set('latitude', e.target.value)} placeholder="43.123456" />
            </div>
            <div className="form-group">
              <label>Longitude</label>
              <input type="number" step="any" value={form.longitude} onChange={(e) => set('longitude', e.target.value)} placeholder="16.123456" />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Dozvoljeni radijus (m)</label>
              <input type="number" min="0" value={form.allowed_radius_m} onChange={(e) => set('allowed_radius_m', e.target.value)} placeholder="0 = fiksni objekt" />
            </div>
            <div className="form-group" />
          </div>

          <div className="form-group">
            <label>Datalogger URL</label>
            <input value={form.datalogger_url} onChange={(e) => set('datalogger_url', e.target.value)} placeholder="http://192.168.1.100" />
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>Poll interval (s)</label>
              <input type="number" value={form.poll_interval_sec} onChange={(e) => set('poll_interval_sec', e.target.value)} min={10} />
            </div>
            <div className="form-group" style={{ justifyContent: 'flex-end' }}>
              <label className="filter-checkbox" style={{ marginTop: 24 }}>
                <input type="checkbox" checked={form.polling_enabled} onChange={(e) => set('polling_enabled', e.target.checked)} style={{ width: 'auto' }} />
                Polling uključen
              </label>
            </div>
          </div>

          <div className="form-group">
            <label>Opis</label>
            <input value={form.description} onChange={(e) => set('description', e.target.value)} placeholder="Opis objekta" />
          </div>

          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>Odustani</button>
            <button type="submit" className="btn-primary" disabled={saving}>
              {saving ? <><span className="spinner" style={{ width: 14, height: 14 }} /> Sprema...</> : 'Kreiraj objekt'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export default function ObjectsPage() {
  const { isAdmin } = useAuth();
  const [searchParams] = useSearchParams();
  const [search, setSearch] = useState('');
  const [regionFilter, setRegionFilter] = useState(() => searchParams.get('region_id') || '');
  const [activeFilter, setActiveFilter] = useState<'all' | 'active' | 'inactive'>(() => {
    const a = searchParams.get('active');
    return a === 'true' ? 'active' : a === 'false' ? 'inactive' : 'all';
  });
  const [alarmFilter, setAlarmFilter] = useState(false);
  const [page, setPage] = useState(1);
  const [showCreate, setShowCreate] = useState(false);
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const PAGE_SIZE = viewMode === 'grid' ? 100 : 20;

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
      {showCreate && <CreateObjectModal onClose={() => setShowCreate(false)} />}

      <div className="page-header">
        <div>
          <h2>Objekti</h2>
          <span className="text-muted">{data?.total != null ? `${data.total} stanica` : 'Prikaz stanica i datalogera'}</span>
        </div>
        {isAdmin && (
          <button className="btn-primary" onClick={() => setShowCreate(true)}>
            <Plus size={15} /> Novi objekt
          </button>
        )}
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

        <div className="view-toggle">
          <button
            className={`view-toggle-btn ${viewMode === 'list' ? 'active' : ''}`}
            onClick={() => setViewMode('list')}
            title="Prikaz liste"
          ><List size={15} /></button>
          <button
            className={`view-toggle-btn ${viewMode === 'grid' ? 'active' : ''}`}
            onClick={() => setViewMode('grid')}
            title="Prikaz mreže"
          ><LayoutGrid size={15} /></button>
        </div>
      </div>

      {isLoading ? (
        <div className="page-spinner"><div className="spinner" /></div>
      ) : (
        <>
          {/* Status grid / heatmap view */}
          {viewMode === 'grid' && (
            <div className="obj-status-grid card">
              {data?.data.length === 0 && (
                <div className="obj-card-empty">Nema rezultata</div>
              )}
              {data?.data.map((obj) => (
                <Link
                  to={`/objects/${obj.id}`}
                  key={obj.id}
                  className={`obj-status-cell ${obj.alarm_active ? 'cell-alarm' : obj.is_active ? 'cell-active' : 'cell-inactive'}`}
                  title={`${obj.name} · ${obj.station_id}${obj.alarm_active ? ' · ⚠ Alarm aktivan' : ''}`}
                >
                  <span className="cell-name">{obj.short_name || obj.name}</span>
                  {obj.alarm_active && <AlertTriangle size={10} />}
                </Link>
              ))}
            </div>
          )}

          {/* Desktop table view */}
          {viewMode === 'list' && (
            <div className="objects-table card objects-table">
              <div className="table-scroll">
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
                          {obj.program_features != null
                            ? <span className="badge" style={{ fontSize: 10, background: 'var(--accent)', color: '#fff', marginTop: 2 }}>Tip 2</span>
                            : <span className="badge badge-neutral" style={{ fontSize: 10, marginTop: 2 }}>Tip 1</span>
                          }
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
            </div>
          )}

          {/* Mobile card view — only in list mode */}
          {viewMode === 'list' && (
            <div className="obj-card-list">
              {data?.data.length === 0 && (
                <div className="obj-card-empty">Nema rezultata</div>
              )}
              {data?.data.map((obj) => (
                <Link to={`/objects/${obj.id}`} key={obj.id} className="obj-card card">
                  <div className="obj-card-top">
                    <div className="obj-card-name">
                      <span className={`status-dot ${obj.alarm_active ? 'status-dot-alarm' : obj.is_active ? 'status-dot-active' : 'status-dot-inactive'}`} />
                      <span>{obj.name}</span>
                    </div>
                    <AlarmBadge active={obj.alarm_active} count={obj.alarm_count} />
                  </div>
                  <div className="obj-card-meta">
                    <span className="region-tag">
                      <span className="region-dot" style={{ background: obj.region_color }} />
                      {obj.region_name}
                    </span>
                    {obj.location_name && (
                      <span className="location-cell">
                        <MapPin size={12} />
                        {obj.location_name}
                      </span>
                    )}
                  </div>
                  <div className="obj-card-footer">
                    <code className="station-id">{obj.station_id}</code>
                    {obj.program_features != null
                      ? <span className="badge" style={{ fontSize: 10, background: 'var(--accent)', color: '#fff' }}>Tip 2</span>
                      : <span className="badge badge-neutral" style={{ fontSize: 10 }}>Tip 1</span>
                    }
                    {obj.is_active
                      ? <span className="badge badge-success" style={{ fontSize: 11 }}>Aktivan</span>
                      : <span className="badge badge-neutral" style={{ fontSize: 11 }}>Neaktivan</span>
                    }
                  </div>
                </Link>
              ))}
            </div>
          )}

          {viewMode === 'list' && data && data.total_pages > 1 && (
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
