import type { ControllerDevice } from '../models';

type BatteryDetails = Pick<ControllerDevice, 'batteryLevel' | 'charging' | 'charged'>;

export function formatBatteryStatus(device: BatteryDetails): string {
  const percentage = device.batteryLevel === null ? null : `${Math.round(device.batteryLevel)}%`;

  if (device.charged) return percentage ? `${percentage} · Charged` : 'Charged';
  if (device.charging) return percentage ? `${percentage} · Charging` : 'Charging';
  return percentage ? `${percentage} battery` : 'Battery level unknown';
}

export function formatCompactBatteryStatus(device: BatteryDetails): string {
  if (device.batteryLevel !== null) return `${Math.round(device.batteryLevel)}%`;
  if (device.charged) return 'Charged';
  if (device.charging) return 'Charging';
  return 'Battery —';
}
