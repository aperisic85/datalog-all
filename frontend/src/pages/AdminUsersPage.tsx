import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  listUsers,
  createUser,
  getUserRegions,
  grantRegionAccess,
  revokeRegionAccess,
  listRegions,
} from '../api/endpoints';
import { Plus, ChevronDown, ChevronRight, Trash2, X, Check } from 'lucide-react';
import { format, parseISO } from 'date-fns';
import type { UserPublic } from '../types';
import './AdminPage.css';

function CreateUserForm({ onDone, onCancel }: { onDone: () => void; onCancel: () => void }) {
  const qc = useQueryClient();
  const [form, setForm] = useState({ username: '', email: '', password: '', full_name: '', role: 'viewer' });
  const [err, setErr] = useState('');

  const create = useMutation({
    mutationFn: createUser,
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['users'] }); onDone(); },
    onError: () => setErr('Greška pri kreiranju korisnika'),
  });

  return (
    <form
      className="inline-form card"
      onSubmit={(e) => {
        e.preventDefault();
        create.mutate({ ...form, full_name: form.full_name || undefined });
      }}
    >
      <h4 style={{ marginBottom: 12 }}>Novi korisnik</h4>
      {err && <div className="error-msg" style={{ marginBottom: 10 }}>{err}</div>}
      <div className="form-row">
        <div className="form-group">
          <label>Korisničko ime *</label>
          <input value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} required />
        </div>
        <div className="form-group">
          <label>Email *</label>
          <input type="email" value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} required />
        </div>
      </div>
      <div className="form-row">
        <div className="form-group">
          <label>Puno ime</label>
          <input value={form.full_name} onChange={(e) => setForm({ ...form, full_name: e.target.value })} />
        </div>
        <div className="form-group">
          <label>Lozinka *</label>
          <input type="password" value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} required minLength={8} />
        </div>
      </div>
      <div className="form-group">
        <label>Uloga *</label>
        <select value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value })}>
          <option value="admin">Admin</option>
          <option value="operator">Operator</option>
          <option value="viewer">Viewer</option>
        </select>
      </div>
      <div className="form-actions">
        <button type="submit" className="btn-primary" disabled={create.isPending}>
          <Check size={14} /> Kreiraj
        </button>
        <button type="button" className="btn-secondary" onClick={onCancel}>
          <X size={14} /> Odustani
        </button>
      </div>
    </form>
  );
}

function UserRegions({ user }: { user: UserPublic }) {
  const qc = useQueryClient();
  const [regionId, setRegionId] = useState('');
  const [permission, setPermission] = useState('viewer');

  const { data: userRegions } = useQuery({
    queryKey: ['user-regions', user.id],
    queryFn: () => getUserRegions(user.id),
  });

  const { data: allRegions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const grant = useMutation({
    mutationFn: grantRegionAccess,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['user-regions', user.id] });
      setRegionId('');
    },
  });

  const revoke = useMutation({
    mutationFn: ({ uid, rid }: { uid: string; rid: string }) => revokeRegionAccess(uid, rid),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['user-regions', user.id] }),
  });

  const available = allRegions?.filter(
    (r) => !userRegions?.some((ur) => ur.region_id === r.id)
  );

  return (
    <div className="user-regions">
      <div className="user-regions-list">
        {userRegions?.length === 0 && (
          <span style={{ color: 'var(--text2)', fontSize: 13 }}>Nema pristupa regijama</span>
        )}
        {userRegions?.map((ur) => (
          <div key={ur.id} className="user-region-tag">
            <span className="region-dot" style={{ background: ur.region_color, display: 'inline-block', width: 8, height: 8, borderRadius: '50%' }} />
            <span>{ur.region_name}</span>
            <span className="badge badge-neutral" style={{ fontSize: 11 }}>{ur.permission}</span>
            <button
              className="icon-btn-sm"
              onClick={() => revoke.mutate({ uid: user.id, rid: ur.region_id })}
              title="Ukloni pristup"
            >
              <Trash2 size={12} />
            </button>
          </div>
        ))}
      </div>

      {(available?.length ?? 0) > 0 && (
        <div className="grant-form">
          <select value={regionId} onChange={(e) => setRegionId(e.target.value)} style={{ width: 'auto', minWidth: 140 }}>
            <option value="">Dodaj regiju...</option>
            {available?.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
          </select>
          <select value={permission} onChange={(e) => setPermission(e.target.value)} style={{ width: 'auto' }}>
            <option value="viewer">viewer</option>
            <option value="operator">operator</option>
          </select>
          <button
            className="btn-primary"
            style={{ padding: '6px 12px' }}
            disabled={!regionId || grant.isPending}
            onClick={() => grant.mutate({ user_id: user.id, region_id: regionId, permission })}
          >
            <Plus size={13} /> Dodaj
          </button>
        </div>
      )}
    </div>
  );
}

export default function AdminUsersPage() {
  const [showAdd, setShowAdd] = useState(false);
  const [expanded, setExpanded] = useState<string | null>(null);

  const { data: users, isLoading } = useQuery({ queryKey: ['users'], queryFn: listUsers });

  if (isLoading) return <div className="page-spinner"><div className="spinner" /></div>;

  return (
    <div className="admin-page">
      <div className="page-header">
        <h2>Korisnici</h2>
        <button className="btn-primary" onClick={() => setShowAdd(!showAdd)}>
          <Plus size={14} /> Novi korisnik
        </button>
      </div>

      {showAdd && <CreateUserForm onDone={() => setShowAdd(false)} onCancel={() => setShowAdd(false)} />}

      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th style={{ width: 32 }}></th>
              <th>Korisnik</th>
              <th>Email</th>
              <th>Uloga</th>
              <th>Status</th>
              <th>Zadnja prijava</th>
            </tr>
          </thead>
          <tbody>
            {users?.map((u) => (
              <>
                <tr
                  key={u.id}
                  className="expandable-row"
                  onClick={() => setExpanded(expanded === u.id ? null : u.id)}
                >
                  <td>
                    {expanded === u.id
                      ? <ChevronDown size={14} color="var(--text2)" />
                      : <ChevronRight size={14} color="var(--text2)" />
                    }
                  </td>
                  <td>
                    <div style={{ fontWeight: 500 }}>{u.username}</div>
                    {u.full_name && <div style={{ fontSize: 12, color: 'var(--text2)' }}>{u.full_name}</div>}
                  </td>
                  <td style={{ color: 'var(--text2)', fontSize: 13 }}>{u.email}</td>
                  <td>
                    <span className={`badge ${u.role === 'admin' ? 'badge-danger' : u.role === 'operator' ? 'badge-warning' : 'badge-neutral'}`}>
                      {u.role}
                    </span>
                  </td>
                  <td>
                    {u.is_active
                      ? <span className="badge badge-success">Aktivan</span>
                      : <span className="badge badge-neutral">Neaktivan</span>
                    }
                  </td>
                  <td style={{ fontSize: 12, color: 'var(--text2)' }}>
                    {u.last_login_at ? format(parseISO(u.last_login_at), 'dd.MM.yyyy HH:mm') : '—'}
                  </td>
                </tr>
                {expanded === u.id && (
                  <tr key={`regions-${u.id}`}>
                    <td></td>
                    <td colSpan={5} style={{ paddingTop: 0, paddingBottom: 12 }}>
                      <div style={{ fontSize: 13, color: 'var(--text2)', marginBottom: 6 }}>Pristup regijama:</div>
                      <UserRegions user={u} />
                    </td>
                  </tr>
                )}
              </>
            ))}
          </tbody>
        </table>
        </div>
      </div>
    </div>
  );
}
