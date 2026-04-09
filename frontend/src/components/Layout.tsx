import { NavLink, Outlet, useNavigate } from 'react-router-dom';
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
} from 'lucide-react';
import './Layout.css';

export default function Layout() {
  const { user, logout, isAdmin } = useAuth();
  const navigate = useNavigate();
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
          <button className="hamburger-btn" onClick={() => setSidebarOpen(true)} title="Meni">
            <Menu size={22} />
          </button>
          <div className="mobile-logo">
            <Activity size={18} />
            <span>DataLogger</span>
          </div>
          <button className="logout-btn" onClick={toggleTheme} title={theme === 'dark' ? 'Svijetla tema' : 'Tamna tema'}>
            {theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
          </button>
        </div>
        <div className="content-inner">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
