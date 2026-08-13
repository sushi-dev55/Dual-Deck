import { invoke, isTauri } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open } from '@tauri-apps/plugin-dialog';
import { actionCatalog } from '../data/catalog';
import type {
  ControlId,
  ControllerDevice,
  DeckState,
  MappedAction,
  Preferences,
  Profile,
} from '../models';

const storageKey = 'dual-deck.browser-preview.v1';
const profileAccents = ['#84a7ff', '#7fcdbb', '#d9a86c', '#b49af3', '#e78e9b', '#67b7d4'];

export type NativeAction =
  | { type: 'incomplete'; actionId: string; configuration: Record<string, unknown> }
  | { type: 'openApplication'; path: string; arguments: string[]; workingDirectory: string | null }
  | { type: 'openPath'; path: string }
  | { type: 'openUrl'; url: string }
  | { type: 'hotkey'; hotkey: { modifiers: NativeModifier[]; key: string } }
  | { type: 'typeText'; text: string }
  | { type: 'media'; command: 'playPause' | 'nextTrack' | 'previousTrack' | 'stop' }
  | { type: 'volume'; command: 'up' | 'down' | 'mute' }
  | { type: 'playSound'; path: string }
  | { type: 'webhook'; request: NativeWebhook }
  | { type: 'closeApplication'; executableName: string }
  | { type: 'switchProfile'; profileId: string }
  | { type: 'delay'; durationMs: number }
  | { type: 'multiAction'; steps: NativeActionStep[]; stopOnError: boolean };

export type NativeModifier = 'control' | 'alt' | 'shift' | 'meta';

interface NativeWebhook {
  url: string;
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  headers: Record<string, string>;
  body: string | null;
  timeoutMs: number;
}

interface NativeActionStep {
  action: NativeAction;
  delayAfterMs: number;
}

export interface NativeProfile {
  id: string;
  name: string;
  description: string | null;
  automaticApp: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface NativeBinding {
  id: string;
  profileId: string;
  input: { kind: string; value: unknown };
  trigger: Record<string, unknown> & { kind: string };
  action: NativeAction;
  label: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

interface NativeSettings {
  activeProfileId: string;
  launchAtStartup: boolean;
  startMinimized: boolean;
  minimizeToTray: boolean;
  closeToTray: boolean;
  mappingsPaused: boolean;
  checkForUpdates: boolean;
  automaticProfileSwitching: boolean;
  actionToasts: boolean;
  controllerFeedback: boolean;
  reducedMotion: boolean;
  updateChannel: 'stable' | 'preview';
}

interface NativeAppSnapshot {
  profiles: NativeProfile[];
  activeProfile: NativeProfile;
  bindings: NativeBinding[];
  settings: NativeSettings;
}

export interface NativeControllerDevice {
  name: string;
  connection: 'wired' | 'wireless' | 'unknown';
  battery: {
    state: 'charging' | 'charged' | 'discharging' | 'not_present' | 'unknown';
    percentage: number | null;
  };
}

export interface NativeControllerSnapshot {
  device: NativeControllerDevice | null;
  pressedButtons: string[];
  paused: boolean;
  ignoredDeviceCount: number;
}

export interface NativeControllerStatus {
  snapshot: NativeControllerSnapshot | null;
  initializationError: string | null;
}

interface NativeControllerEvent {
  type: string;
  button?: string;
  state?: 'pressed' | 'released';
}

interface NativeActionEvent {
  bindingId: string;
  code: string | null;
  message: string | null;
}

export interface NativeDrop {
  paths: string[];
  x: number;
  y: number;
}

export interface DeckEventHandlers {
  stateChanged: (reason: string) => void;
  controllerChanged: (
    device: ControllerDevice,
    paused: boolean | null,
    pressedControls: ControlId[],
  ) => void;
  actionCompleted: (bindingId: string) => void;
  actionFailed: (bindingId: string, message: string) => void;
  filesDropped: (drop: NativeDrop) => void;
}

export interface DeckApi {
  readonly native: boolean;
  load(): Promise<DeckState>;
  persistPreview(state: DeckState): Promise<void>;
  saveMapping(
    profileId: string,
    controlId: ControlId,
    mapping: MappedAction,
  ): Promise<MappedAction>;
  deleteMapping(id: string): Promise<void>;
  createProfile(name: string): Promise<Profile>;
  duplicateProfile(id: string, source: Profile): Promise<Profile>;
  deleteProfile(id: string): Promise<void>;
  saveProfile(profile: Profile): Promise<Profile>;
  activateProfile(id: string): Promise<void>;
  savePreferences(
    activeProfileId: string,
    preferences: Preferences,
    mappingsPaused: boolean,
  ): Promise<void>;
  setMappingsPaused(paused: boolean): Promise<void>;
  runAction(bindingId: string): Promise<void>;
  choosePath(actionId: string): Promise<string | null>;
  controllerStatus(): Promise<{
    device: ControllerDevice;
    paused: boolean | null;
    pressedControls: ControlId[];
  }>;
  subscribe(handlers: DeckEventHandlers): Promise<UnlistenFn>;
}

const defaultPreferences = (): Preferences => ({
  startWithWindows: true,
  launchMinimized: true,
  minimizeToTray: true,
  closeToTray: true,
  automaticProfiles: false,
  actionToasts: true,
  controllerFeedback: false,
  reducedMotion: false,
  updateChannel: 'Stable',
  checkForUpdates: false,
});

const disconnectedDevice = (initializationError?: string): ControllerDevice => ({
  connected: false,
  name: 'DualSense Wireless Controller',
  connection: 'Unknown',
  batteryLevel: null,
  charging: false,
  charged: false,
  initializationError,
  ignoredDeviceCount: 0,
});

const starterProfile = (): Profile => ({
  id: crypto.randomUUID(),
  name: 'Default',
  accent: profileAccents[0],
  applicationRule: '',
  mappings: {},
  createdAt: Date.now(),
});

export const createInitialState = (): DeckState => {
  const profile = starterProfile();
  return {
    appVersion: '0.1.0',
    profiles: [profile],
    activeProfileId: profile.id,
    device: disconnectedDevice(),
    preferences: defaultPreferences(),
    mappingsPaused: false,
    pressedControls: [],
  };
};

export class BrowserPreviewDeckApi implements DeckApi {
  readonly native = false;

  async load(): Promise<DeckState> {
    try {
      const stored = localStorage.getItem(storageKey);
      if (!stored) return createInitialState();
      const parsed = JSON.parse(stored) as DeckState;
      if (!parsed.profiles?.length) return createInitialState();
      const initial = createInitialState();
      return {
        ...initial,
        ...parsed,
        device: initial.device,
        preferences: { ...initial.preferences, ...parsed.preferences },
        mappingsPaused: parsed.mappingsPaused ?? false,
        pressedControls: [],
      };
    } catch {
      return createInitialState();
    }
  }

  async persistPreview(state: DeckState): Promise<void> {
    localStorage.setItem(storageKey, JSON.stringify(state));
  }

  async saveMapping(
    _profileId: string,
    _controlId: ControlId,
    mapping: MappedAction,
  ): Promise<MappedAction> {
    return mapping;
  }

  async deleteMapping(): Promise<void> {}

  async createProfile(name: string): Promise<Profile> {
    const id = crypto.randomUUID();
    return {
      id,
      name,
      description: undefined,
      accent: accentFor(id),
      applicationRule: '',
      mappings: {},
      createdAt: Date.now(),
    };
  }

  async duplicateProfile(_id: string, source: Profile): Promise<Profile> {
    const id = crypto.randomUUID();
    const mappings = Object.fromEntries(
      Object.entries(source.mappings).map(([control, mapping]) => [
        control,
        mapping ? { ...structuredClone(mapping), id: crypto.randomUUID() } : mapping,
      ]),
    );
    return {
      ...structuredClone(source),
      id,
      name: `${source.name} copy`,
      accent: accentFor(id),
      applicationRule: '',
      mappings,
      createdAt: Date.now(),
    };
  }

  async deleteProfile(): Promise<void> {}

  async saveProfile(profile: Profile): Promise<Profile> {
    return profile;
  }

  async activateProfile(): Promise<void> {}

  async savePreferences(): Promise<void> {}

  async setMappingsPaused(): Promise<void> {}

  async runAction(): Promise<void> {
    throw new Error('Actions run only in the Dual Deck desktop application');
  }

  async choosePath(): Promise<string | null> {
    return null;
  }

  async controllerStatus() {
    return { device: disconnectedDevice(), paused: null, pressedControls: [] };
  }

  async subscribe(): Promise<UnlistenFn> {
    return () => {};
  }
}

class NativeDeckApi implements DeckApi {
  readonly native = true;

  async load(): Promise<DeckState> {
    const [snapshot, controller, appVersion] = await Promise.all([
      invoke<NativeAppSnapshot>('get_app_snapshot'),
      this.controllerStatus(),
      getVersion(),
    ]);
    const profiles = snapshot.profiles.map((profile) =>
      profileFromNative(
        profile,
        snapshot.bindings.filter((binding) => binding.profileId === profile.id),
      ),
    );
    return {
      appVersion,
      profiles,
      activeProfileId: snapshot.settings.activeProfileId,
      preferences: preferencesFromNative(snapshot.settings),
      mappingsPaused: snapshot.settings.mappingsPaused,
      device: controller.device,
      pressedControls: controller.pressedControls,
    };
  }

  async persistPreview(): Promise<void> {}

  async saveMapping(
    profileId: string,
    controlId: ControlId,
    mapping: MappedAction,
  ): Promise<MappedAction> {
    const binding = await invoke<NativeBinding>('upsert_binding', {
      id: mapping.id,
      draft: bindingDraft(profileId, controlId, mapping),
    });
    return mappingFromNative(binding);
  }

  async deleteMapping(id: string): Promise<void> {
    await invoke('delete_binding', { id });
  }

  async createProfile(name: string): Promise<Profile> {
    const profile = await invoke<NativeProfile>('create_profile', {
      draft: { name, description: null, automaticApp: null },
    });
    return profileFromNative(profile, []);
  }

  async duplicateProfile(id: string, _source: Profile): Promise<Profile> {
    const profile = await invoke<NativeProfile>('duplicate_profile', { id });
    const bindings = await invoke<NativeBinding[]>('list_bindings', { profileId: profile.id });
    return profileFromNative(profile, bindings);
  }

  async deleteProfile(id: string): Promise<void> {
    await invoke('delete_profile', { id });
  }

  async saveProfile(profile: Profile): Promise<Profile> {
    const updated = await invoke<NativeProfile>('update_profile', {
      id: profile.id,
      draft: {
        name: profile.name,
        description: profile.description ?? null,
        automaticApp: profile.applicationRule || null,
      },
    });
    return {
      ...profileFromNative(updated, []),
      mappings: profile.mappings,
      accent: profile.accent,
    };
  }

  async activateProfile(id: string): Promise<void> {
    await invoke('set_active_profile', { id });
  }

  async savePreferences(
    activeProfileId: string,
    preferences: Preferences,
    mappingsPaused: boolean,
  ): Promise<void> {
    await invoke('update_settings', {
      settings: preferencesToNative(activeProfileId, preferences, mappingsPaused),
    });
  }

  async setMappingsPaused(paused: boolean): Promise<void> {
    await invoke('set_mappings_paused', { paused });
  }

  async runAction(bindingId: string): Promise<void> {
    await invoke('execute_binding', { id: bindingId });
  }

  async choosePath(actionId: string): Promise<string | null> {
    const selected = await open(dialogOptions(actionId));
    return typeof selected === 'string' ? selected : null;
  }

  async controllerStatus() {
    const status = await invoke<NativeControllerStatus>('get_controller_status');
    return controllerFromNative(status);
  }

  async subscribe(handlers: DeckEventHandlers): Promise<UnlistenFn> {
    const view = getCurrentWebview();
    let active = true;
    let controllerRefreshRunning = false;
    let controllerRefreshQueued = false;
    const refreshControllerEvent = () => {
      controllerRefreshQueued = true;
      if (controllerRefreshRunning) return;
      controllerRefreshRunning = true;
      void (async () => {
        try {
          while (active && controllerRefreshQueued) {
            controllerRefreshQueued = false;
            try {
              const current = await this.controllerStatus();
              if (active) {
                handlers.controllerChanged(current.device, current.paused, current.pressedControls);
              }
            } catch {
              if (active) handlers.stateChanged('controller_status_unavailable');
            }
          }
        } finally {
          controllerRefreshRunning = false;
          if (active && controllerRefreshQueued) refreshControllerEvent();
        }
      })().catch(() => undefined);
    };
    const unlisteners = await Promise.all([
      view.listen<{ reason: string }>('state-changed', ({ payload }) =>
        handlers.stateChanged(payload.reason),
      ),
      view.listen<NativeControllerEvent>('controller-event', ({ payload }) => {
        if (payload.type === 'button_changed') {
          const control = rawButtonToControl[payload.button ?? ''];
          if (control) refreshControllerEvent();
          return;
        }
        if (
          [
            'connected',
            'disconnected',
            'device_updated',
            'paused_changed',
            'additional_devices_ignored',
            'events_dropped',
            'backend_error',
          ].includes(payload.type)
        ) {
          refreshControllerEvent();
        }
      }),
      view.listen<NativeActionEvent>('action-completed', ({ payload }) =>
        handlers.actionCompleted(payload.bindingId),
      ),
      view.listen<NativeActionEvent>('action-failed', ({ payload }) =>
        handlers.actionFailed(
          payload.bindingId,
          payload.message ?? 'The action could not be completed',
        ),
      ),
      view.onDragDropEvent(({ payload }) => {
        if (payload.type !== 'drop') return;
        handlers.filesDropped({
          paths: payload.paths,
          x: payload.position.x / window.devicePixelRatio,
          y: payload.position.y / window.devicePixelRatio,
        });
      }),
    ]);
    return () => {
      active = false;
      controllerRefreshQueued = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }
}

export function profileFromNative(profile: NativeProfile, bindings: NativeBinding[]): Profile {
  const mappings: Partial<Record<ControlId, MappedAction>> = {};
  for (const binding of bindings) {
    const control = inputToControl(binding.input);
    if (!control) {
      throw new BindingConversionError(binding, inputConversionFailure(binding.input));
    }
    if (mappings[control]) {
      throw new BindingConversionError(binding, `more than one binding targets ${control}`);
    }
    mappings[control] = mappingFromNative(binding);
  }
  return {
    id: profile.id,
    name: profile.name,
    description: profile.description ?? undefined,
    accent: accentFor(profile.id),
    applicationRule: profile.automaticApp ?? '',
    mappings,
    createdAt: Date.parse(profile.createdAt),
  };
}

export function mappingFromNative(binding: NativeBinding): MappedAction {
  try {
    const converted = actionFromNative(binding.action);
    const catalog = actionCatalog.find((action) => action.id === converted.actionId);
    return {
      id: binding.id,
      actionId: converted.actionId,
      title: binding.label || catalog?.title || 'Action',
      detail: actionDetail(
        converted.actionId,
        converted.configuration,
        catalog?.defaultDetail ?? 'Configured action',
      ),
      icon: catalog?.icon ?? 'app',
      accent: catalog?.accent ?? '#84a7ff',
      trigger: triggerFromNative(binding.trigger),
      configuration: { ...converted.configuration, ...triggerConfiguration(binding.trigger) },
      enabled: binding.enabled,
    };
  } catch (error) {
    if (error instanceof BindingConversionError) throw error;
    throw new BindingConversionError(binding, errorMessage(error));
  }
}

export function bindingDraft(profileId: string, controlId: ControlId, mapping: MappedAction) {
  return {
    profileId,
    input: { kind: 'button', value: controlToNativeButton[controlId] },
    trigger: triggerToNative(mapping.trigger, mapping.configuration),
    action: actionToNative(mapping),
    label:
      mapping.title.trim() ||
      actionCatalog.find((action) => action.id === mapping.actionId)?.title ||
      'Action',
    enabled: mapping.enabled,
  };
}

export function actionToNative(mapping: MappedAction): NativeAction {
  const configuration = mapping.configuration;
  const path = stringValue(configuration.path);
  const url = stringValue(configuration.url);
  switch (mapping.actionId) {
    case 'launch-application':
      return path
        ? {
            type: 'openApplication',
            path,
            arguments: stringArray(configuration.arguments),
            workingDirectory: optionalString(configuration.workingDirectory),
          }
        : incomplete(mapping);
    case 'open-file':
    case 'open-folder':
      return path ? { type: 'openPath', path } : incomplete(mapping);
    case 'open-website':
      return validHttpUrl(url) ? { type: 'openUrl', url } : incomplete(mapping);
    case 'keyboard-shortcut': {
      const hotkey = parseShortcut(stringValue(configuration.shortcut));
      return hotkey ? { type: 'hotkey', hotkey } : incomplete(mapping);
    }
    case 'type-text': {
      const text = stringValue(configuration.text);
      return text ? { type: 'typeText', text } : incomplete(mapping);
    }
    case 'media-play-pause':
      return { type: 'media', command: 'playPause' };
    case 'media-next':
      return { type: 'media', command: 'nextTrack' };
    case 'media-previous':
      return { type: 'media', command: 'previousTrack' };
    case 'media-stop':
      return { type: 'media', command: 'stop' };
    case 'volume-up':
      return { type: 'volume', command: 'up' };
    case 'volume-down':
      return { type: 'volume', command: 'down' };
    case 'volume-mute':
      return { type: 'volume', command: 'mute' };
    case 'soundboard':
      return path ? { type: 'playSound', path } : incomplete(mapping);
    case 'webhook':
      return validHttpUrl(url)
        ? {
            type: 'webhook',
            request: {
              url,
              method: webhookMethod(configuration.method),
              headers: objectStrings(configuration.headers),
              body: optionalString(configuration.body),
              timeoutMs: numberValue(configuration.timeoutMs, 10_000),
            },
          }
        : incomplete(mapping);
    case 'close-application': {
      const executableName = stringValue(configuration.executableName);
      return executableName ? { type: 'closeApplication', executableName } : incomplete(mapping);
    }
    case 'switch-profile': {
      const profileId = stringValue(configuration.profileId);
      return isUuid(profileId) ? { type: 'switchProfile', profileId } : incomplete(mapping);
    }
    case 'multi-action': {
      const steps = Array.isArray(configuration.steps)
        ? (configuration.steps as NativeActionStep[])
        : [];
      return steps.length
        ? { type: 'multiAction', steps, stopOnError: configuration.stopOnError !== false }
        : incomplete(mapping);
    }
    default:
      throw new Error(`Action "${mapping.actionId}" cannot be edited by this version of Dual Deck`);
  }
}

export function actionFromNative(action: NativeAction): {
  actionId: string;
  configuration: Record<string, unknown>;
} {
  switch (action.type) {
    case 'incomplete': {
      if (!actionCatalog.some((candidate) => candidate.id === action.actionId)) {
        throw new Error(
          `Action "${action.actionId}" cannot be represented by this version of Dual Deck`,
        );
      }
      return { actionId: action.actionId, configuration: action.configuration };
    }
    case 'openApplication':
      return {
        actionId: 'launch-application',
        configuration: {
          path: action.path,
          arguments: action.arguments,
          workingDirectory: action.workingDirectory ?? '',
        },
      };
    case 'openPath':
      return {
        actionId: pathLooksLikeFolder(action.path) ? 'open-folder' : 'open-file',
        configuration: { path: action.path },
      };
    case 'openUrl':
      return { actionId: 'open-website', configuration: { url: action.url } };
    case 'hotkey':
      return {
        actionId: 'keyboard-shortcut',
        configuration: { shortcut: shortcutFromNative(action.hotkey) },
      };
    case 'typeText':
      return { actionId: 'type-text', configuration: { text: action.text } };
    case 'media':
      return {
        actionId:
          action.command === 'playPause'
            ? 'media-play-pause'
            : action.command === 'nextTrack'
              ? 'media-next'
              : action.command === 'previousTrack'
                ? 'media-previous'
                : 'media-stop',
        configuration: {},
      };
    case 'volume':
      return { actionId: `volume-${action.command}` as 'volume-up', configuration: {} };
    case 'playSound':
      return { actionId: 'soundboard', configuration: { path: action.path } };
    case 'webhook':
      return {
        actionId: 'webhook',
        configuration: {
          url: action.request.url,
          method: action.request.method,
          headers: action.request.headers,
          body: action.request.body ?? '',
          timeoutMs: action.request.timeoutMs,
        },
      };
    case 'closeApplication':
      return {
        actionId: 'close-application',
        configuration: { executableName: action.executableName },
      };
    case 'switchProfile':
      return { actionId: 'switch-profile', configuration: { profileId: action.profileId } };
    case 'delay':
      throw new Error('Top-level delay bindings cannot be represented in the action editor');
    case 'multiAction':
      return {
        actionId: 'multi-action',
        configuration: { steps: action.steps, stopOnError: action.stopOnError },
      };
    default:
      throw new Error(
        `Native action type "${String((action as { type?: unknown }).type)}" cannot be represented in the action editor`,
      );
  }
}

function incomplete(mapping: MappedAction): NativeAction {
  return { type: 'incomplete', actionId: mapping.actionId, configuration: mapping.configuration };
}

function preferencesFromNative(settings: NativeSettings): Preferences {
  return {
    startWithWindows: settings.launchAtStartup,
    launchMinimized: settings.startMinimized,
    minimizeToTray: settings.minimizeToTray,
    closeToTray: settings.closeToTray,
    automaticProfiles: settings.automaticProfileSwitching,
    actionToasts: settings.actionToasts,
    controllerFeedback: settings.controllerFeedback,
    reducedMotion: settings.reducedMotion,
    updateChannel: settings.updateChannel === 'preview' ? 'Preview' : 'Stable',
    checkForUpdates: settings.checkForUpdates,
  };
}

function preferencesToNative(
  activeProfileId: string,
  preferences: Preferences,
  mappingsPaused: boolean,
): NativeSettings {
  return {
    activeProfileId,
    launchAtStartup: preferences.startWithWindows,
    startMinimized: preferences.launchMinimized,
    minimizeToTray: preferences.minimizeToTray,
    closeToTray: preferences.closeToTray,
    mappingsPaused,
    checkForUpdates: preferences.checkForUpdates,
    automaticProfileSwitching: preferences.automaticProfiles,
    actionToasts: preferences.actionToasts,
    controllerFeedback: preferences.controllerFeedback,
    reducedMotion: preferences.reducedMotion,
    updateChannel: preferences.updateChannel === 'Preview' ? 'preview' : 'stable',
  };
}

export function controllerFromNative(status: NativeControllerStatus) {
  const snapshot = status.snapshot;
  if (!snapshot?.device) {
    return {
      device: disconnectedDevice(status.initializationError ?? undefined),
      paused: snapshot?.paused ?? null,
      pressedControls: [] as ControlId[],
    };
  }
  const device: ControllerDevice = {
    connected: true,
    name: snapshot.device.name,
    connection:
      snapshot.device.connection === 'wired'
        ? 'USB'
        : snapshot.device.connection === 'wireless'
          ? 'Bluetooth'
          : 'Unknown',
    batteryLevel: snapshot.device.battery.percentage,
    charging: snapshot.device.battery.state === 'charging',
    charged: snapshot.device.battery.state === 'charged',
    initializationError: status.initializationError ?? undefined,
    ignoredDeviceCount: snapshot.ignoredDeviceCount,
  };
  return {
    device,
    paused: snapshot.paused,
    pressedControls: snapshot.pressedButtons
      .map((button) => rawButtonToControl[button])
      .filter((control): control is ControlId => Boolean(control)),
  };
}

export function triggerToNative(
  trigger: MappedAction['trigger'],
  configuration: Record<string, unknown>,
) {
  switch (trigger) {
    case 'Release':
      return { kind: 'release' };
    case 'Long press':
      return { kind: 'longPress', durationMs: numberValue(configuration.triggerDurationMs, 600) };
    case 'Double press':
      return {
        kind: 'doublePress',
        intervalMs: numberValue(configuration.doublePressIntervalMs, 350),
      };
    case 'Hold':
      return {
        kind: 'holdRepeat',
        initialDelayMs: numberValue(configuration.holdInitialDelayMs, 400),
        intervalMs: numberValue(configuration.holdIntervalMs, 150),
      };
    default:
      return { kind: 'press' };
  }
}

export function triggerFromNative(trigger: NativeBinding['trigger']): MappedAction['trigger'] {
  switch (trigger.kind) {
    case 'press':
      return 'Press';
    case 'release':
      return 'Release';
    case 'longPress':
      return 'Long press';
    case 'doublePress':
      return 'Double press';
    case 'holdRepeat':
      return 'Hold';
    default:
      throw new Error(
        `Trigger "${String(trigger.kind)}" cannot be represented in the action editor`,
      );
  }
}

export function triggerConfiguration(trigger: NativeBinding['trigger']): Record<string, unknown> {
  switch (trigger.kind) {
    case 'press':
    case 'release':
      return {};
    case 'longPress':
      return { triggerDurationMs: trigger.durationMs };
    case 'doublePress':
      return { doublePressIntervalMs: trigger.intervalMs };
    case 'holdRepeat':
      return {
        holdInitialDelayMs: trigger.initialDelayMs,
        holdIntervalMs: trigger.intervalMs,
      };
    default:
      throw new Error(
        `Trigger "${String(trigger.kind)}" cannot be represented in the action editor`,
      );
  }
}

function inputToControl(input: NativeBinding['input']): ControlId | null {
  if (input.kind !== 'button' || typeof input.value !== 'string') return null;
  return nativeButtonToControl[input.value] ?? null;
}

function inputConversionFailure(input: NativeBinding['input']): string {
  if (input.kind === 'combination') return 'button combinations require a combination editor';
  if (input.kind !== 'button') return `controller input kind "${input.kind}" is not editable`;
  if (typeof input.value !== 'string') return 'the controller button value is invalid';
  return `controller button "${input.value}" is not recognized`;
}

export class BindingConversionError extends Error {
  constructor(binding: Pick<NativeBinding, 'id' | 'label'>, reason: string) {
    const name = binding.label.trim() || binding.id;
    super(`Binding "${name}" cannot be loaded: ${reason}`);
    this.name = 'BindingConversionError';
  }
}

const controlToNativeButton: Record<ControlId, string> = {
  l2: 'leftTrigger',
  l1: 'leftBumper',
  r2: 'rightTrigger',
  r1: 'rightBumper',
  create: 'create',
  options: 'options',
  touchpad: 'touchpad',
  'dpad-up': 'dpadUp',
  'dpad-right': 'dpadRight',
  'dpad-down': 'dpadDown',
  'dpad-left': 'dpadLeft',
  triangle: 'triangle',
  circle: 'circle',
  cross: 'cross',
  square: 'square',
  l3: 'leftStick',
  r3: 'rightStick',
  ps: 'playstation',
  mute: 'mute',
};

const nativeButtonToControl = Object.fromEntries(
  Object.entries(controlToNativeButton).map(([control, button]) => [button, control]),
) as Record<string, ControlId>;

const rawButtonToControl: Record<string, ControlId> = {
  triangle: 'triangle',
  circle: 'circle',
  cross: 'cross',
  square: 'square',
  create: 'create',
  play_station: 'ps',
  options: 'options',
  l3: 'l3',
  r3: 'r3',
  l1: 'l1',
  r1: 'r1',
  l2: 'l2',
  r2: 'r2',
  d_pad_up: 'dpad-up',
  d_pad_down: 'dpad-down',
  d_pad_left: 'dpad-left',
  d_pad_right: 'dpad-right',
  microphone: 'mute',
  touchpad: 'touchpad',
};

function actionDetail(
  actionId: string,
  configuration: Record<string, unknown>,
  fallback: string,
): string {
  const path = stringValue(configuration.path);
  const url = stringValue(configuration.url);
  if (path) return path;
  if (url) return url;
  if (actionId === 'keyboard-shortcut') return stringValue(configuration.shortcut) || fallback;
  if (actionId === 'type-text') return stringValue(configuration.text) || fallback;
  if (actionId === 'switch-profile') return stringValue(configuration.profileId) || fallback;
  return fallback;
}

function dialogOptions(actionId: string) {
  if (actionId === 'open-folder')
    return { title: 'Choose a folder', directory: true, multiple: false };
  if (actionId === 'launch-application') {
    return {
      title: 'Choose an application',
      multiple: false,
      directory: false,
      filters: [{ name: 'Windows applications', extensions: ['exe'] }],
    };
  }
  if (actionId === 'soundboard') {
    return {
      title: 'Choose an audio file',
      multiple: false,
      directory: false,
      filters: [{ name: 'Wave audio', extensions: ['wav'] }],
    };
  }
  return { title: 'Choose a file', multiple: false, directory: false };
}

function parseShortcut(value: string): { modifiers: NativeModifier[]; key: string } | null {
  const parts = value
    .split('+')
    .map((part) => part.trim())
    .filter(Boolean);
  if (!parts.length) return null;
  const key = parts.pop();
  if (!key) return null;
  const modifiers: NativeModifier[] = [];
  for (const part of parts) {
    const normalized = part.toLowerCase();
    const modifier =
      normalized === 'ctrl' || normalized === 'control'
        ? 'control'
        : normalized === 'alt'
          ? 'alt'
          : normalized === 'shift'
            ? 'shift'
            : ['win', 'windows', 'meta'].includes(normalized)
              ? 'meta'
              : null;
    if (!modifier || modifiers.includes(modifier)) return null;
    modifiers.push(modifier);
  }
  return { modifiers, key };
}

function shortcutFromNative(hotkey: { modifiers: NativeModifier[]; key: string }): string {
  const names: Record<NativeModifier, string> = {
    control: 'Ctrl',
    alt: 'Alt',
    shift: 'Shift',
    meta: 'Win',
  };
  return [...hotkey.modifiers.map((modifier) => names[modifier]), hotkey.key].join(' + ');
}

function webhookMethod(value: unknown): NativeWebhook['method'] {
  return ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].includes(String(value).toUpperCase())
    ? (String(value).toUpperCase() as NativeWebhook['method'])
    : 'POST';
}

function pathLooksLikeFolder(path: string): boolean {
  const name = path.split(/[\\/]/).pop() ?? '';
  return !name.includes('.');
}

function accentFor(id: string): string {
  let hash = 0;
  for (const character of id) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  return profileAccents[hash % profileAccents.length];
}

function stringValue(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function optionalString(value: unknown): string | null {
  const converted = stringValue(value);
  return converted || null;
}

function numberValue(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function objectStrings(value: unknown): Record<string, string> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value).filter(
      (entry): entry is [string, string] => typeof entry[1] === 'string',
    ),
  );
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : [];
}

function validHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return ['http:', 'https:'].includes(url.protocol) && Boolean(url.host);
  } catch {
    return false;
  }
}

function isUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);
}

export function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object') {
    if ('message' in error && typeof error.message === 'string') return error.message;
    if ('error' in error && typeof error.error === 'string') return error.error;
  }
  return 'The operation could not be completed';
}

export const deckApi: DeckApi = isTauri() ? new NativeDeckApi() : new BrowserPreviewDeckApi();
