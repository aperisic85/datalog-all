import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { Activity } from 'lucide-react';
import './LoginPage.css';

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
          <div className="login-logo-wrap">
            <Activity size={26} color="var(--accent)" />
          </div>
          <h1>DataLogger</h1>
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
