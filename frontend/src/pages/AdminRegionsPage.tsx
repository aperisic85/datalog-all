import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listRegions, createRegion, updateRegion } from '../api/endpoints';
import { Plus, Pencil, X, Check } from 'lucide-react';
import type { Region } from '../types';
import './AdminPage.css';

const COLORS = ['#3b82f6','#22c55e','#f59e0b','#ef4444','#8b5cf6','#06b6d4','#ec4899','#f97316'];

function RegionForm({
  initial,
  onSubmit,
  onCancel,
}: {
  initial?: Partial<Region>;
  onSubmit: (data: { name: string; code: string; description?: string; color?: string; is_active?: boolean }) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initial?.name || '');
  const [code, setCode] = useState(initial?.code || '');
  const [description, setDescription] = useState(initial?.description || '');
  const [color, setColor] = useState(initial?.color || COLORS[0]);
  const [isActive, setIsActive] = useState(initial?.is_active ?? true);

  return (
    <form
      className="inline-form card"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit({ name, code, description: description || undefined, color, is_active: isActive });
      }}
    >
      <div className="form-row">
        <div className="form-group">
          <label>Naziv *</label>
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </div>
        <div className="form-group">
          <label>Kod *</label>
          <input value={code} onChange={(e) => setCode(e.target.value)} required maxLength={10} />
        </div>
      </div>
      <div className="form-group">
        <label>Opis</label>
        <input value={description} onChange={(e) => setDescription(e.target.value)} />
      </div>
      <div className="form-group">
        <label>Boja</label>
        <div className="color-picker">
          {COLORS.map((c) => (
            <button
              key={c}
              type="button"
              className={`color-swatch ${color === c ? 'selected' : ''}`}
              style={{ background: c }}
              onClick={() => setColor(c)}
            />
          ))}
          <input type="color" value={color} onChange={(e) => setColor(e.target.value)} style={{ width: 36, padding: 2 }} />
        </div>
      </div>
      {initial && (
        <div className="form-group">
          <label className="checkbox-label">
            <input type="checkbox" checked={isActive} onChange={(e) => setIsActive(e.target.checked)} style={{ width: 'auto' }} />
            Aktivna
          </label>
        </div>
      )}
      <div className="form-actions">
        <button type="submit" className="btn-primary">
          <Check size={14} /> {initial ? 'Sačuvaj' : 'Dodaj'}
        </button>
        <button type="button" className="btn-secondary" onClick={onCancel}>
          <X size={14} /> Odustani
        </button>
      </div>
    </form>
  );
}

export default function AdminRegionsPage() {
  const qc = useQueryClient();
  const [showAdd, setShowAdd] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);

  const { data: regions, isLoading } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const create = useMutation({
    mutationFn: createRegion,
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['regions'] }); setShowAdd(false); },
  });

  const update = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Parameters<typeof updateRegion>[1] }) =>
      updateRegion(id, data),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['regions'] }); setEditing(null); },
  });

  if (isLoading) return <div className="page-spinner"><div className="spinner" /></div>;

  return (
    <div className="admin-page">
      <div className="page-header">
        <h2>Regije</h2>
        <button className="btn-primary" onClick={() => setShowAdd(true)}>
          <Plus size={14} /> Nova regija
        </button>
      </div>

      {showAdd && (
        <RegionForm
          onSubmit={(d) => create.mutate(d)}
          onCancel={() => setShowAdd(false)}
        />
      )}

      {create.isError && <div className="error-msg">Greška pri kreiranju regije</div>}
      {update.isError && <div className="error-msg">Greška pri ažuriranju regije</div>}

      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <table>
          <thead>
            <tr>
              <th>Naziv</th>
              <th>Kod</th>
              <th>Opis</th>
              <th>Status</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {regions?.map((r) => (
              <>
                <tr key={r.id}>
                  <td>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      <span className="region-dot" style={{ background: r.color, display: 'inline-block', width: 10, height: 10, borderRadius: '50%', flexShrink: 0 }} />
                      <strong>{r.name}</strong>
                    </div>
                  </td>
                  <td><code className="station-id">{r.code}</code></td>
                  <td style={{ color: 'var(--text2)', fontSize: 13 }}>{r.description || '—'}</td>
                  <td>
                    {r.is_active
                      ? <span className="badge badge-success">Aktivna</span>
                      : <span className="badge badge-neutral">Neaktivna</span>
                    }
                  </td>
                  <td>
                    <button
                      className="btn-secondary icon-btn"
                      onClick={() => setEditing(editing === r.id ? null : r.id)}
                    >
                      <Pencil size={14} />
                    </button>
                  </td>
                </tr>
                {editing === r.id && (
                  <tr key={`edit-${r.id}`}>
                    <td colSpan={5} style={{ padding: 0 }}>
                      <RegionForm
                        initial={r}
                        onSubmit={(d) => update.mutate({ id: r.id, data: d })}
                        onCancel={() => setEditing(null)}
                      />
                    </td>
                  </tr>
                )}
              </>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
