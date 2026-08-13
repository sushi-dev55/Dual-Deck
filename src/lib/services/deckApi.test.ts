import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { triggerModes } from '../models';
import type { MappedAction } from '../models';
import {
  actionFromNative,
  actionToNative,
  bindingDraft,
  BrowserPreviewDeckApi,
  controllerFromNative,
  createInitialState,
  mappingFromNative,
  profileFromNative,
  triggerConfiguration,
  triggerFromNative,
  triggerToNative,
  type NativeBinding,
} from './deckApi';

const profileId = '6e64cecb-78ee-4dc7-991e-29ca2266d960';
const bindingId = 'ddff92e7-e013-4f30-854e-dff854434734';

function mapping(overrides: Partial<MappedAction> = {}): MappedAction {
  return {
    id: bindingId,
    actionId: 'launch-application',
    title: 'Launch application',
    detail: 'Choose an application',
    icon: 'app',
    accent: '#84a7ff',
    trigger: 'Press',
    configuration: {},
    enabled: true,
    ...overrides,
  };
}

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => values.delete(key),
    setItem: (key, value) => values.set(key, value),
  };
}

beforeEach(() => {
  vi.stubGlobal('localStorage', memoryStorage());
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('browser preview state', () => {
  it('starts disconnected with production-aligned defaults and a usable profile', async () => {
    const state = await new BrowserPreviewDeckApi().load();

    expect(state.profiles).toHaveLength(1);
    expect(state.profiles[0]).toMatchObject({ name: 'Default', mappings: {} });
    expect(state.activeProfileId).toBe(state.profiles[0].id);
    expect(state.device).toMatchObject({ connected: false, connection: 'Unknown' });
    expect(state.pressedControls).toEqual([]);
    expect(state.preferences).toMatchObject({
      startWithWindows: true,
      launchMinimized: true,
      minimizeToTray: true,
      closeToTray: true,
      checkForUpdates: false,
    });
  });

  it('keeps saved preferences while discarding simulated controller state', async () => {
    const api = new BrowserPreviewDeckApi();
    const state = createInitialState();
    state.preferences.reducedMotion = true;
    state.device = {
      connected: true,
      name: 'Simulated controller',
      connection: 'USB',
      batteryLevel: 100,
      charging: true,
      charged: false,
    };
    state.pressedControls = ['triangle'];
    await api.persistPreview(state);

    const loaded = await api.load();

    expect(loaded.preferences.reducedMotion).toBe(true);
    expect(loaded.device.connected).toBe(false);
    expect(loaded.device.name).toBe('DualSense Wireless Controller');
    expect(loaded.pressedControls).toEqual([]);
  });
});

describe('native action conversion', () => {
  it('normalizes a configured application action in both directions', () => {
    const native = actionToNative(
      mapping({
        configuration: {
          path: '  C:\\Tools\\Capture.exe  ',
          arguments: ['--minimized', 4, '--profile=stream'],
          workingDirectory: '   ',
        },
      }),
    );

    expect(native).toEqual({
      type: 'openApplication',
      path: 'C:\\Tools\\Capture.exe',
      arguments: ['--minimized', '--profile=stream'],
      workingDirectory: null,
    });
    expect(actionFromNative(native)).toEqual({
      actionId: 'launch-application',
      configuration: {
        path: 'C:\\Tools\\Capture.exe',
        arguments: ['--minimized', '--profile=stream'],
        workingDirectory: '',
      },
    });
  });

  it('preserves incomplete actions instead of creating unsafe native payloads', () => {
    const invalidWebsite = mapping({
      actionId: 'open-website',
      configuration: { url: 'javascript:alert(1)' },
    });
    const invalidShortcut = mapping({
      actionId: 'keyboard-shortcut',
      configuration: { shortcut: 'Ctrl + Ctrl + K' },
    });

    expect(actionToNative(invalidWebsite)).toEqual({
      type: 'incomplete',
      actionId: 'open-website',
      configuration: invalidWebsite.configuration,
    });
    expect(actionToNative(invalidShortcut)).toEqual({
      type: 'incomplete',
      actionId: 'keyboard-shortcut',
      configuration: invalidShortcut.configuration,
    });
  });

  it('converts keyboard modifiers and webhook values deterministically', () => {
    expect(
      actionToNative(
        mapping({
          actionId: 'keyboard-shortcut',
          configuration: { shortcut: 'Ctrl + Shift + K' },
        }),
      ),
    ).toEqual({
      type: 'hotkey',
      hotkey: { modifiers: ['control', 'shift'], key: 'K' },
    });

    expect(
      actionToNative(
        mapping({
          actionId: 'webhook',
          configuration: {
            url: 'https://example.com/hook',
            method: 'patch',
            headers: { Authorization: 'token', ignored: 4 },
            body: '',
            timeoutMs: 2500,
          },
        }),
      ),
    ).toEqual({
      type: 'webhook',
      request: {
        url: 'https://example.com/hook',
        method: 'PATCH',
        headers: { Authorization: 'token' },
        body: null,
        timeoutMs: 2500,
      },
    });
  });
});

describe('native binding and trigger conversion', () => {
  it('offers release as an editable trigger mode', () => {
    expect(triggerModes).toContain('Release');
  });

  it.each([
    ['Press', {}, { kind: 'press' }],
    ['Release', {}, { kind: 'release' }],
    ['Long press', { triggerDurationMs: 725 }, { kind: 'longPress', durationMs: 725 }],
    ['Double press', { doublePressIntervalMs: 280 }, { kind: 'doublePress', intervalMs: 280 }],
    [
      'Hold',
      { holdInitialDelayMs: 450, holdIntervalMs: 125 },
      { kind: 'holdRepeat', initialDelayMs: 450, intervalMs: 125 },
    ],
  ] as const)('converts %s triggers', (mode, configuration, expected) => {
    const native = triggerToNative(mode, configuration);

    expect(native).toEqual(expected);
    expect(triggerFromNative(native)).toBe(mode);
    expect(triggerConfiguration(native)).toEqual(configuration);
  });

  it('rejects trigger shapes the editor cannot represent', () => {
    expect(() => triggerFromNative({ kind: 'gesture' })).toThrow(
      'Trigger "gesture" cannot be represented',
    );
  });

  it('round-trips a controller binding without losing its control or trigger settings', () => {
    const source = mapping({
      actionId: 'volume-mute',
      title: 'Mute desktop audio',
      detail: 'Mute or unmute system volume',
      icon: 'volume-off',
      trigger: 'Hold',
      configuration: { holdInitialDelayMs: 500, holdIntervalMs: 175 },
      enabled: false,
    });
    const draft = bindingDraft(profileId, 'ps', source);

    expect(draft).toMatchObject({
      profileId,
      input: { kind: 'button', value: 'playstation' },
      trigger: { kind: 'holdRepeat', initialDelayMs: 500, intervalMs: 175 },
      action: { type: 'volume', command: 'mute' },
      label: 'Mute desktop audio',
      enabled: false,
    });

    const binding: NativeBinding = {
      id: bindingId,
      ...draft,
      createdAt: '2026-08-06T00:00:00.000Z',
      updatedAt: '2026-08-06T00:00:00.000Z',
    };
    const converted = mappingFromNative(binding);

    expect(converted).toMatchObject({
      id: bindingId,
      actionId: 'volume-mute',
      title: 'Mute desktop audio',
      trigger: 'Hold',
      configuration: { holdInitialDelayMs: 500, holdIntervalMs: 175 },
      enabled: false,
    });
  });

  it('round-trips a release binding without changing it to a press binding', () => {
    const source = mapping({
      actionId: 'media-play-pause',
      trigger: 'Release',
      configuration: {},
    });
    const draft = bindingDraft(profileId, 'triangle', source);
    const converted = mappingFromNative({
      id: bindingId,
      ...draft,
      createdAt: '2026-08-06T00:00:00.000Z',
      updatedAt: '2026-08-06T00:00:00.000Z',
    });

    expect(draft.trigger).toEqual({ kind: 'release' });
    expect(converted.trigger).toBe('Release');
  });

  it('fails visibly instead of hiding combination bindings', () => {
    const binding: NativeBinding = {
      id: bindingId,
      profileId,
      input: { kind: 'combination', value: ['triangle', 'leftBumper'] },
      trigger: { kind: 'press' },
      action: { type: 'media', command: 'playPause' },
      label: 'Scene shortcut',
      enabled: true,
      createdAt: '2026-08-06T00:00:00.000Z',
      updatedAt: '2026-08-06T00:00:00.000Z',
    };

    expect(() =>
      profileFromNative(
        {
          id: profileId,
          name: 'Default',
          description: null,
          automaticApp: null,
          sortOrder: 0,
          createdAt: '2026-08-06T00:00:00.000Z',
          updatedAt: '2026-08-06T00:00:00.000Z',
        },
        [binding],
      ),
    ).toThrow('Binding "Scene shortcut" cannot be loaded: button combinations');
  });

  it('rejects top-level delays after they leave the action library', () => {
    expect(() => actionFromNative({ type: 'delay', durationMs: 500 })).toThrow(
      'Top-level delay bindings cannot be represented',
    );
  });

  it('rejects stored action identifiers that have no editor', () => {
    expect(() =>
      actionFromNative({ type: 'incomplete', actionId: 'obs-scene', configuration: {} }),
    ).toThrow('Action "obs-scene" cannot be represented');
  });

  it('rejects multiple bindings for one control instead of overwriting one', () => {
    const binding: NativeBinding = {
      id: bindingId,
      profileId,
      input: { kind: 'button', value: 'triangle' },
      trigger: { kind: 'press' },
      action: { type: 'media', command: 'playPause' },
      label: 'First action',
      enabled: true,
      createdAt: '2026-08-06T00:00:00.000Z',
      updatedAt: '2026-08-06T00:00:00.000Z',
    };

    expect(() =>
      profileFromNative(
        {
          id: profileId,
          name: 'Default',
          description: null,
          automaticApp: null,
          sortOrder: 0,
          createdAt: '2026-08-06T00:00:00.000Z',
          updatedAt: '2026-08-06T00:00:00.000Z',
        },
        [
          binding,
          { ...binding, id: '449b9611-b1b7-423f-b0c7-3ef588dfe98a', label: 'Second action' },
        ],
      ),
    ).toThrow('more than one binding targets triangle');
  });
});

describe('controller status conversion', () => {
  it('preserves an unknown battery percentage independently from power state', () => {
    const converted = controllerFromNative({
      snapshot: {
        device: {
          name: 'DualSense Wireless Controller',
          connection: 'wired',
          battery: { state: 'charging', percentage: null },
        },
        pressedButtons: [],
        paused: false,
        ignoredDeviceCount: 0,
      },
      initializationError: null,
    });

    expect(converted.device).toMatchObject({
      batteryLevel: null,
      charging: true,
      charged: false,
    });
  });
});
