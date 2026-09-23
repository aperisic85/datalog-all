import { useState, useEffect, type ReactNode } from 'react';
import type { UserPublic } from '../types';
import * as endpoints from '../api/endpoints';
import { AuthContext } from './auth';

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<UserPublic | null>(null);
  const [loading, setLoading] = useState(() => localStorage.getItem('access_token') !== null);

  useEffect(() => {
    const token = localStorage.getItem('access_token');
    if (!token) return;

    endpoints.me()
      .then(setUser)
      .catch(() => {
        localStorage.removeItem('access_token');
        localStorage.removeItem('refresh_token');
      })
      .finally(() => setLoading(false));
  }, []);

  const login = async (username: string, password: string) => {
    const data = await endpoints.login(username, password);
    localStorage.setItem('access_token', data.access_token);
    localStorage.setItem('refresh_token', data.refresh_token);
    setUser(data.user);
  };

  const logout = async () => {
    const refreshToken = localStorage.getItem('refresh_token');
    if (refreshToken) {
      try {
        await endpoints.logout(refreshToken);
      } catch { /* ignore */ }
    }
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, loading, login, logout, isAdmin: user?.role === 'admin' }}>
      {children}
    </AuthContext.Provider>
  );
}
