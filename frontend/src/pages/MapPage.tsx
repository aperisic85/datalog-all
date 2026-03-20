import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { MapContainer, TileLayer, CircleMarker, Popup } from 'react-leaflet';
import { Link } from 'react-router-dom';
import { listObjects } from '../api/endpoints';
import type { ObjectView } from '../types';
import 'leaflet/dist/leaflet.css';
import './MapPage.css';

function markerColor(obj: ObjectView): string {
  if (!obj.is_active) return '#6b7280';
  if (obj.alarm_active) return '#ef4444';
  return '#22c55e';
}

function AlarmBadge({ obj }: { obj: ObjectView }) {
  if (!obj.alarm_active) return null;
  return (
    <span className="map-alarm-badge">
      ⚠ {obj.alarm_count} alarm{obj.alarm_count !== 1 ? 'a' : ''}
    </span>
  );
}

export default function MapPage() {
  const [filter, setFilter] = useState<'all' | 'alarm' | 'inactive'>('all');

  const { data, isLoading } = useQuery({
    queryKey: ['objects-map'],
    queryFn: () => listObjects({ page_size: 500 }),
  });

  const objects = (data?.data ?? []).filter((o) => o.latitude && o.longitude);
  const filtered = objects.filter((o) => {
    if (filter === 'alarm') return o.alarm_active;
    if (filter === 'inactive') return !o.is_active;
    return true;
  });

  const center: [number, number] =
    objects.length > 0
      ? [
          objects.reduce((s, o) => s + o.latitude!, 0) / objects.length,
          objects.reduce((s, o) => s + o.longitude!, 0) / objects.length,
        ]
      : [44.8, 16.5]; // Hrvatska

  const alarmCount = objects.filter((o) => o.alarm_active).length;
  const inactiveCount = objects.filter((o) => !o.is_active).length;

  return (
    <div className="map-page">
      <div className="map-header">
        <div className="map-title">
          <h1>Karta objekata</h1>
          <span className="map-subtitle">{objects.length} objekata s koordinatama</span>
        </div>
        <div className="map-filters">
          <button
            className={`map-filter-btn ${filter === 'all' ? 'active' : ''}`}
            onClick={() => setFilter('all')}
          >
            Svi ({objects.length})
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

      <div className="map-legend">
        <span className="legend-item"><span className="legend-dot active" />Aktivan</span>
        <span className="legend-item"><span className="legend-dot alarm" />Alarm</span>
        <span className="legend-item"><span className="legend-dot inactive" />Neaktivan</span>
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
          </MapContainer>
        </div>
      )}
    </div>
  );
}
