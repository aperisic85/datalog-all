import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { MapContainer, TileLayer, CircleMarker, Popup, useMap } from 'react-leaflet';
import MarkerClusterGroup from 'react-leaflet-cluster';
import L from 'leaflet';
import { Link } from 'react-router-dom';
import { listObjects, listRegions } from '../api/endpoints';
import type { ObjectView } from '../types';
import 'leaflet/dist/leaflet.css';
import 'leaflet.markercluster/dist/MarkerCluster.css';
import './MapPage.css';

const ALARM_COLOR = '#ef4444';
const ACTIVE_COLOR = '#22c55e';
const INACTIVE_COLOR = '#6b7280';

function markerColor(obj: ObjectView): string {
  if (!obj.is_active) return INACTIVE_COLOR;
  if (obj.alarm_active) return ALARM_COLOR;
  return ACTIVE_COLOR;
}

function AlarmBadge({ obj }: { obj: ObjectView }) {
  if (!obj.alarm_active) return null;
  return (
    <span className="map-alarm-badge">
      ⚠ {obj.alarm_count} alarm{obj.alarm_count !== 1 ? 'a' : ''}
    </span>
  );
}

// Cluster ikona — boja ovisi o tome ima li grupa objekata u alarmu (gustoća alarma)
function clusterIcon(cluster: { getChildCount: () => number; getAllChildMarkers: () => { options?: { fillColor?: string } }[] }) {
  const count = cluster.getChildCount();
  const children = cluster.getAllChildMarkers();
  const hasAlarm = children.some((m) => m.options?.fillColor === ALARM_COLOR);
  const size = count < 10 ? 'sm' : count < 50 ? 'md' : 'lg';
  return L.divIcon({
    html: `<div><span>${count}</span></div>`,
    className: `map-cluster map-cluster-${size}${hasAlarm ? ' map-cluster-alarm' : ''}`,
    iconSize: L.point(40, 40, true),
  });
}

// Prilagodi prikaz granicama filtriranih objekata kad se promijeni filter/regija
function FitToObjects({ objects }: { objects: ObjectView[] }) {
  const map = useMap();
  const key = objects.map((o) => o.id).join(',');
  useEffect(() => {
    if (objects.length === 0) return;
    const bounds = L.latLngBounds(objects.map((o) => [o.latitude!, o.longitude!] as [number, number]));
    map.fitBounds(bounds, { padding: [40, 40], maxZoom: 13, animate: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return null;
}

type StatusFilter = 'all' | 'alarm' | 'inactive';

export default function MapPage() {
  const [filter, setFilter] = useState<StatusFilter>('all');
  const [regionFilter, setRegionFilter] = useState('');

  const { data, isLoading } = useQuery({
    queryKey: ['objects-map'],
    queryFn: () => listObjects({ page_size: 500 }),
  });
  const { data: regions } = useQuery({ queryKey: ['regions'], queryFn: listRegions });

  const objects = useMemo(
    () => (data?.data ?? []).filter((o) => o.latitude && o.longitude),
    [data]
  );

  // Skup nakon regionalnog filtra — baza za statistiku i prikaz
  const inRegion = useMemo(
    () => (regionFilter ? objects.filter((o) => o.region_id === regionFilter) : objects),
    [objects, regionFilter]
  );

  const filtered = useMemo(
    () =>
      inRegion.filter((o) => {
        if (filter === 'alarm') return o.alarm_active;
        if (filter === 'inactive') return !o.is_active;
        return true;
      }),
    [inRegion, filter]
  );

  const center: [number, number] =
    objects.length > 0
      ? [
          objects.reduce((s, o) => s + o.latitude!, 0) / objects.length,
          objects.reduce((s, o) => s + o.longitude!, 0) / objects.length,
        ]
      : [44.8, 16.5]; // Hrvatska

  const alarmCount = inRegion.filter((o) => o.alarm_active).length;
  const inactiveCount = inRegion.filter((o) => !o.is_active).length;

  return (
    <div className="map-page">
      <div className="map-header">
        <div className="map-title">
          <h1>Karta objekata</h1>
          <span className="map-subtitle">{inRegion.length} objekata s koordinatama</span>
        </div>
        <div className="map-controls">
          <select
            className="map-region-select"
            value={regionFilter}
            onChange={(e) => setRegionFilter(e.target.value)}
          >
            <option value="">Sve regije</option>
            {regions?.map((r) => (
              <option key={r.id} value={r.id}>{r.name}</option>
            ))}
          </select>
          <div className="map-filters">
            <button
              className={`map-filter-btn ${filter === 'all' ? 'active' : ''}`}
              onClick={() => setFilter('all')}
            >
              Svi ({inRegion.length})
            </button>
            <button
              className={`map-filter-btn alarm ${filter === 'alarm' ? 'active' : ''}`}
              onClick={() => setFilter('alarm')}
            >
              Alarmi ({alarmCount})
            </button>
            <button
              className={`map-filter-btn inactive ${filter === 'inactive' ? 'active' : ''}`}
              onClick={() => setFilter('inactive')}
            >
              Neaktivni ({inactiveCount})
            </button>
          </div>
        </div>
      </div>

      <div className="map-legend">
        <span className="legend-item"><span className="legend-dot active" />Aktivan</span>
        <span className="legend-item"><span className="legend-dot alarm" />Alarm</span>
        <span className="legend-item"><span className="legend-dot inactive" />Neaktivan</span>
        <span className="legend-item map-legend-hint">Skupine se otvaraju klikom</span>
      </div>

      {isLoading ? (
        <div className="map-loading"><div className="spinner" /></div>
      ) : (
        <div className="map-container">
          <MapContainer center={center} zoom={8} style={{ height: '100%', width: '100%' }}>
            <TileLayer
              attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
              url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            <FitToObjects objects={filtered} />
            <MarkerClusterGroup
              chunkedLoading
              maxClusterRadius={50}
              showCoverageOnHover={false}
              spiderfyOnMaxZoom
              iconCreateFunction={clusterIcon}
            >
              {filtered.map((obj) => (
                <CircleMarker
                  key={obj.id}
                  center={[obj.latitude!, obj.longitude!]}
                  radius={8}
                  pathOptions={{
                    color: markerColor(obj),
                    fillColor: markerColor(obj),
                    fillOpacity: 0.85,
                    weight: 2,
                  }}
                >
                  <Popup>
                    <div className="map-popup">
                      <div className="popup-name">{obj.name}</div>
                      <div className="popup-meta">
                        <span>{obj.station_id}</span>
                        <span className="popup-region" style={{ color: obj.region_color }}>
                          {obj.region_name}
                        </span>
                      </div>
                      {obj.location_name && (
                        <div className="popup-location">📍 {obj.location_name}</div>
                      )}
                      <AlarmBadge obj={obj} />
                      <Link to={`/objects/${obj.id}`} className="popup-link">
                        Otvori detalje →
                      </Link>
                    </div>
                  </Popup>
                </CircleMarker>
              ))}
            </MarkerClusterGroup>
          </MapContainer>
        </div>
      )}
    </div>
  );
}
