import axios, { type AxiosError, type InternalAxiosRequestConfig } from 'axios';

// U produkciji (Docker) VITE_API_URL nije postavljen — koristimo relativni path
// pa nginx proxy hvata /api/* i prosljeđuje backendu.
// Za lokalni dev: VITE_API_URL=http://localhost:8095
const BASE_URL = import.meta.env.VITE_API_URL ?? '';

interface RetryableRequestConfig extends InternalAxiosRequestConfig {
  _retry?: boolean;
}

interface RefreshResponse {
  access_token: string;
}

let refreshPromise: Promise<string> | null = null;

function clearSession() {
  localStorage.removeItem('access_token');
  localStorage.removeItem('refresh_token');
}

function redirectToLogin() {
  clearSession();
  if (window.location.pathname !== '/login') {
    window.location.assign('/login');
  }
}

async function refreshAccessToken(): Promise<string> {
  if (refreshPromise) return refreshPromise;

  const refreshToken = localStorage.getItem('refresh_token');
  if (!refreshToken) {
    return Promise.reject(new Error('Missing refresh token'));
  }

  refreshPromise = axios
    .post<RefreshResponse>(`${BASE_URL}/api/v1/auth/refresh`, {
      refresh_token: refreshToken,
    })
    .then(({ data }) => {
      localStorage.setItem('access_token', data.access_token);
      return data.access_token;
    })
    .finally(() => {
      refreshPromise = null;
    });

  return refreshPromise;
}

export const api = axios.create({
  baseURL: BASE_URL,
  headers: { 'Content-Type': 'application/json' },
});

api.interceptors.request.use((config) => {
  const token = localStorage.getItem('access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

api.interceptors.response.use(
  (res) => res,
  async (err: AxiosError) => {
    const original = err.config as RetryableRequestConfig | undefined;

    // Auth endpoint greške propagiramo pozivatelju bez automatskog refresha.
    if (!original || original.url?.includes('/api/v1/auth/')) {
      return Promise.reject(err);
    }

    if (err.response?.status !== 401 || original._retry) {
      return Promise.reject(err);
    }

    original._retry = true;

    try {
      // Svi paralelni 401 odgovori čekaju isti refresh request.
      const accessToken = await refreshAccessToken();
      original.headers.Authorization = `Bearer ${accessToken}`;
      return api(original);
    } catch (refreshError) {
      redirectToLogin();
      return Promise.reject(refreshError);
    }
  }
);
