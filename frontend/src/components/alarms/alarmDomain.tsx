import type { ReactNode } from 'react';
import {
  AlertTriangle,
  Battery,
  DoorOpen,
  Eye,
  Lightbulb,
  MapPin,
  PhoneCall,
  Thermometer,
  Wifi,
  WifiOff,
  Wind,
  Zap,
} from 'lucide-react';
import type { AlarmListItem, AlarmShelf } from '../../types';

export type AlarmKey = keyof Pick<AlarmListItem,
  'alarm_battery_voltage_flat' | 'alarm_battery_voltage_low' | 'alarm_battery_other_error' |
  'alarm_datalogger_high_temp' | 'alarm_datalogger_high_voltage' | 'alarm_datalogger_other_error' |
  'alarm_garmin_comm_failed' | 'alarm_garmin_other_error' | 'alarm_station_out_of_radius' |
  'alarm_lantern_night_light_off' | 'alarm_lantern_day_light_on' |
  'alarm_lantern_comm_failed' | 'alarm_lantern_other_error' |
  'alarm_modem_network_error' | 'alarm_modem_other_error' | 'alarm_station_other_error' |
  'alarm_visibility_comm_failed' | 'alarm_visibility_error' |
  'alarm_fog_signal_off_during_fog' | 'alarm_fog_signal_on_while_no_fog' |
  'alarm_aton_call_request' | 'alarm_aton_temperature' |
  'alarm_aton_voltage_light' | 'alarm_aton_voltage_automat' |
  'alarm_aton_door_open' | 'alarm_aton_flash_code' |
  'alarm_aton_light_on_automat' | 'alarm_aton_automat_on_light' |
  'alarm_aton_lamp_blown' | 'alarm_aton_not_work_at_night' |
  'alarm_aton_photocell_error' | 'alarm_aton_work_at_day'
>;

type AlarmDefinition = {
  key: AlarmKey;
  label: string;
  icon: ReactNode;
  severity: 'danger' | 'warning';
};

export const ALARM_DEFS: AlarmDefinition[] = [
  { key: 'alarm_battery_voltage_flat', label: 'Baterija prazna', icon: <Battery size={12} />, severity: 'danger' },
  { key: 'alarm_battery_voltage_low', label: 'Baterija slaba', icon: <Battery size={12} />, severity: 'warning' },
  { key: 'alarm_battery_other_error', label: 'Greška baterije', icon: <Battery size={12} />, severity: 'warning' },
  { key: 'alarm_datalogger_high_temp', label: 'Visoka temp.', icon: <Thermometer size={12} />, severity: 'warning' },
  { key: 'alarm_datalogger_high_voltage', label: 'Visoki napon', icon: <Zap size={12} />, severity: 'warning' },
  { key: 'alarm_datalogger_other_error', label: 'Greška datalogera', icon: <AlertTriangle size={12} />, severity: 'warning' },
  { key: 'alarm_garmin_comm_failed', label: 'GPS komunikacija pala', icon: <MapPin size={12} />, severity: 'danger' },
  { key: 'alarm_garmin_other_error', label: 'GPS greška', icon: <MapPin size={12} />, severity: 'warning' },
  { key: 'alarm_station_out_of_radius', label: 'Van radijusa', icon: <MapPin size={12} />, severity: 'danger' },
  { key: 'alarm_lantern_night_light_off', label: 'Svjetlo ugašeno noću', icon: <Zap size={12} />, severity: 'danger' },
  { key: 'alarm_lantern_day_light_on', label: 'Svjetlo upaljeno danju', icon: <Zap size={12} />, severity: 'warning' },
  { key: 'alarm_lantern_comm_failed', label: 'Svjetlo komun. pala', icon: <WifiOff size={12} />, severity: 'danger' },
  { key: 'alarm_lantern_other_error', label: 'Svjetlo greška', icon: <Zap size={12} />, severity: 'warning' },
  { key: 'alarm_modem_network_error', label: 'Greška mreže', icon: <Wifi size={12} />, severity: 'warning' },
  { key: 'alarm_modem_other_error', label: 'Greška modema', icon: <WifiOff size={12} />, severity: 'warning' },
  { key: 'alarm_station_other_error', label: 'Greška stanice', icon: <AlertTriangle size={12} />, severity: 'warning' },
  { key: 'alarm_visibility_comm_failed', label: 'Vidljivost: greška veze', icon: <Eye size={12} />, severity: 'danger' },
  { key: 'alarm_visibility_error', label: 'Vidljivost: greška senzora', icon: <Eye size={12} />, severity: 'warning' },
  { key: 'alarm_fog_signal_off_during_fog', label: 'Sirena: nije aktivna u magli', icon: <Wind size={12} />, severity: 'danger' },
  { key: 'alarm_fog_signal_on_while_no_fog', label: 'Sirena: aktivna bez magle', icon: <Wind size={12} />, severity: 'warning' },
  { key: 'alarm_aton_lamp_blown', label: 'Pregorena žarulja', icon: <Lightbulb size={12} />, severity: 'danger' },
  { key: 'alarm_aton_not_work_at_night', label: 'Ne radi po noći', icon: <Lightbulb size={12} />, severity: 'danger' },
  { key: 'alarm_aton_photocell_error', label: 'Greška fotoćelije', icon: <Eye size={12} />, severity: 'danger' },
  { key: 'alarm_aton_flash_code', label: 'Pogrešna karakteristika', icon: <Lightbulb size={12} />, severity: 'danger' },
  { key: 'alarm_aton_voltage_light', label: 'Napon baterije GL.SVJ.', icon: <Battery size={12} />, severity: 'danger' },
  { key: 'alarm_aton_voltage_automat', label: 'Napon baterije automata', icon: <Battery size={12} />, severity: 'danger' },
  { key: 'alarm_aton_work_at_day', label: 'Svjetlo radi po danu', icon: <Lightbulb size={12} />, severity: 'warning' },
  { key: 'alarm_aton_light_on_automat', label: 'Svjetlo na bat. automata', icon: <Battery size={12} />, severity: 'warning' },
  { key: 'alarm_aton_automat_on_light', label: 'Automat na bat. svjetla', icon: <Battery size={12} />, severity: 'warning' },
  { key: 'alarm_aton_temperature', label: 'Temp. izvan granica', icon: <Thermometer size={12} />, severity: 'warning' },
  { key: 'alarm_aton_door_open', label: 'Vrata otvorena', icon: <DoorOpen size={12} />, severity: 'warning' },
  { key: 'alarm_aton_call_request', label: 'Zahtjev za pozivom', icon: <PhoneCall size={12} />, severity: 'warning' },
];

const CRITICAL_KEYS = ALARM_DEFS.filter(def => def.severity === 'danger').map(def => def.key);

export function isCriticalAlarm(item: AlarmListItem) {
  return CRITICAL_KEYS.some(key => item[key] > 0);
}

export type AlarmSeverity = 'critical' | 'warning' | 'ack';

export function severityOf(item: AlarmListItem): AlarmSeverity {
  if (item.acknowledged_at) return 'ack';
  return isCriticalAlarm(item) ? 'critical' : 'warning';
}

export const shelfKey = (key: AlarmKey) => key.replace(/^alarm_/, '');

export const shelfTypeLabel = (type: string | null) =>
  type === null ? 'Svi alarmi' : (ALARM_DEFS.find(def => shelfKey(def.key) === type)?.label ?? type);

export function isItemShelved(item: AlarmListItem, shelves: AlarmShelf[]): boolean {
  const own = shelves.filter(shelf => shelf.object_id === item.object_id);
  if (own.length === 0) return false;
  if (own.some(shelf => shelf.alarm_type === null)) return true;

  const activeKeys = ALARM_DEFS
    .filter(def => item[def.key] > 0)
    .map(def => shelfKey(def.key));

  return activeKeys.length > 0 && activeKeys.every(key => own.some(shelf => shelf.alarm_type === key));
}

export const SHELF_DURATIONS: { minutes: number; label: string }[] = [
  { minutes: 30, label: '30 minuta' },
  { minutes: 60, label: '1 sat' },
  { minutes: 4 * 60, label: '4 sata' },
  { minutes: 8 * 60, label: '8 sati' },
  { minutes: 24 * 60, label: '24 sata' },
  { minutes: 3 * 1440, label: '3 dana' },
  { minutes: 7 * 1440, label: '7 dana' },
];

export function AlarmTags({ item }: { item: AlarmListItem }) {
  const active = ALARM_DEFS.filter(def => item[def.key] > 0);
  if (active.length === 0) return null;

  return (
    <div className="alarm-tag-list">
      {active.map(def => (
        <span key={def.key} className={`alarm-tag alarm-tag-${def.severity}`}>
          {def.icon} {def.label}
        </span>
      ))}
    </div>
  );
}
