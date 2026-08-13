import { describe, expect, it } from 'vitest';
import { formatBatteryStatus, formatCompactBatteryStatus } from './battery';

describe('battery presentation', () => {
  it('keeps an unknown percentage distinct from charging state', () => {
    expect(formatBatteryStatus({ batteryLevel: null, charging: true, charged: false })).toBe(
      'Charging',
    );
    expect(formatBatteryStatus({ batteryLevel: null, charging: false, charged: true })).toBe(
      'Charged',
    );
    expect(formatBatteryStatus({ batteryLevel: null, charging: false, charged: false })).toBe(
      'Battery level unknown',
    );
    expect(
      formatCompactBatteryStatus({ batteryLevel: null, charging: false, charged: false }),
    ).toBe('Battery —');
  });

  it('renders a known percentage without inventing a power state', () => {
    expect(formatBatteryStatus({ batteryLevel: 67, charging: false, charged: false })).toBe(
      '67% battery',
    );
    expect(formatBatteryStatus({ batteryLevel: 67, charging: true, charged: false })).toBe(
      '67% · Charging',
    );
  });
});
