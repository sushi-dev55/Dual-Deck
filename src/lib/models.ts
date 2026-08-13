export type WorkspaceView = 'editor' | 'profiles' | 'settings';

export type ControlId =
  | 'l2'
  | 'l1'
  | 'r2'
  | 'r1'
  | 'create'
  | 'options'
  | 'touchpad'
  | 'dpad-up'
  | 'dpad-right'
  | 'dpad-down'
  | 'dpad-left'
  | 'triangle'
  | 'circle'
  | 'cross'
  | 'square'
  | 'l3'
  | 'r3'
  | 'ps'
  | 'mute';

export type ActionCategory = 'Launch' | 'Keyboard' | 'Media' | 'Workflow' | 'Streaming';

export type ActionIcon =
  | 'app'
  | 'file'
  | 'folder'
  | 'globe'
  | 'keyboard'
  | 'text'
  | 'play'
  | 'next'
  | 'previous'
  | 'volume-up'
  | 'volume-down'
  | 'volume-off'
  | 'layers'
  | 'timer'
  | 'webhook'
  | 'profile'
  | 'audio'
  | 'scene'
  | 'broadcast'
  | 'record';

export interface ActionDefinition {
  id: string;
  title: string;
  description: string;
  category: ActionCategory;
  icon: ActionIcon;
  accent: string;
  defaultDetail: string;
}

export const triggerModes = ['Press', 'Release', 'Long press', 'Double press', 'Hold'] as const;

export type TriggerMode = (typeof triggerModes)[number];

export interface MappedAction {
  id: string;
  actionId: string;
  title: string;
  detail: string;
  icon: ActionIcon;
  accent: string;
  trigger: TriggerMode;
  configuration: Record<string, unknown>;
  enabled: boolean;
}

export interface Profile {
  id: string;
  name: string;
  description?: string;
  accent: string;
  applicationRule: string;
  mappings: Partial<Record<ControlId, MappedAction>>;
  createdAt: number;
}

export interface ControllerDevice {
  connected: boolean;
  name: string;
  connection: 'USB' | 'Bluetooth' | 'Unknown';
  batteryLevel: number | null;
  charging: boolean;
  charged: boolean;
  initializationError?: string;
  ignoredDeviceCount?: number;
}

export interface Preferences {
  startWithWindows: boolean;
  launchMinimized: boolean;
  minimizeToTray: boolean;
  closeToTray: boolean;
  automaticProfiles: boolean;
  actionToasts: boolean;
  controllerFeedback: boolean;
  reducedMotion: boolean;
  updateChannel: 'Stable' | 'Preview';
  checkForUpdates: boolean;
}

export interface DeckState {
  appVersion: string;
  profiles: Profile[];
  activeProfileId: string;
  device: ControllerDevice;
  preferences: Preferences;
  mappingsPaused: boolean;
  pressedControls: ControlId[];
}

export interface ToastMessage {
  id: number;
  title: string;
  detail?: string;
  tone?: 'neutral' | 'success' | 'warning';
}
