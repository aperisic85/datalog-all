import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import './LoginPage.css';

function LighthouseIcon({ size = 24 }: { size?: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="12" y1="1" x2="12" y2="3" />
      <line x1="8.5" y1="1.8" x2="9.5" y2="3.6" />
      <line x1="15.5" y1="1.8" x2="14.5" y2="3.6" />
      <rect x="9" y="4" width="6" height="3" rx="0.5" />
      <path d="M10 7 L8.5 19 L15.5 19 L14 7" />
      <line x1="9.1" y1="12" x2="14.9" y2="12" />
      <rect x="7" y="19" width="10" height="2" rx="0.5" />
      <path d="M3 22.5 Q5 21.5 7 22.5 Q9 23.5 11 22.5 Q13 21.5 15 22.5 Q17 23.5 19 22.5 Q21 21.5 23 22.5" />
    </svg>
  );
}

export default function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      await login(username, password);
      navigate('/dashboard');
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(msg || 'Pogrešno korisničko ime ili lozinka');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-page">
      <div className="ocean-waves" aria-hidden="true">
        <div className="wave wave-1" />
        <div className="wave wave-2" />
        <div className="wave wave-3" />
      </div>
      <div className="login-box card">
        <div className="login-header">
          <div className="login-logo-wrap" style={{ color: 'var(--accent)' }}>
            <LighthouseIcon size={26} />
          </div>
          <h1>Beacon</h1>
          <p>Prijavite se na sistem</p>
        </div>
        <div className="login-divider" />

        <form onSubmit={handleSubmit}>
          {error && <div className="error-msg" style={{ marginBottom: 14 }}>{error}</div>}

          <div className="form-group">
            <label>Korisničko ime</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="username"
              autoFocus
              required
            />
          </div>

          <div className="form-group">
            <label>Lozinka</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
              required
            />
          </div>

          <button type="submit" className="btn-primary login-submit" disabled={loading}>
            {loading ? <><span className="spinner" style={{ width: 16, height: 16 }} /> Prijava...</> : 'Prijavi se'}
          </button>
        </form>
      </div>
    </div>
  );
}
