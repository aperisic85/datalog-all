import { api } from './client';
import type {
  LoginResponse,
  Region,
  RegionSummary,
  StationType,
  ObjectView,
  Page,
  Measurement10min,
  Measurement1h,
  AlarmRecord,
  EventLogRecord,
  LatestMeasurement,
  UserPublic,
  UserRegionAccessView,
} from '../types';

// Auth
export const login = (username: string, password: string) =>
  api.post<LoginResponse>('/api/v1/auth/login', { username, password }).then((r) => r.data);

export const logout = (refresh_token: string) =>
  api.post('/api/v1/auth/logout', { refresh_token });

export const me = () =>
  api.get<UserPublic>('/api/v1/auth/me').then((r) => r.data);

// Regions
export const listRegions = () =>
  api.get<Region[]>('/api/v1/regions').then((r) => r.data);

export const regionSummary = () =>
  api.get<RegionSummary[]>('/api/v1/regions/summary').then((r) => r.data);

export const createRegion = (data: {
  name: string;
  code: string;
  description?: string;
  color?: string;
}) => api.post<Region>('/api/v1/regions', data).then((r) => r.data);

export const updateRegion = (id: string, data: {
  name?: string;
  description?: string;
  color?: string;
  is_active?: boolean;
}) => api.put<Region>(`/api/v1/regions/${id}`, data).then((r) => r.data);

// Station types
export const listStationTypes = () =>
  api.get<StationType[]>('/api/v1/station-types').then((r) => r.data);

// Objects
export interface ObjectsQueryParams {
  page?: number;
  page_size?: number;
  search?: string;
  region_id?: string;
  active?: boolean;
  in_alarm?: boolean;
}

export const listObjects = (params?: ObjectsQueryParams) =>
  api.get<Page<ObjectView>>('/api/v1/objects', { params }).then((r) => r.data);

export const getObject = (id: string) =>
  api.get<ObjectView>(`/api/v1/objects/${id}`).then((r) => r.data);

export const createObject = (data: Record<string, unknown>) =>
  api.post<ObjectView>('/api/v1/objects', data).then((r) => r.data);

export const updateObject = (id: string, data: Record<string, unknown>) =>
  api.patch<ObjectView>(`/api/v1/objects/${id}`, data).then((r) => r.data);

export const deleteObject = (id: string) =>
  api.delete(`/api/v1/objects/${id}`);

export const pollObject = (id: string) =>
  api.post<{ station_id: string; results: { table: string; records?: number; error?: string }[] }>(
    `/api/v1/objects/${id}/poll`
  ).then((r) => r.data);

// Measurements
export const getMeasurements10min = (id: string, params?: { from?: string; to?: string; limit?: number }) =>
  api.get<Measurement10min[]>(`/api/v1/objects/${id}/measurements/10min`, { params }).then((r) => r.data);

export const getMeasurements1h = (id: string, params?: { from?: string; to?: string; limit?: number }) =>
  api.get<Measurement1h[]>(`/api/v1/objects/${id}/measurements/1h`, { params }).then((r) => r.data);

export const getLatestMeasurement = (id: string) =>
  api.get<LatestMeasurement>(`/api/v1/objects/${id}/measurements/latest`).then((r) => r.data);

// Alarms
export const getAlarms = (id: string, params?: { from?: string; to?: string; limit?: number }) =>
  api.get<AlarmRecord[]>(`/api/v1/objects/${id}/alarms`, { params }).then((r) => r.data);

export const getActiveAlarms = (id: string) =>
  api.get<AlarmRecord[]>(`/api/v1/objects/${id}/alarms/active`).then((r) => r.data);

// Event logs
export const getEventLogs = (id: string, params?: { from?: string; to?: string; limit?: number }) =>
  api.get<EventLogRecord[]>(`/api/v1/objects/${id}/eventlogs`, { params }).then((r) => r.data);

// Users
export const listUsers = () =>
  api.get<UserPublic[]>('/api/v1/users').then((r) => r.data);

export const createUser = (data: {
  username: string;
  email: string;
  password: string;
  full_name?: string;
  role: string;
}) => api.post<UserPublic>('/api/v1/users', data).then((r) => r.data);

export const getUserRegions = (userId: string) =>
  api.get<UserRegionAccessView[]>(`/api/v1/users/${userId}/regions`).then((r) => r.data);

export const grantRegionAccess = (data: {
  user_id: string;
  region_id: string;
  permission: string;
}) => api.post('/api/v1/users/regions', data).then((r) => r.data);

export const revokeRegionAccess = (userId: string, regionId: string) =>
  api.delete(`/api/v1/users/${userId}/regions/${regionId}`);
