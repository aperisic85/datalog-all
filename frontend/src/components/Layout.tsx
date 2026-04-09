import { NavLink, Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import {
  LayoutDashboard,
  Radio,
  Users,
  MapPin,
  LogOut,
  Activity,
  Sun,
  Moon,
  Map,
  Menu,
  X,
  AlertTriangle,
} from 'lucide-react';
import './Layout.css';

export default function Layout() {
  const { user, logout, isAdmin } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    return (localStorage.getItem('theme') as 'dark' | 'light') || 'dark';
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
          <Activity size={20} />
          <span>DataLogger</span>
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
            <Activity size={18} />
            <span>DataLogger</span>
          </div>
          <div className="mobile-topbar-actions">
            <button className="icon-btn" onClick={toggleTheme} title={theme === 'dark' ? 'Svijetla tema' : 'Tamna tema'}>
              {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
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
