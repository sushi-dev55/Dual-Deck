<script lang="ts">
  import { TriangleAlert, X } from '@lucide/svelte';
  import { onDestroy, onMount, tick } from 'svelte';
  import { actionCatalog, controlNames } from '../data/catalog';
  import type {
    ActionDefinition,
    ControlId,
    DeckState,
    MappedAction,
    Preferences,
    Profile,
    ToastMessage,
    WorkspaceView,
  } from '../models';
  import { createInitialState, deckApi, errorMessage } from '../services/deckApi';
  import ActionLibrary from './ActionLibrary.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import ControllerCanvas from './ControllerCanvas.svelte';
  import Inspector from './Inspector.svelte';
  import ProfilesView from './ProfilesView.svelte';
  import SettingsView from './SettingsView.svelte';
  import ToastHost from './ToastHost.svelte';
  import TopBar from './TopBar.svelte';

  type DeletionRequest =
    | {
        kind: 'mapping';
        profileId: string;
        control: ControlId;
        mappingId: string;
        title: string;
      }
    | { kind: 'profile'; profileId: string; title: string };

  let deck = $state<DeckState>(createInitialState());
  let view = $state<WorkspaceView>('editor');
  let selectedControl = $state<ControlId>('triangle');
  let toasts = $state<ToastMessage[]>([]);
  let deletionRequest = $state<DeletionRequest | null>(null);
  let workspaceError = $state<string | null>(null);
  let toastId = 0;
  let unlisten: (() => void) | undefined;
  let settingsTimer: number | undefined;
  let refreshTimer: number | undefined;
  let refreshInFlight = false;
  let refreshRequested = false;
  let refreshVersion = 0;
  let localStateVersion = 0;
  let pendingNativeMutations = 0;
  let preferencesRevision = 0;
  let activationRevision = 0;
  let pauseRevision = 0;
  let controllerRefreshRevision = 0;
  let settingsDirty = false;
  let activationDirty = false;
  let pauseDirty = false;
  const mappingTimers = new Map<string, number>();
  const profileTimers = new Map<string, number>();
  const mappingRevisions = new Map<string, number>();
  const profileRevisions = new Map<string, number>();
  const dirtyMappings = new Set<string>();
  const dirtyProfiles = new Set<string>();
  const mappingOperationChains = new Map<string, Promise<void>>();
  const profileOperationChains = new Map<string, Promise<void>>();
  let settingsOperationChain: Promise<void> = Promise.resolve();
  let activeProfile = $derived(
    deck.profiles.find((profile) => profile.id === deck.activeProfileId) ?? deck.profiles[0]!,
  );
  let selectedMapping = $derived(activeProfile.mappings[selectedControl]);
  let deletionDialog = $derived(
    deletionRequest?.kind === 'mapping'
      ? {
          title: `Remove ${deletionRequest.title}?`,
          description: `This action will be removed from ${controlNames[deletionRequest.control]} in the current profile.`,
          confirmLabel: 'Remove mapping',
        }
      : deletionRequest?.kind === 'profile'
        ? {
            title: `Delete ${deletionRequest.title}?`,
            description:
              'The profile and all of its controller mappings will be permanently deleted.',
            confirmLabel: 'Delete profile',
          }
        : null,
  );

  onMount(() => {
    let mounted = true;
    void initialize(() => mounted);
    return () => {
      mounted = false;
      unlisten?.();
    };
  });

  onDestroy(() => {
    if (settingsTimer) window.clearTimeout(settingsTimer);
    if (refreshTimer) window.clearTimeout(refreshTimer);
    mappingTimers.forEach((timer) => window.clearTimeout(timer));
    profileTimers.forEach((timer) => window.clearTimeout(timer));
  });

  async function initialize(isMounted: () => boolean) {
    try {
      const startedAtLocalVersion = localStateVersion;
      const loaded = await deckApi.load();
      if (!isMounted()) return;
      workspaceError = null;
      if (startedAtLocalVersion === localStateVersion && !hasPendingLocalWrites()) deck = loaded;
      else requestNativeRefresh();
      if (loaded.device.initializationError) {
        notify('Controller service unavailable', loaded.device.initializationError, 'warning');
      }
      unlisten = await deckApi.subscribe({
        stateChanged: () => {
          requestNativeRefresh();
        },
        controllerChanged: (device, paused, pressedControls) => {
          controllerRefreshRevision += 1;
          deck.device = device;
          if (paused !== null && !pauseDirty) deck.mappingsPaused = paused;
          deck.pressedControls = pressedControls;
          touchLocalState();
        },
        actionCompleted: (bindingId) => {
          if (!deck.preferences.actionToasts) return;
          notify('Action completed', findMapping(bindingId)?.title, 'success');
        },
        actionFailed: (bindingId, message) => {
          notify(
            'Action failed',
            `${findMapping(bindingId)?.title ?? 'Mapping'}: ${message}`,
            'warning',
          );
        },
        filesDropped: ({ paths, x, y }) => {
          const target = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-control]');
          const control = target?.dataset.control as ControlId | undefined;
          if (!control) {
            notify('Drop onto a controller control', 'The file was not assigned.', 'warning');
            return;
          }
          void assignPaths(control, paths);
        },
      });
      requestNativeRefresh();
    } catch (error) {
      const message = errorMessage(error);
      workspaceError = message;
      notify('Dual Deck could not load', message, 'warning');
    }
  }

  function notify(title: string, detail?: string, tone: ToastMessage['tone'] = 'neutral') {
    const id = ++toastId;
    toasts = [...toasts, { id, title, detail, tone }].slice(-4);
    window.setTimeout(() => dismissToast(id), 3600);
  }

  function dismissToast(id: number) {
    toasts = toasts.filter((toast) => toast.id !== id);
  }

  function touchLocalState() {
    localStateVersion += 1;
  }

  function hasPendingLocalWrites() {
    return (
      dirtyMappings.size > 0 ||
      dirtyProfiles.size > 0 ||
      settingsDirty ||
      activationDirty ||
      pauseDirty
    );
  }

  function canRefresh() {
    return pendingNativeMutations === 0 && !hasPendingLocalWrites();
  }

  function requestNativeRefresh() {
    if (!deckApi.native) return;
    refreshRequested = true;
    refreshVersion += 1;
    scheduleRefresh();
  }

  function scheduleRefresh(delay = 80) {
    if (!deckApi.native || !refreshRequested || !canRefresh() || refreshInFlight) return;
    if (refreshTimer) window.clearTimeout(refreshTimer);
    refreshTimer = window.setTimeout(() => {
      refreshTimer = undefined;
      void refreshDeck();
    }, delay);
  }

  async function refreshDeck() {
    if (!canRefresh() || refreshInFlight) {
      refreshRequested = true;
      return;
    }
    refreshInFlight = true;
    refreshRequested = false;
    const startedAtLocalVersion = localStateVersion;
    const startedAtRefreshVersion = refreshVersion;
    try {
      const loaded = await deckApi.load();
      if (
        startedAtLocalVersion !== localStateVersion ||
        startedAtRefreshVersion !== refreshVersion ||
        !canRefresh()
      ) {
        refreshRequested = true;
        return;
      }
      deck = loaded;
      workspaceError = null;
    } catch (error) {
      const message = errorMessage(error);
      workspaceError = message;
      notify('Could not refresh workspace', message, 'warning');
    } finally {
      refreshInFlight = false;
      if (refreshRequested) scheduleRefresh();
    }
  }

  async function withNativeMutation<T>(operation: () => Promise<T>): Promise<T> {
    if (!deckApi.native) return operation();
    pendingNativeMutations += 1;
    touchLocalState();
    if (refreshTimer) {
      window.clearTimeout(refreshTimer);
      refreshTimer = undefined;
    }
    try {
      return await operation();
    } finally {
      pendingNativeMutations -= 1;
      if (refreshRequested) scheduleRefresh();
    }
  }

  function enqueueKeyedOperation<T>(
    chains: Map<string, Promise<void>>,
    key: string,
    operation: () => Promise<T>,
  ): Promise<T> {
    const previous = chains.get(key) ?? Promise.resolve();
    const result = previous.then(operation, operation);
    const settled = result.then(
      () => undefined,
      () => undefined,
    );
    chains.set(key, settled);
    void settled.then(() => {
      if (chains.get(key) === settled) chains.delete(key);
    });
    return result;
  }

  function enqueueSettingsOperation<T>(operation: () => Promise<T>): Promise<T> {
    const result = settingsOperationChain.then(operation, operation);
    settingsOperationChain = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  function finishLocalWrite() {
    if (refreshRequested) scheduleRefresh();
  }

  function cloneState<T>(value: T): T {
    return structuredClone($state.snapshot(value)) as T;
  }

  function persistPreview() {
    if (deckApi.native) return;
    const snapshot = cloneState(deck);
    void deckApi
      .persistPreview(snapshot)
      .catch((error) => notify('Preview state was not saved', errorMessage(error), 'warning'));
  }

  function configurationFor(action: ActionDefinition): Record<string, unknown> {
    if (['launch-application', 'open-file', 'open-folder', 'soundboard'].includes(action.id))
      return { path: '' };
    if (action.id === 'open-website') return { url: '' };
    if (action.id === 'webhook')
      return { url: '', method: 'POST', headers: {}, body: '', timeoutMs: 10_000 };
    if (action.id === 'keyboard-shortcut') return { shortcut: '' };
    if (action.id === 'type-text') return { text: '' };
    if (action.id === 'switch-profile') return { profileId: '' };
    if (action.id === 'close-application') return { executableName: '' };
    return {};
  }

  function mappingKey(profileId: string, control: ControlId) {
    return `${profileId}:${control}`;
  }

  function markMappingChanged(profileId: string, control: ControlId) {
    const key = mappingKey(profileId, control);
    const revision = (mappingRevisions.get(key) ?? 0) + 1;
    mappingRevisions.set(key, revision);
    dirtyMappings.add(key);
    touchLocalState();
    return revision;
  }

  function cancelMappingTimer(profileId: string, control: ControlId) {
    const key = mappingKey(profileId, control);
    const timer = mappingTimers.get(key);
    if (timer) window.clearTimeout(timer);
    mappingTimers.delete(key);
  }

  async function persistMappingSnapshot(
    profileId: string,
    control: ControlId,
    mapping: MappedAction,
    revision: number,
  ) {
    const key = mappingKey(profileId, control);
    try {
      const saved = await enqueueKeyedOperation(mappingOperationChains, key, () =>
        withNativeMutation(() => deckApi.saveMapping(profileId, control, cloneState(mapping))),
      );
      const profile = deck.profiles.find((item) => item.id === profileId);
      const current = profile?.mappings[control];
      if (!profile || mappingRevisions.get(key) !== revision || current?.id !== mapping.id) {
        return false;
      }
      profile.mappings[control] = saved;
      dirtyMappings.delete(key);
      persistPreview();
      finishLocalWrite();
      return true;
    } catch (error) {
      if (mappingRevisions.get(key) === revision) {
        dirtyMappings.delete(key);
        finishLocalWrite();
      }
      throw error;
    }
  }

  async function assignAction(
    action: ActionDefinition,
    control = selectedControl,
    configuration?: Record<string, unknown>,
  ) {
    const targetProfileId = activeProfile.id;
    const targetControl = control;
    const targetProfile = deck.profiles.find((profile) => profile.id === targetProfileId);
    if (!targetProfile) return false;
    const previous = targetProfile.mappings[targetControl]
      ? cloneState(targetProfile.mappings[targetControl])
      : undefined;
    const mapping: MappedAction = {
      id: previous?.id ?? crypto.randomUUID(),
      actionId: action.id,
      title: action.title,
      detail: action.defaultDetail,
      icon: action.icon,
      accent: action.accent,
      trigger: previous?.trigger ?? 'Press',
      configuration: configuration ?? configurationFor(action),
      enabled: previous?.enabled ?? true,
    };
    targetProfile.mappings[targetControl] = mapping;
    selectedControl = targetControl;
    const revision = markMappingChanged(targetProfileId, targetControl);
    try {
      const applied = await persistMappingSnapshot(
        targetProfileId,
        targetControl,
        mapping,
        revision,
      );
      if (applied) {
        notify(
          'Action assigned',
          `${action.title} · ${targetControl.replace('dpad-', 'D-pad ')}`,
          'success',
        );
      }
      return applied;
    } catch (error) {
      const profile = deck.profiles.find((item) => item.id === targetProfileId);
      const current = profile?.mappings[targetControl];
      const key = mappingKey(targetProfileId, targetControl);
      if (profile && mappingRevisions.get(key) === revision && current?.id === mapping.id) {
        if (previous) profile.mappings[targetControl] = previous;
        else delete profile.mappings[targetControl];
        mappingRevisions.set(key, revision + 1);
        dirtyMappings.delete(key);
        touchLocalState();
        persistPreview();
        notify('Action was not saved', errorMessage(error), 'warning');
        requestNativeRefresh();
      }
      return false;
    }
  }

  function assignActionById(control: ControlId, actionId: string) {
    const action = actionCatalog.find((item) => item.id === actionId);
    if (action) void assignAction(action, control);
  }

  function assignFiles(control: ControlId, files: File[]) {
    const file = files[0] as (File & { path?: string }) | undefined;
    if (!file) return;
    if (file.path) {
      void assignPaths(control, [file.path]);
      return;
    }
    if (deckApi.native) {
      notify(
        'File path unavailable',
        'Use Browse or drop the file directly onto a controller control.',
        'warning',
      );
      return;
    }
    void assignPaths(control, [file.name]);
  }

  async function assignPaths(control: ControlId, paths: string[]) {
    const path = paths[0];
    if (!path) return;
    const targetProfileId = activeProfile.id;
    const targetControl = control;
    const actionId = /\.exe$/i.test(path)
      ? 'launch-application'
      : pathLooksLikeFolder(path)
        ? 'open-folder'
        : 'open-file';
    const action = actionCatalog.find((item) => item.id === actionId);
    if (!action) return;
    const assigned = await assignAction(action, targetControl, { path });
    if (!assigned) return;
    const targetProfile = deck.profiles.find((profile) => profile.id === targetProfileId);
    const mapping = targetProfile?.mappings[targetControl];
    if (!mapping) return;
    mapping.title = fileDisplayName(path);
    mapping.detail = path;
    markMappingChanged(targetProfileId, targetControl);
    scheduleMappingSave(targetProfileId, targetControl, 0);
  }

  function updateMapping(changes: Partial<MappedAction>) {
    const targetProfileId = activeProfile.id;
    const targetControl = selectedControl;
    const mapping = activeProfile.mappings[targetControl];
    if (!mapping) return;
    Object.assign(mapping, changes);
    mapping.detail = mappingDetail(mapping);
    markMappingChanged(targetProfileId, targetControl);
    scheduleMappingSave(targetProfileId, targetControl);
  }

  function scheduleMappingSave(profileId: string, control: ControlId, delay = 240) {
    const key = `${profileId}:${control}`;
    const previous = mappingTimers.get(key);
    if (previous) window.clearTimeout(previous);
    mappingTimers.set(
      key,
      window.setTimeout(() => {
        mappingTimers.delete(key);
        void saveMapping(profileId, control);
      }, delay),
    );
  }

  async function saveMapping(profileId: string, control: ControlId) {
    const profile = deck.profiles.find((item) => item.id === profileId);
    const mapping = profile?.mappings[control];
    if (!profile || !mapping) return;
    const revision = mappingRevisions.get(mappingKey(profileId, control)) ?? 0;
    const snapshot = cloneState(mapping);
    try {
      await persistMappingSnapshot(profileId, control, snapshot, revision);
    } catch (error) {
      if (mappingRevisions.get(mappingKey(profileId, control)) === revision) {
        notify('Mapping was not saved', errorMessage(error), 'warning');
        requestNativeRefresh();
      }
    }
  }

  async function browseForMapping() {
    const targetProfileId = activeProfile.id;
    const targetControl = selectedControl;
    const mapping = activeProfile.mappings[targetControl];
    if (!mapping) return;
    const mappingId = mapping.id;
    const actionId = mapping.actionId;
    try {
      const path = await deckApi.choosePath(actionId);
      if (!path) {
        if (!deckApi.native) notify('Browse is available in the desktop app');
        return;
      }
      const profile = deck.profiles.find((item) => item.id === targetProfileId);
      const current = profile?.mappings[targetControl];
      if (!current || current.id !== mappingId || current.actionId !== actionId) return;
      current.configuration = { ...current.configuration, path };
      current.detail = path;
      if (current.title === actionCatalog.find((action) => action.id === actionId)?.title) {
        current.title = fileDisplayName(path);
      }
      markMappingChanged(targetProfileId, targetControl);
      scheduleMappingSave(targetProfileId, targetControl, 0);
    } catch (error) {
      notify('Could not open the file picker', errorMessage(error), 'warning');
    }
  }

  function requestMappingRemoval() {
    const mapping = activeProfile.mappings[selectedControl];
    if (!mapping) return;
    deletionRequest = {
      kind: 'mapping',
      profileId: activeProfile.id,
      control: selectedControl,
      mappingId: mapping.id,
      title: mapping.title,
    };
  }

  async function removeMapping(request: Extract<DeletionRequest, { kind: 'mapping' }>) {
    const targetProfileId = request.profileId;
    const targetControl = request.control;
    const profile = deck.profiles.find((item) => item.id === targetProfileId);
    const mapping = profile?.mappings[targetControl];
    if (!mapping || mapping.id !== request.mappingId) return;
    const snapshot = cloneState(mapping);
    const title = mapping.title;
    cancelMappingTimer(targetProfileId, targetControl);
    delete profile.mappings[targetControl];
    const revision = markMappingChanged(targetProfileId, targetControl);
    try {
      await enqueueKeyedOperation(
        mappingOperationChains,
        mappingKey(targetProfileId, targetControl),
        () => withNativeMutation(() => deckApi.deleteMapping(mapping.id)),
      );
      if (
        mappingRevisions.get(mappingKey(targetProfileId, targetControl)) !== revision ||
        profile.mappings[targetControl]
      ) {
        return;
      }
      dirtyMappings.delete(mappingKey(targetProfileId, targetControl));
      persistPreview();
      finishLocalWrite();
      notify('Mapping removed', `${title} is no longer assigned.`);
    } catch (error) {
      const key = mappingKey(targetProfileId, targetControl);
      if (mappingRevisions.get(key) === revision && !profile.mappings[targetControl]) {
        profile.mappings[targetControl] = snapshot;
        mappingRevisions.set(key, revision + 1);
        dirtyMappings.delete(key);
        touchLocalState();
        persistPreview();
        finishLocalWrite();
        notify('Mapping was not removed', errorMessage(error), 'warning');
        requestNativeRefresh();
      }
    }
  }

  async function runMapping() {
    const mapping = activeProfile.mappings[selectedControl];
    if (!mapping) return;
    const bindingId = mapping.id;
    const title = mapping.title;
    if (deck.mappingsPaused) {
      notify('Mappings are paused', 'Resume mappings before running this action.', 'warning');
      return;
    }
    try {
      await deckApi.runAction(bindingId);
      notify('Action completed', title, 'success');
    } catch (error) {
      notify('Action failed', errorMessage(error), 'warning');
    }
  }

  function changeView(nextView: WorkspaceView) {
    view = nextView;
  }

  async function activateProfile(id: string) {
    const profile = deck.profiles.find((item) => item.id === id);
    if (!profile || id === deck.activeProfileId) return;
    const previousProfileId = deck.activeProfileId;
    const revision = ++activationRevision;
    activationDirty = true;
    deck.activeProfileId = id;
    touchLocalState();
    persistPreview();
    try {
      await enqueueSettingsOperation(() => withNativeMutation(() => deckApi.activateProfile(id)));
      if (activationRevision !== revision) return;
      activationDirty = false;
      finishLocalWrite();
      notify('Profile active', profile.name, 'success');
    } catch (error) {
      if (activationRevision !== revision) return;
      if (deck.profiles.some((item) => item.id === previousProfileId)) {
        deck.activeProfileId = previousProfileId;
      }
      activationDirty = false;
      touchLocalState();
      persistPreview();
      finishLocalWrite();
      notify('Profile was not activated', errorMessage(error), 'warning');
      requestNativeRefresh();
    }
  }

  async function createProfile() {
    const used = new Set(deck.profiles.map((profile) => profile.name.toLowerCase()));
    let number = deck.profiles.length + 1;
    while (used.has(`profile ${number}`)) number += 1;
    const name = `Profile ${number}`;
    try {
      const profile = await withNativeMutation(() => deckApi.createProfile(name));
      if (!deck.profiles.some((item) => item.id === profile.id)) deck.profiles.push(profile);
      touchLocalState();
      persistPreview();
      await activateProfile(profile.id);
      notify('Profile created', profile.name, 'success');
    } catch (error) {
      notify('Profile was not created', errorMessage(error), 'warning');
    }
  }

  async function duplicateProfile(id: string) {
    const source = deck.profiles.find((profile) => profile.id === id);
    if (!source) return;
    const snapshot = cloneState(source);
    try {
      const profile = await withNativeMutation(() => deckApi.duplicateProfile(id, snapshot));
      if (!deck.profiles.some((item) => item.id === profile.id)) deck.profiles.push(profile);
      touchLocalState();
      persistPreview();
      notify('Profile duplicated', profile.name, 'success');
    } catch (error) {
      notify('Profile was not duplicated', errorMessage(error), 'warning');
    }
  }

  function markProfileChanged(id: string) {
    const revision = (profileRevisions.get(id) ?? 0) + 1;
    profileRevisions.set(id, revision);
    dirtyProfiles.add(id);
    touchLocalState();
    return revision;
  }

  function cancelProfileTimer(id: string) {
    const timer = profileTimers.get(id);
    if (timer) window.clearTimeout(timer);
    profileTimers.delete(id);
  }

  async function deleteProfile(id: string) {
    if (deck.profiles.length === 1) return;
    const index = deck.profiles.findIndex((item) => item.id === id);
    if (index < 0) return;
    const profile = cloneState(deck.profiles[index]);
    const previousActiveProfileId = deck.activeProfileId;
    cancelProfileTimer(id);
    const mappingPrefix = `${id}:`;
    const pendingMappingWrites: Promise<void>[] = [];
    for (const [key, operation] of mappingOperationChains) {
      if (key.startsWith(mappingPrefix)) pendingMappingWrites.push(operation);
    }
    for (const [key, timer] of mappingTimers) {
      if (!key.startsWith(mappingPrefix)) continue;
      window.clearTimeout(timer);
      mappingTimers.delete(key);
    }
    for (const key of [...dirtyMappings]) {
      if (!key.startsWith(mappingPrefix)) continue;
      mappingRevisions.set(key, (mappingRevisions.get(key) ?? 0) + 1);
      dirtyMappings.delete(key);
    }
    deck.profiles = deck.profiles.filter((item) => item.id !== id);
    if (deck.activeProfileId === id) {
      activationRevision += 1;
      activationDirty = false;
      deck.activeProfileId = deck.profiles[0].id;
    }
    const revision = markProfileChanged(id);
    persistPreview();
    try {
      await enqueueKeyedOperation(profileOperationChains, id, async () => {
        await Promise.all(pendingMappingWrites);
        await enqueueSettingsOperation(() => withNativeMutation(() => deckApi.deleteProfile(id)));
      });
      if (profileRevisions.get(id) !== revision || deck.profiles.some((item) => item.id === id)) {
        return;
      }
      dirtyProfiles.delete(id);
      finishLocalWrite();
      notify('Profile deleted', profile.name);
    } catch (error) {
      if (profileRevisions.get(id) === revision && !deck.profiles.some((item) => item.id === id)) {
        deck.profiles.splice(Math.min(index, deck.profiles.length), 0, profile);
        if (previousActiveProfileId === id) deck.activeProfileId = id;
        profileRevisions.set(id, revision + 1);
        dirtyProfiles.delete(id);
        touchLocalState();
        persistPreview();
        finishLocalWrite();
        notify('Profile was not deleted', errorMessage(error), 'warning');
        requestNativeRefresh();
      }
    }
  }

  function requestProfileDeletion(id: string) {
    if (deck.profiles.length === 1) return;
    const profile = deck.profiles.find((item) => item.id === id);
    if (!profile) return;
    deletionRequest = { kind: 'profile', profileId: profile.id, title: profile.name };
  }

  function confirmDeletion() {
    const request = deletionRequest;
    if (!request) return;
    deletionRequest = null;
    if (request.kind === 'mapping') void removeMapping(request);
    else void deleteProfile(request.profileId);
  }

  function updateProfile(id: string, changes: Partial<Profile>) {
    const profile = deck.profiles.find((item) => item.id === id);
    if (!profile) return;
    Object.assign(profile, changes);
    markProfileChanged(id);
    const previous = profileTimers.get(id);
    if (previous) window.clearTimeout(previous);
    profileTimers.set(
      id,
      window.setTimeout(() => {
        profileTimers.delete(id);
        void saveProfile(id);
      }, 320),
    );
  }

  async function saveProfile(id: string) {
    const profile = deck.profiles.find((item) => item.id === id);
    if (!profile) return;
    const revision = profileRevisions.get(id) ?? 0;
    const snapshot = cloneState(profile);
    try {
      const updated = await enqueueKeyedOperation(profileOperationChains, id, () =>
        withNativeMutation(() => deckApi.saveProfile(snapshot)),
      );
      const current = deck.profiles.find((item) => item.id === id);
      if (!current || profileRevisions.get(id) !== revision) return;
      current.name = updated.name;
      current.description = updated.description;
      current.accent = updated.accent;
      current.applicationRule = updated.applicationRule;
      current.createdAt = updated.createdAt;
      dirtyProfiles.delete(id);
      persistPreview();
      finishLocalWrite();
    } catch (error) {
      if (profileRevisions.get(id) === revision) {
        dirtyProfiles.delete(id);
        finishLocalWrite();
        notify('Profile was not saved', errorMessage(error), 'warning');
        requestNativeRefresh();
      }
    }
  }

  function updatePreference<K extends keyof Preferences>(key: K, value: Preferences[K]) {
    deck.preferences[key] = value;
    preferencesRevision += 1;
    settingsDirty = true;
    touchLocalState();
    if (settingsTimer) window.clearTimeout(settingsTimer);
    settingsTimer = window.setTimeout(() => void savePreferences(), 180);
  }

  async function savePreferences() {
    settingsTimer = undefined;
    const revision = preferencesRevision;
    const activeProfileId = deck.activeProfileId;
    const preferences = cloneState(deck.preferences);
    const mappingsPaused = deck.mappingsPaused;
    try {
      await enqueueSettingsOperation(() =>
        withNativeMutation(() =>
          deckApi.savePreferences(activeProfileId, preferences, mappingsPaused),
        ),
      );
      if (preferencesRevision !== revision) return;
      settingsDirty = false;
      persistPreview();
      finishLocalWrite();
    } catch (error) {
      if (preferencesRevision === revision) {
        settingsDirty = false;
        finishLocalWrite();
        notify('Settings were not saved', errorMessage(error), 'warning');
        requestNativeRefresh();
      }
    }
  }

  async function refreshController() {
    const revision = ++controllerRefreshRevision;
    try {
      const status = await deckApi.controllerStatus();
      if (controllerRefreshRevision !== revision) return;
      deck.device = status.device;
      if (status.paused !== null && !pauseDirty) deck.mappingsPaused = status.paused;
      deck.pressedControls = status.pressedControls;
      touchLocalState();
      if (status.device.connected) {
        notify(
          'Controller ready',
          `${status.device.name} · ${status.device.connection}`,
          'success',
        );
      } else {
        notify(
          'DualSense not found',
          status.device.initializationError ??
            'Connect the controller by USB. Detection continues automatically.',
          'warning',
        );
      }
    } catch (error) {
      notify('Controller status unavailable', errorMessage(error), 'warning');
    }
  }

  async function toggleMappings() {
    const previousPaused = deck.mappingsPaused;
    const paused = !deck.mappingsPaused;
    const revision = ++pauseRevision;
    pauseDirty = true;
    deck.mappingsPaused = paused;
    touchLocalState();
    persistPreview();
    try {
      await enqueueSettingsOperation(() =>
        withNativeMutation(() => deckApi.setMappingsPaused(paused)),
      );
      if (pauseRevision !== revision) return;
      pauseDirty = false;
      finishLocalWrite();
      notify(
        paused ? 'Mappings paused' : 'Mappings resumed',
        paused
          ? 'Controller input still passes through to other apps.'
          : 'Assigned actions are active again.',
        paused ? 'warning' : 'success',
      );
    } catch (error) {
      if (pauseRevision !== revision) return;
      deck.mappingsPaused = previousPaused;
      pauseDirty = false;
      touchLocalState();
      persistPreview();
      finishLocalWrite();
      notify('Pause state was not changed', errorMessage(error), 'warning');
      requestNativeRefresh();
    }
  }

  function findMapping(id: string): MappedAction | undefined {
    for (const profile of deck.profiles) {
      const mapping = Object.values(profile.mappings).find((item) => item?.id === id);
      if (mapping) return mapping;
    }
    return undefined;
  }

  function fileDisplayName(path: string): string {
    const name = path.split(/[\\/]/).filter(Boolean).pop() ?? path;
    return name.replace(/\.[^.]+$/, '') || name;
  }

  function pathLooksLikeFolder(path: string): boolean {
    const name = path.split(/[\\/]/).filter(Boolean).pop() ?? '';
    return !name.includes('.');
  }

  function mappingDetail(mapping: MappedAction): string {
    const path = typeof mapping.configuration.path === 'string' ? mapping.configuration.path : '';
    const url = typeof mapping.configuration.url === 'string' ? mapping.configuration.url : '';
    const shortcut =
      typeof mapping.configuration.shortcut === 'string' ? mapping.configuration.shortcut : '';
    const text = typeof mapping.configuration.text === 'string' ? mapping.configuration.text : '';
    if (path) return path;
    if (url) return url;
    if (shortcut) return shortcut;
    if (text) return text;
    return (
      actionCatalog.find((action) => action.id === mapping.actionId)?.defaultDetail ??
      mapping.detail
    );
  }

  async function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      view = 'editor';
      await tick();
      document.querySelector<HTMLInputElement>('#action-search')?.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class:motion-reduced={deck.preferences.reducedMotion} class="app-shell">
  <TopBar
    profiles={deck.profiles}
    activeProfileId={deck.activeProfileId}
    {view}
    device={deck.device}
    mappingsPaused={deck.mappingsPaused}
    onviewchange={changeView}
    onprofilechange={activateProfile}
    onrefreshdevice={refreshController}
    onpausetoggle={toggleMappings}
  />

  {#if workspaceError}
    <div class="workspace-error" role="alert">
      <TriangleAlert size={17} aria-hidden="true" />
      <span>
        <strong>Saved workspace could not be loaded</strong>
        <small>{workspaceError}</small>
      </span>
      <button
        type="button"
        aria-label="Dismiss workspace error"
        onclick={() => (workspaceError = null)}><X size={15} /></button
      >
    </div>
  {/if}

  {#key view}
    <div class:editor-view={view === 'editor'} class="workspace-view">
      {#if view === 'editor'}
        <div class="editor-grid">
          <ActionLibrary onassign={(action) => void assignAction(action)} />
          <ControllerCanvas
            profile={activeProfile}
            {selectedControl}
            device={deck.device}
            pressedControls={deck.pressedControls}
            onselect={(control) => (selectedControl = control)}
            ondropaction={assignActionById}
            ondropfiles={assignFiles}
            onreconnect={refreshController}
          />
          <Inspector
            control={selectedControl}
            mapping={selectedMapping}
            profiles={deck.profiles}
            onassign={(action) => void assignAction(action)}
            onupdate={updateMapping}
            onremove={requestMappingRemoval}
            onrun={() => void runMapping()}
            onbrowse={() => void browseForMapping()}
          />
        </div>
      {:else if view === 'profiles'}
        <ProfilesView
          profiles={deck.profiles}
          activeProfileId={deck.activeProfileId}
          onactivate={(id) => void activateProfile(id)}
          oncreate={() => void createProfile()}
          onduplicate={(id) => void duplicateProfile(id)}
          ondelete={requestProfileDeletion}
          onupdate={updateProfile}
        />
      {:else}
        <SettingsView
          appVersion={deck.appVersion}
          preferences={deck.preferences}
          device={deck.device}
          onpreference={updatePreference}
          onscan={() => void refreshController()}
        />
      {/if}
    </div>
  {/key}

  {#if deletionRequest && deletionDialog}
    <ConfirmDialog
      title={deletionDialog.title}
      description={deletionDialog.description}
      confirmLabel={deletionDialog.confirmLabel}
      onconfirm={confirmDeletion}
      oncancel={() => (deletionRequest = null)}
    />
  {/if}

  <ToastHost {toasts} ondismiss={dismissToast} />
</div>

<style>
  .app-shell {
    position: relative;
    display: grid;
    width: 100%;
    height: 100%;
    grid-template-rows: 60px minmax(0, 1fr);
    color: var(--text);
    background: var(--surface-canvas);
  }

  .editor-grid {
    display: grid;
    min-width: 0;
    min-height: 0;
    grid-template-columns: minmax(232px, 260px) minmax(480px, 1fr) minmax(268px, 294px);
  }

  .workspace-view {
    display: grid;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    animation: workspace-view-in var(--motion-base) var(--ease-emphasized) both;
  }

  .workspace-view.editor-view {
    display: block;
  }

  .workspace-view.editor-view .editor-grid {
    height: 100%;
  }

  .workspace-error {
    position: absolute;
    z-index: 25;
    top: 70px;
    left: 50%;
    display: grid;
    width: min(620px, calc(100% - 32px));
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 11px 12px;
    border: 1px solid color-mix(in srgb, var(--danger) 42%, var(--border));
    border-radius: 10px;
    color: var(--danger);
    background: color-mix(in srgb, var(--surface-overlay) 95%, transparent);
    box-shadow: var(--shadow-large);
    transform: translateX(-50%);
    animation: workspace-error-in var(--motion-slow) var(--ease-emphasized) both;
  }

  .workspace-error > span {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .workspace-error strong {
    color: var(--text);
    font-size: 11px;
  }

  .workspace-error small {
    color: var(--text-muted);
    font-size: 10px;
    line-height: 1.4;
  }

  .workspace-error button {
    display: grid;
    width: 28px;
    height: 28px;
    padding: 0;
    place-items: center;
    border: 0;
    border-radius: 7px;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
  }

  .workspace-error button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  @keyframes workspace-view-in {
    from {
      opacity: 0;
      transform: translateY(5px);
    }
  }

  @keyframes workspace-error-in {
    from {
      opacity: 0;
      transform: translate(-50%, -6px) scale(0.985);
    }
  }

  .motion-reduced,
  .motion-reduced *,
  .motion-reduced *::before,
  .motion-reduced *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }

  @media (max-width: 1220px) {
    .editor-grid {
      grid-template-columns: 230px minmax(480px, 1fr) 264px;
    }
  }

  @media (max-width: 1080px) {
    .editor-grid {
      grid-template-columns: 208px minmax(380px, 1fr) 236px;
    }
  }
</style>
