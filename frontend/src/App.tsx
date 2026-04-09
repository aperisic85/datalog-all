import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AuthProvider, useAuth } from './context/AuthContext';
import Layout from './components/Layout';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import ObjectsPage from './pages/ObjectsPage';
import ObjectDetailPage from './pages/ObjectDetailPage';
import AdminRegionsPage from './pages/AdminRegionsPage';
import AdminUsersPage from './pages/AdminUsersPage';
import MapPage from './pages/MapPage';
import AlarmsPage from './pages/AlarmsPage';
import type { ReactNode } from 'react';

const qc = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: 1 },
  },
});

function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) return <div className="page-spinner" style={{ height: '100vh' }}><div className="spinner" style={{ width: 36, height: 36 }} /></div>;
  if (!user) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function RequireAdmin({ children }: { children: ReactNode }) {
  const { isAdmin } = useAuth();
  if (!isAdmin) return <Navigate to="/dashboard" replace />;
  return <>{children}</>;
}

function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route
        path="/"
        element={
          <RequireAuth>
            <Layout />
          </RequireAuth>
        }
      >
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<DashboardPage />} />
        <Route path="objects" element={<ObjectsPage />} />
        <Route path="objects/:id" element={<ObjectDetailPage />} />
        <Route path="map" element={<MapPage />} />
        <Route path="alarms" element={<AlarmsPage />} />
        <Route
          path="admin/regions"
          element={
            <RequireAdmin>
              <AdminRegionsPage />
            </RequireAdmin>
          }
        />
        <Route
          path="admin/users"
          element={
            <RequireAdmin>
              <AdminUsersPage />
            </RequireAdmin>
          }
        />
      </Route>
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </Routes>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={qc}>
      <AuthProvider>
        <BrowserRouter>
          <AppRoutes />
        </BrowserRouter>
      </AuthProvider>
    </QueryClientProvider>
  );
}
