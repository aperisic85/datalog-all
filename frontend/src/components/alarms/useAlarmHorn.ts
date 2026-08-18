import { useEffect, useRef, useState } from 'react';

// Klasična SCADA sirena: svira dok postoji nepotvrđeni kritični alarm.
// Utišavanje vrijedi do pojave novog kritičnog alarma.
export function useAlarmHorn(criticalCount: number) {
  const [enabled, setEnabled] = useState(() => localStorage.getItem('alarm-horn') === '1');
  const [silenced, setSilenced] = useState(false);
  const previousCount = useRef(criticalCount);

  useEffect(() => {
    if (criticalCount > previousCount.current) {
      setSilenced(false);
    }
    previousCount.current = criticalCount;
  }, [criticalCount]);

  const sounding = enabled && !silenced && criticalCount > 0;

  useEffect(() => {
    if (!sounding) return;

    const ctx = new AudioContext();
    const beep = () => {
      if (ctx.state !== 'running') {
        void ctx.resume();
        return;
      }

      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'square';
      osc.frequency.value = 880;
      gain.gain.setValueAtTime(0.05, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.25);
      osc.connect(gain).connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.25);
    };

    beep();
    const timer = window.setInterval(beep, 2500);

    return () => {
      window.clearInterval(timer);
      void ctx.close();
    };
  }, [sounding]);

  const toggle = () => setEnabled(current => {
    const next = !current;
    localStorage.setItem('alarm-horn', next ? '1' : '0');
    return next;
  });

  return {
    enabled,
    toggle,
    sounding,
    silence: () => setSilenced(true),
  };
}
