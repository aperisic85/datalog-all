import { NavLink, Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import {
  LayoutDashboard,
  Radio,
  Users,
  MapPin,
  LogOut,
  Sun,
  Moon,
  Map,
  Menu,
  X,
  AlertTriangle,
  GitCompare,
} from 'lucide-react';

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
import './Layout.css';

export default function Layout() {
  const { user, logout, isAdmin } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    return (localStorage.getItem('theme') as 'dark' | 'light') || 'light';
  });
  const [sidebarOpen, setSidebarOpen] = useState(false);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
  }, [theme]);

  // Lock body scroll when sidebar is open on mobile
  useEffect(() => {
    if (sidebarOpen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => { document.body.style.overflow = ''; };
  }, [sidebarOpen]);

  const toggleTheme = () => setTheme((t) => t === 'dark' ? 'light' : 'dark');

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  const closeSidebar = () => setSidebarOpen(false);

  return (
    <div className="layout">
      {sidebarOpen && <div className="sidebar-overlay" onClick={closeSidebar} />}

      <aside className={`sidebar${sidebarOpen ? ' sidebar-open' : ''}`}>
        <div className="sidebar-logo">
          <LighthouseIcon size={20} />
          <span>Beacon</span>
          <button className="sidebar-close-btn" onClick={closeSidebar} title="Zatvori">
            <X size={18} />
          </button>
        </div>

        <nav className="sidebar-nav">
          <NavLink to="/dashboard" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
            <LayoutDashboard size={16} />
            Dashboard
          </NavLink>
          <NavLink to="/objects" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
            <Radio size={16} />
            Objekti
          </NavLink>
          <NavLink to="/map" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
            <Map size={16} />
            Karta
          </NavLink>
          <NavLink to="/alarms" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
            <AlertTriangle size={16} />
            Alarmi
          </NavLink>
          <NavLink to="/compare" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
            <GitCompare size={16} />
            Usporedi
          </NavLink>
          {isAdmin && (
            <>
              <div className="nav-section">Admin</div>
              <NavLink to="/admin/regions" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
                <MapPin size={16} />
                Regije
              </NavLink>
              <NavLink to="/admin/users" onClick={closeSidebar} className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}>
                <Users size={16} />
                Korisnici
              </NavLink>
            </>
          )}
        </nav>

        <div className="sidebar-footer">
          <div className="sidebar-user">
            <div className="user-avatar">{user?.username?.[0]?.toUpperCase()}</div>
            <div className="user-info">
              <div className="user-name">{user?.full_name || user?.username}</div>
              <div className="user-role">{user?.role}</div>
            </div>
          </div>
          <button className="logout-btn" onClick={toggleTheme} title={theme === 'dark' ? 'Svijetla tema' : 'Tamna tema'}>
            {theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
          </button>
          <button className="logout-btn" onClick={handleLogout} title="Odjavi se">
            <LogOut size={16} />
          </button>
        </div>
      </aside>

      <main className="content">
        <div className="mobile-topbar">
          <div className="mobile-logo">
            <LighthouseIcon size={18} />
            <span>Beacon</span>
          </div>
          <div className="mobile-topbar-actions">
            <button className="icon-btn" onClick={toggleTheme} title={theme === 'dark' ? 'Svijetla tema' : 'Tamna tema'}>
              {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
            </button>
            <button className="icon-btn" onClick={handleLogout} title="Odjavi se">
              <LogOut size={18} />
            </button>
            <button className="hamburger-btn" onClick={() => setSidebarOpen(true)} title="Više">
              <Menu size={22} />
            </button>
          </div>
        </div>
        <div className="content-inner">
          <Outlet />
        </div>
      </main>

      {/* Bottom navigation — mobile only */}
      <nav className="bottom-nav">
        <NavLink
          to="/dashboard"
          className={`bottom-nav-item${location.pathname === '/dashboard' ? ' active' : ''}`}
        >
          <LayoutDashboard size={22} />
          <span>Dashboard</span>
        </NavLink>
        <NavLink
          to="/objects"
          className={`bottom-nav-item${location.pathname.startsWith('/objects') ? ' active' : ''}`}
        >
          <Radio size={22} />
          <span>Objekti</span>
        </NavLink>
        <NavLink
          to="/alarms"
          className={`bottom-nav-item${location.pathname === '/alarms' ? ' active' : ''}`}
        >
          <AlertTriangle size={22} />
          <span>Alarmi</span>
        </NavLink>
        <NavLink
          to="/compare"
          className={`bottom-nav-item${location.pathname === '/compare' ? ' active' : ''}`}
        >
          <GitCompare size={22} />
          <span>Usporedi</span>
        </NavLink>
        <NavLink
          to="/map"
          className={`bottom-nav-item${location.pathname === '/map' ? ' active' : ''}`}
        >
          <Map size={22} />
          <span>Karta</span>
        </NavLink>
        <button
          className={`bottom-nav-item${location.pathname.startsWith('/admin') ? ' active' : ''}`}
          onClick={() => setSidebarOpen(true)}
        >
          <Menu size={22} />
          <span>{isAdmin ? 'Admin' : 'Više'}</span>
        </button>
      </nav>
    </div>
  );
}
