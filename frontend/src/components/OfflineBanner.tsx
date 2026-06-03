import { useState, useEffect } from 'react';
import { WifiOff, Wifi } from 'lucide-react';
import './OfflineBanner.css';

export default function OfflineBanner() {
  const [isOnline, setIsOnline] = useState(navigator.onLine);
  const [showReconnected, setShowReconnected] = useState(false);

  useEffect(() => {
    const handleOnline = () => {
      setIsOnline(true);
      setShowReconnected(true);
      setTimeout(() => setShowReconnected(false), 3000);
    };
    const handleOffline = () => {
      setIsOnline(false);
      setShowReconnected(false);
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  if (isOnline && !showReconnected) return null;

  return (
    <div className={`offline-banner ${isOnline ? 'offline-banner-online' : 'offline-banner-offline'}`}>
      {isOnline ? (
        <><Wifi size={14} /> Veza uspostavljena — podaci se osvježavaju</>
      ) : (
        <><WifiOff size={14} /> Nema internetske veze — prikazuju se zadnji poznati podaci</>
      )}
    </div>
  );
}
