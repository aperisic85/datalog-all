import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  listNotificationChannels, createNotificationChannel, updateNotificationChannel,
  deleteNotificationChannel, testNotificationChannel,
  listNotificationRules, createNotificationRule, updateNotificationRule, deleteNotificationRule,
  listNotificationLog, listRegions,
} from '../api/endpoints';
import { Plus, Pencil, Trash2, X, Check, Send, Bell } from 'lucide-react';
import type { NotificationChannel, NotificationRule } from '../types';
import './AdminPage.css';

// ── Pomoćne oznake ───────────────────────────────────────────────────────────

const SEVERITY: Record<number, string> = { 1: 'Info', 2: 'Upozorenje', 3: 'Greška', 4: 'Kritično' };

const ALARM_LABELS: Record<string, string> = {
  datalogger_high_temp: 'Visoka temperatura dataloggera',
  datalogger_high_voltage: 'Visoki napon dataloggera',
  datalogger_other_error: 'Greška dataloggera',
  battery_voltage_low: 'Nizak napon baterije',
  battery_voltage_flat: 'Baterija prazna',
  battery_other_error: 'Greška baterije',
  garmin_comm_failed: 'GPS komunikacija prekinuta',
  garmin_other_error: 'GPS greška',
  station_out_of_radius: 'Stanica izvan radijusa',
  lantern_night_light_off: 'Fenjer ugašen noću',
  lantern_day_light_on: 'Fenjer upaljen danju',
  lantern_comm_failed: 'Komunikacija s fenjerom prekinuta',
  lantern_other_error: 'Greška fenjera',
  modem_network_error: 'Modem bez mreže',
  modem_other_error: 'Greška modema',
  station_other_error: 'Greška stanice',
  visibility_comm_failed: 'Senzor vidljivosti — komunikacija',
  visibility_error: 'Greška senzora vidljivosti',
  fog_signal_off_during_fog: 'Maglena sirena ugašena za magle',
  fog_signal_on_while_no_fog: 'Maglena sirena radi bez magle',
};

const KIND_LABELS: Record<string, string> = { telegram: 'Telegram', slack: 'Slack', webhook: 'Webhook' };

const fmt = (s: string) => new Date(s).toLocaleString('hr-HR');

// ── Forma za kanal ───────────────────────────────────────────────────────────

interface ChannelFormData {
  name: string;
  kind: string;
  config: Record<string, unknown>;
  enabled: boolean;
}

function ChannelForm({
  initial, onSubmit, onCancel,
}: {
  initial?: NotificationChannel;
  onSubmit: (data: ChannelFormData) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initial?.name || '');
  const [kind, setKind] = useState<string>(initial?.kind || 'telegram');
  const [botToken, setBotToken] = useState((initial?.config?.bot_token as string) || '');
  const [chatId, setChatId] = useState(
    initial?.config?.chat_id != null ? String(initial.config.chat_id) : ''
  );
  const [url, setUrl] = useState((initial?.config?.url as string) || '');
  const [enabled, setEnabled] = useState(initial?.enabled ?? true);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const config = kind === 'telegram'
      ? { bot_token: botToken, chat_id: chatId }
      : { url };
    onSubmit({ name, kind, config, enabled });
  };

  return (
    <form className="inline-form card" onSubmit={submit}>
      <div className="form-row">
        <div className="form-group">
          <label>Naziv *</label>
          <input value={name} onChange={(e) => setName(e.target.value)} required placeholder="npr. Dežurni Telegram" />
        </div>
        <div className="form-group">
          <label>Vrsta *</label>
          {/* Vrsta se ne mijenja kod uređivanja jer mijenja oblik konfiguracije */}
          <select value={kind} onChange={(e) => setKind(e.target.value)} disabled={!!initial}>
            <option value="telegram">Telegram</option>
            <option value="slack">Slack</option>
            <option value="webhook">Webhook (generički)</option>
          </select>
        </div>
      </div>

      {kind === 'telegram' ? (
        <div className="form-row">
          <div className="form-group">
            <label>Bot token *</label>
            <input value={botToken} onChange={(e) => setBotToken(e.target.value)} required placeholder="123456:ABC-..." />
          </div>
          <div className="form-group">
            <label>Chat ID *</label>
            <input value={chatId} onChange={(e) => setChatId(e.target.value)} required placeholder="-1001234567890" />
          </div>
        </div>
      ) : (
        <div className="form-group">
          <label>URL *</label>
          <input value={url} onChange={(e) => setUrl(e.target.value)} required placeholder="https://..." />
        </div>
      )}

      <div className="form-group">
        <label className="checkbox-label">
          <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} style={{ width: 'auto' }} />
          Omogućen
        </label>
      </div>

      <div className="form-actions">
        <button type="submit" className="btn-primary"><Check size={14} /> {initial ? 'Sačuvaj' : 'Dodaj'}</button>
        <button type="button" className="btn-secondary" onClick={onCancel}><X size={14} /> Odustani</button>
      </div>
    </form>
  );
}

// ── Forma za pravilo ─────────────────────────────────────────────────────────

interface RuleFormData {
  name: string;
  channel_id: string;
  region_id: string | null;
  min_severity: number;
  notify_on_clear: boolean;
  quiet_hours_start: number | null;
  quiet_hours_end: number | null;
  cooldown_minutes: number;
  enabled: boolean;
}

function RuleForm({
  initial, channels, regions, onSubmit, onCancel,
}: {
  initial?: NotificationRule;
  channels: NotificationChannel[];
  regions: { id: string; name: string }[];
  onSubmit: (data: RuleFormData) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initial?.name || '');
  const [channelId, setChannelId] = useState(initial?.channel_id || channels[0]?.id || '');
  const [regionId, setRegionId] = useState(initial?.region_id || '');
  const [minSeverity, setMinSeverity] = useState(initial?.min_severity ?? 3);
  const [notifyOnClear, setNotifyOnClear] = useState(initial?.notify_on_clear ?? true);
  const [quietEnabled, setQuietEnabled] = useState(
    initial?.quiet_hours_start != null && initial?.quiet_hours_end != null
  );
  const [quietStart, setQuietStart] = useState(initial?.quiet_hours_start ?? 22);
  const [quietEnd, setQuietEnd] = useState(initial?.quiet_hours_end ?? 6);
  const [cooldown, setCooldown] = useState(initial?.cooldown_minutes ?? 360);
  const [enabled, setEnabled] = useState(initial?.enabled ?? true);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      name,
      channel_id: channelId,
      region_id: regionId || null,
      min_severity: minSeverity,
      notify_on_clear: notifyOnClear,
      quiet_hours_start: quietEnabled ? quietStart : null,
      quiet_hours_end: quietEnabled ? quietEnd : null,
      cooldown_minutes: cooldown,
      enabled,
    });
  };

  return (
    <form className="inline-form card" onSubmit={submit}>
      <div className="form-row">
        <div className="form-group">
          <label>Naziv *</label>
          <input value={name} onChange={(e) => setName(e.target.value)} required placeholder="npr. Kritični alarmi Split" />
        </div>
        <div className="form-group">
          <label>Kanal *</label>
          <select value={channelId} onChange={(e) => setChannelId(e.target.value)} required>
            {channels.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
          </select>
        </div>
      </div>

      <div className="form-row">
        <div className="form-group">
          <label>Regija</label>
          <select value={regionId} onChange={(e) => setRegionId(e.target.value)}>
            <option value="">Sve regije</option>
            {regions.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
          </select>
        </div>
        <div className="form-group">
          <label>Min. ozbiljnost</label>
          <select value={minSeverity} onChange={(e) => setMinSeverity(Number(e.target.value))}>
            <option value={1}>Info i više</option>
            <option value={2}>Upozorenje i više</option>
            <option value={3}>Greška i više</option>
            <option value={4}>Samo kritično</option>
          </select>
        </div>
      </div>

      <div className="form-row">
        <div className="form-group">
          <label>Ponovno javljanje (min)</label>
          <input type="number" min={0} value={cooldown} onChange={(e) => setCooldown(Number(e.target.value))} />
        </div>
        <div className="form-group">
          <label className="checkbox-label" style={{ marginTop: 26 }}>
            <input type="checkbox" checked={notifyOnClear} onChange={(e) => setNotifyOnClear(e.target.checked)} style={{ width: 'auto' }} />
            Javi i kad se alarm riješi
          </label>
        </div>
      </div>

      <div className="form-group">
        <label className="checkbox-label">
          <input type="checkbox" checked={quietEnabled} onChange={(e) => setQuietEnabled(e.target.checked)} style={{ width: 'auto' }} />
          Tihi sati (suspendiraju ne-kritične obavijesti, UTC)
        </label>
      </div>
      {quietEnabled && (
        <div className="form-row">
          <div className="form-group">
            <label>Od (sat, UTC)</label>
            <input type="number" min={0} max={23} value={quietStart} onChange={(e) => setQuietStart(Number(e.target.value))} />
          </div>
          <div className="form-group">
            <label>Do (sat, UTC)</label>
            <input type="number" min={0} max={23} value={quietEnd} onChange={(e) => setQuietEnd(Number(e.target.value))} />
          </div>
        </div>
      )}

      <div className="form-group">
        <label className="checkbox-label">
          <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} style={{ width: 'auto' }} />
          Omogućeno
        </label>
      </div>

      <div className="form-actions">
        <button type="submit" className="btn-primary"><Check size={14} /> {initial ? 'Sačuvaj' : 'Dodaj'}</button>
        <button type="button" className="btn-secondary" onClick={onCancel}><X size={14} /> Odustani</button>
      </div>
    </form>
  );
}

// ── Glavna stranica ──────────────────────────────────────────────────────────

export default function AdminNotificationsPage() {
  const qc = useQueryClient();
  const [showAddCh, setShowAddCh] = useState(false);
  const [editingCh, setEditingCh] = useState<string | null>(null);
  const [showAddRule, setShowAddRule] = useState(false);
  const [editingRule, setEditingRule] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<Record<string, string>>({});

  const channelsQ = useQuery({ queryKey: ['notif-channels'], queryFn: listNotificationChannels });
  const rulesQ    = useQuery({ queryKey: ['notif-rules'], queryFn: listNotificationRules });
  const regionsQ  = useQuery({ queryKey: ['regions'], queryFn: listRegions });
  const logQ      = useQuery({ queryKey: ['notif-log'], queryFn: () => listNotificationLog({ page_size: 50 }) });

  const channels = channelsQ.data ?? [];
  const regions  = regionsQ.data ?? [];
  const channelName = (id?: string) => channels.find((c) => c.id === id)?.name ?? '—';
  const regionName  = (id?: string | null) => id ? (regions.find((r) => r.id === id)?.name ?? '—') : 'Sve regije';

  const invCh   = () => qc.invalidateQueries({ queryKey: ['notif-channels'] });
  const invRule = () => qc.invalidateQueries({ queryKey: ['notif-rules'] });

  const createCh = useMutation({ mutationFn: createNotificationChannel, onSuccess: () => { invCh(); setShowAddCh(false); } });
  const updateCh = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Parameters<typeof updateNotificationChannel>[1] }) => updateNotificationChannel(id, data),
    onSuccess: () => { invCh(); setEditingCh(null); },
  });
  const deleteCh = useMutation({ mutationFn: deleteNotificationChannel, onSuccess: invCh });
  const testCh = useMutation({
    mutationFn: testNotificationChannel,
    onSuccess: (res, id) => {
      setTestResult((p) => ({ ...p, [id]: res.status === 'sent' ? 'Poslano ✓' : `Greška: ${res.error ?? ''}` }));
      qc.invalidateQueries({ queryKey: ['notif-log'] });
    },
  });

  const createRule = useMutation({ mutationFn: createNotificationRule, onSuccess: () => { invRule(); setShowAddRule(false); } });
  const updateRule = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Parameters<typeof updateNotificationRule>[1] }) => updateNotificationRule(id, data),
    onSuccess: () => { invRule(); setEditingRule(null); },
  });
  const deleteRule = useMutation({ mutationFn: deleteNotificationRule, onSuccess: invRule });

  // Za uređivanje pravila šaljemo i clear_region kad korisnik odabere "Sve regije"
  const ruleUpdatePayload = (d: RuleFormData) => ({ ...d, clear_region: d.region_id === null });

  return (
    <div className="admin-page">
      <div className="page-header">
        <h2><Bell size={20} style={{ verticalAlign: -4, marginRight: 8 }} />Obavještavanje</h2>
      </div>

      <p style={{ color: 'var(--text2)', marginTop: -8, marginBottom: 20, maxWidth: 760 }}>
        Definirajte <strong>kanale</strong> (kamo se šalju obavijesti) i <strong>pravila</strong> (koji alarmi, po regiji i
        ozbiljnosti, idu na koji kanal). Obavijesti se šalju kad alarm nastane, ponavljaju nakon razdoblja mirovanja,
        te po želji javljaju i kad se alarm riješi.
      </p>

      {/* ── Kanali ── */}
      <div className="section-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0 }}>Kanali</h3>
        <button className="btn-primary" onClick={() => setShowAddCh(true)}><Plus size={14} /> Novi kanal</button>
      </div>

      {showAddCh && (
        <ChannelForm onSubmit={(d) => createCh.mutate(d)} onCancel={() => setShowAddCh(false)} />
      )}

      <div className="card" style={{ padding: 0, overflow: 'hidden', marginBottom: 28 }}>
        <div className="table-scroll">
          <table>
            <thead>
              <tr><th>Naziv</th><th>Vrsta</th><th>Status</th><th></th></tr>
            </thead>
            <tbody>
              {channels.length === 0 && (
                <tr><td colSpan={4} style={{ color: 'var(--text2)', textAlign: 'center', padding: 20 }}>Još nema kanala.</td></tr>
              )}
              {channels.map((c) => (
                <>
                  <tr key={c.id}>
                    <td><strong>{c.name}</strong></td>
                    <td><span className="badge badge-neutral">{KIND_LABELS[c.kind] ?? c.kind}</span></td>
                    <td>
                      {c.enabled
                        ? <span className="badge badge-success">Omogućen</span>
                        : <span className="badge badge-neutral">Onemogućen</span>}
                      {testResult[c.id] && (
                        <span style={{ marginLeft: 8, fontSize: 12, color: 'var(--text2)' }}>{testResult[c.id]}</span>
                      )}
                    </td>
                    <td>
                      <div style={{ display: 'flex', gap: 6 }}>
                        <button className="btn-secondary icon-btn" title="Pošalji probnu poruku"
                          disabled={testCh.isPending} onClick={() => testCh.mutate(c.id)}>
                          <Send size={14} />
                        </button>
                        <button className="btn-secondary icon-btn" title="Uredi"
                          onClick={() => setEditingCh(editingCh === c.id ? null : c.id)}>
                          <Pencil size={14} />
                        </button>
                        <button className="btn-secondary icon-btn" title="Obriši"
                          onClick={() => { if (confirm(`Obrisati kanal "${c.name}"?`)) deleteCh.mutate(c.id); }}>
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                  {editingCh === c.id && (
                    <tr key={`edit-${c.id}`}>
                      <td colSpan={4} style={{ padding: 0 }}>
                        <ChannelForm initial={c}
                          onSubmit={(d) => updateCh.mutate({ id: c.id, data: { name: d.name, config: d.config, enabled: d.enabled } })}
                          onCancel={() => setEditingCh(null)} />
                      </td>
                    </tr>
                  )}
                </>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* ── Pravila ── */}
      <div className="section-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h3 style={{ margin: 0 }}>Pravila</h3>
        <button className="btn-primary" disabled={channels.length === 0} title={channels.length === 0 ? 'Prvo dodajte kanal' : ''}
          onClick={() => setShowAddRule(true)}><Plus size={14} /> Novo pravilo</button>
      </div>

      {showAddRule && channels.length > 0 && (
        <RuleForm channels={channels} regions={regions}
          onSubmit={(d) => createRule.mutate(d)} onCancel={() => setShowAddRule(false)} />
      )}

      <div className="card" style={{ padding: 0, overflow: 'hidden', marginBottom: 28 }}>
        <div className="table-scroll">
          <table>
            <thead>
              <tr><th>Naziv</th><th>Kanal</th><th>Regija</th><th>Ozbiljnost</th><th>Mirovanje</th><th>Riješeno</th><th>Status</th><th></th></tr>
            </thead>
            <tbody>
              {(rulesQ.data ?? []).length === 0 && (
                <tr><td colSpan={8} style={{ color: 'var(--text2)', textAlign: 'center', padding: 20 }}>Još nema pravila.</td></tr>
              )}
              {(rulesQ.data ?? []).map((r) => (
                <>
                  <tr key={r.id}>
                    <td><strong>{r.name}</strong></td>
                    <td>{channelName(r.channel_id)}</td>
                    <td>{regionName(r.region_id)}</td>
                    <td><span className="badge badge-neutral">{SEVERITY[r.min_severity]}+</span></td>
                    <td style={{ color: 'var(--text2)', fontSize: 13 }}>{r.cooldown_minutes} min</td>
                    <td>{r.notify_on_clear ? <Check size={14} /> : '—'}</td>
                    <td>{r.enabled ? <span className="badge badge-success">Aktivno</span> : <span className="badge badge-neutral">Pauzirano</span>}</td>
                    <td>
                      <div style={{ display: 'flex', gap: 6 }}>
                        <button className="btn-secondary icon-btn" title="Uredi"
                          onClick={() => setEditingRule(editingRule === r.id ? null : r.id)}>
                          <Pencil size={14} />
                        </button>
                        <button className="btn-secondary icon-btn" title="Obriši"
                          onClick={() => { if (confirm(`Obrisati pravilo "${r.name}"?`)) deleteRule.mutate(r.id); }}>
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                  {editingRule === r.id && (
                    <tr key={`edit-${r.id}`}>
                      <td colSpan={8} style={{ padding: 0 }}>
                        <RuleForm initial={r} channels={channels} regions={regions}
                          onSubmit={(d) => updateRule.mutate({ id: r.id, data: ruleUpdatePayload(d) })}
                          onCancel={() => setEditingRule(null)} />
                      </td>
                    </tr>
                  )}
                </>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* ── Povijest ── */}
      <div className="section-header" style={{ marginBottom: 12 }}>
        <h3 style={{ margin: 0 }}>Povijest obavijesti</h3>
      </div>
      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <div className="table-scroll">
          <table>
            <thead>
              <tr><th>Vrijeme</th><th>Objekt</th><th>Alarm</th><th>Događaj</th><th>Kanal</th><th>Status</th></tr>
            </thead>
            <tbody>
              {(logQ.data?.data ?? []).length === 0 && (
                <tr><td colSpan={6} style={{ color: 'var(--text2)', textAlign: 'center', padding: 20 }}>Još nema poslanih obavijesti.</td></tr>
              )}
              {(logQ.data?.data ?? []).map((e) => (
                <tr key={e.id}>
                  <td style={{ whiteSpace: 'nowrap', fontSize: 13 }}>{fmt(e.created_at)}</td>
                  <td>{e.object_name ?? '—'}</td>
                  <td style={{ fontSize: 13 }}>{e.alarm_type ? (ALARM_LABELS[e.alarm_type] ?? e.alarm_type) : '—'}</td>
                  <td>
                    {e.event === 'raised' && <span className="badge badge-danger">Alarm</span>}
                    {e.event === 'cleared' && <span className="badge badge-success">Riješeno</span>}
                    {e.event === 'test' && <span className="badge badge-neutral">Test</span>}
                  </td>
                  <td>{e.channel_name ?? '—'}</td>
                  <td>
                    {e.status === 'sent'
                      ? <span className="badge badge-success">Poslano</span>
                      : <span className="badge badge-danger" title={e.error}>Neuspjeh</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
