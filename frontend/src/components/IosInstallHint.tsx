import { useState } from 'react';
import { Share, SquarePlus, X } from 'lucide-react';
import './IosInstallHint.css';

const DISMISS_KEY = 'ios-install-hint-dismissed';

function isIos(): boolean {
  const ua = navigator.userAgent;
  // iPadOS se od 13+ predstavlja kao Macintosh, pa provjeravamo i touch
  return /iPhone|iPad|iPod/.test(ua)
    || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1);
}

function isStandalone(): boolean {
  return window.matchMedia('(display-mode: standalone)').matches
    || (navigator as unknown as { standalone?: boolean }).standalone === true;
}

export default function IosInstallHint() {
  const [dismissed, setDismissed] = useState(
    () => localStorage.getItem(DISMISS_KEY) === '1'
  );

  if (dismissed || !isIos() || isStandalone()) return null;

  const dismiss = () => {
    localStorage.setItem(DISMISS_KEY, '1');
    setDismissed(true);
  };

  return (
    <div className="ios-install-hint">
      <div className="ios-install-hint-text">
        Instaliraj Beacon na početni zaslon: otvori u Safariju, dodirni{' '}
        <Share size={14} className="ios-install-hint-icon" /> <strong>Podijeli</strong>, zatim{' '}
        <SquarePlus size={14} className="ios-install-hint-icon" /> <strong>Dodaj na početni zaslon</strong>
      </div>
      <button className="ios-install-hint-close" onClick={dismiss} title="Zatvori">
        <X size={16} />
      </button>
    </div>
  );
}
