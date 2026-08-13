<script lang="ts">
  import {
    ArrowRightLeft,
    ChevronRight,
    CirclePlay,
    FileSearch,
    GripVertical,
    Info,
    Keyboard,
    Plus,
    Trash2,
  } from '@lucide/svelte';
  import { actionCatalog, controlNames, controlShortNames } from '../data/catalog';
  import { triggerModes } from '../models';
  import type { ActionDefinition, ControlId, MappedAction, Profile, TriggerMode } from '../models';
  import ActionIcon from './ActionIcon.svelte';

  interface Props {
    control: ControlId;
    mapping?: MappedAction;
    profiles: Profile[];
    onassign: (action: ActionDefinition) => void;
    onupdate: (changes: Partial<MappedAction>) => void;
    onremove: () => void;
    onrun: () => void;
    onbrowse: () => void;
  }

  let { control, mapping, profiles, onassign, onupdate, onremove, onrun, onbrowse }: Props =
    $props();
  let inspectorDrag = $state(false);
  let recordingShortcut = $state(false);
  const suggestions = actionCatalog.filter((action) =>
    ['launch-application', 'keyboard-shortcut', 'media-play-pause'].includes(action.id),
  );
  function updateConfiguration(key: string, value: string | number | boolean) {
    if (!mapping) return;
    onupdate({ configuration: { ...mapping.configuration, [key]: value } });
  }

  function inspectorDrop(event: DragEvent) {
    event.preventDefault();
    inspectorDrag = false;
    const actionId = event.dataTransfer?.getData('application/x-dual-deck-action');
    const action = actionCatalog.find((item) => item.id === actionId);
    if (action) onassign(action);
  }

  function captureShortcut(event: KeyboardEvent) {
    if (!recordingShortcut || !mapping) return;
    event.preventDefault();
    event.stopPropagation();
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return;
    if (event.key === 'Escape') {
      recordingShortcut = false;
      return;
    }
    const parts: string[] = [];
    if (event.ctrlKey) parts.push('Ctrl');
    if (event.altKey) parts.push('Alt');
    if (event.shiftKey) parts.push('Shift');
    if (event.metaKey) parts.push('Win');
    parts.push(shortcutKey(event.key));
    updateConfiguration('shortcut', parts.join(' + '));
    recordingShortcut = false;
  }

  function shortcutKey(key: string): string {
    const names: Record<string, string> = {
      ' ': 'Space',
      ArrowUp: 'Up',
      ArrowDown: 'Down',
      ArrowLeft: 'Left',
      ArrowRight: 'Right',
      PageUp: 'PageUp',
      PageDown: 'PageDown',
      '+': 'Plus',
      '-': 'Minus',
      ',': 'Comma',
      '.': 'Period',
    };
    return names[key] ?? (key.length === 1 ? key.toUpperCase() : key);
  }

  function mappingIsComplete(value: MappedAction): boolean {
    const pathActions = ['launch-application', 'open-file', 'open-folder', 'soundboard'];
    if (pathActions.includes(value.actionId))
      return Boolean(String(value.configuration.path ?? '').trim());
    if (['open-website', 'webhook'].includes(value.actionId))
      return Boolean(String(value.configuration.url ?? '').trim());
    if (value.actionId === 'keyboard-shortcut')
      return Boolean(String(value.configuration.shortcut ?? '').trim());
    if (value.actionId === 'type-text') return Boolean(String(value.configuration.text ?? ''));
    if (value.actionId === 'switch-profile')
      return Boolean(String(value.configuration.profileId ?? '').trim());
    if (value.actionId === 'close-application')
      return Boolean(String(value.configuration.executableName ?? '').trim());
    if (value.actionId === 'multi-action')
      return Array.isArray(value.configuration.steps) && value.configuration.steps.length > 0;
    return true;
  }
</script>

<aside class="inspector" aria-label="Control inspector">
  <header>
    <div>
      <span class="section-label">Inspector</span>
      <h2>{controlNames[control]}</h2>
    </div>
    <span class="control-badge">{controlShortNames[control]}</span>
  </header>

  {#if mapping}
    <div class="inspector-scroll mapped-state">
      <section class="mapped-card" style={`--mapping-accent:${mapping.accent}`}>
        <span class="mapped-card-icon"><ActionIcon name={mapping.icon} size={20} /></span>
        <span>
          <small>Assigned action</small>
          <strong>{mapping.title}</strong>
          <em>{mapping.detail}</em>
        </span>
      </section>

      <section class="form-section">
        <div class="form-heading">
          <span>Action</span>
        </div>
        <label class="field">
          <span>Name</span>
          <input
            value={mapping.title}
            oninput={(event) => onupdate({ title: event.currentTarget.value })}
          />
        </label>

        {#if ['launch-application', 'open-file', 'open-folder', 'soundboard'].includes(mapping.actionId)}
          <label class="field">
            <span
              >{mapping.actionId === 'launch-application'
                ? 'Application'
                : mapping.actionId === 'soundboard'
                  ? 'WAV file'
                  : 'Location'}</span
            >
            <div class="field-with-button">
              <input
                value={String(mapping.configuration.path ?? '')}
                placeholder={mapping.detail}
                oninput={(event) => updateConfiguration('path', event.currentTarget.value)}
              />
              <button type="button" aria-label="Browse" onclick={onbrowse}>
                <FileSearch size={15} />
              </button>
            </div>
          </label>
        {:else if ['open-website', 'webhook'].includes(mapping.actionId)}
          <label class="field">
            <span>{mapping.actionId === 'webhook' ? 'Endpoint' : 'Web address'}</span>
            <input
              type="url"
              value={String(mapping.configuration.url ?? '')}
              placeholder="https://"
              oninput={(event) => updateConfiguration('url', event.currentTarget.value)}
            />
          </label>
        {:else if mapping.actionId === 'keyboard-shortcut'}
          <label class="field">
            <span>Shortcut</span>
            <button
              type="button"
              class="shortcut-recorder"
              class:recording={recordingShortcut}
              onclick={() => (recordingShortcut = true)}
              onkeydown={captureShortcut}
            >
              <Keyboard size={15} />
              {recordingShortcut
                ? 'Press a shortcut'
                : String(mapping.configuration.shortcut || 'Record shortcut')}
              <kbd>{recordingShortcut ? 'Esc' : 'Record'}</kbd>
            </button>
          </label>
        {:else if mapping.actionId === 'type-text'}
          <label class="field">
            <span>Text</span>
            <textarea
              rows="4"
              value={String(mapping.configuration.text ?? '')}
              placeholder="Text to type"
              oninput={(event) => updateConfiguration('text', event.currentTarget.value)}
            ></textarea>
          </label>
        {:else if mapping.actionId === 'switch-profile'}
          <label class="field">
            <span>Destination profile</span>
            <select
              value={String(mapping.configuration.profileId ?? '')}
              onchange={(event) => updateConfiguration('profileId', event.currentTarget.value)}
            >
              <option value="">Choose profile</option>
              {#each profiles as profile}
                <option value={profile.id}>{profile.name}</option>
              {/each}
            </select>
          </label>
        {:else if mapping.actionId === 'close-application'}
          <label class="field">
            <span>Executable name</span>
            <input
              value={String(mapping.configuration.executableName ?? '')}
              placeholder="example.exe"
              oninput={(event) => updateConfiguration('executableName', event.currentTarget.value)}
            />
          </label>
        {:else if mapping.actionId === 'multi-action'}
          <div class="sequence-field">
            <span>Sequence</span>
            <div class="sequence-empty">
              <GripVertical size={14} />
              <span
                >{Array.isArray(mapping.configuration.steps)
                  ? `${mapping.configuration.steps.length} saved steps`
                  : 'No steps configured'}</span
              >
            </div>
          </div>
        {/if}

        {#if !mappingIsComplete(mapping)}
          <div class="notice-row">
            <Info size={14} />
            <span>Finish configuring this action before it can run.</span>
          </div>
        {/if}
      </section>

      <section class="form-section">
        <div class="form-heading">
          <span>Activation</span>
        </div>
        <label class="field">
          <span>Trigger</span>
          <select
            value={mapping.trigger}
            onchange={(event) => onupdate({ trigger: event.currentTarget.value as TriggerMode })}
          >
            {#each triggerModes as mode}<option value={mode}>{mode}</option>{/each}
          </select>
        </label>
        <div class="passthrough-row">
          <span class="passthrough-icon"><ArrowRightLeft size={15} /></span>
          <span>
            <strong>Pass through input</strong>
            <small>The active app also receives this button</small>
          </span>
          <span class="always-on">On</span>
        </div>
        <div class="notice-row">
          <Info size={14} />
          <span>Mappings work while Dual Deck is in the notification area.</span>
        </div>
      </section>
    </div>

    <footer class="inspector-footer">
      <button type="button" class="remove-button" aria-label="Remove mapping" onclick={onremove}
        ><Trash2 size={15} /></button
      >
      <button type="button" class="test-button" onclick={onrun}
        ><CirclePlay size={15} /> Run action</button
      >
    </footer>
  {:else}
    <div class="inspector-scroll empty-state">
      <div
        class:dragging={inspectorDrag}
        class="assign-zone"
        role="region"
        aria-label={`Assign an action to ${controlNames[control]}`}
        ondragover={(event) => {
          event.preventDefault();
          inspectorDrag = true;
        }}
        ondragleave={() => (inspectorDrag = false)}
        ondrop={inspectorDrop}
      >
        <span class="assign-glyph"><Plus size={18} /></span>
        <strong>Assign an action</strong>
        <p>Drag one here or choose from the library.</p>
      </div>

      <section class="suggestions">
        <span class="suggestion-label">Quick choices</span>
        {#each suggestions as action}
          <button type="button" onclick={() => onassign(action)}>
            <span class="suggestion-icon" style={`--action-accent:${action.accent}`}>
              <ActionIcon name={action.icon} size={16} />
            </span>
            <span>
              <strong>{action.title}</strong>
              <small>{action.description}</small>
            </span>
            <ChevronRight size={14} />
          </button>
        {/each}
      </section>

      <div class="empty-note">
        <Info size={14} />
        <span>Every control can have a different action in each profile.</span>
      </div>
    </div>
  {/if}
</aside>

<style>
  .inspector {
    display: grid;
    min-width: 0;
    min-height: 0;
    grid-template-rows: auto 1fr auto;
    border-left: 1px solid var(--border);
    background: var(--surface-panel);
  }

  header {
    display: flex;
    min-height: 62px;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 15px;
    border-bottom: 1px solid var(--border);
  }

  header > div {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .section-label,
  .suggestion-label {
    color: var(--text-subtle);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h2 {
    overflow: hidden;
    margin: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .control-badge {
    display: grid;
    min-width: 32px;
    height: 28px;
    padding: 0 7px;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    color: var(--text-muted);
    background: var(--surface-high);
    font-size: 10px;
    font-weight: 700;
  }

  .inspector-scroll {
    min-height: 0;
    padding: 14px;
    overflow: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--surface-high) transparent;
  }

  .mapped-card {
    display: grid;
    min-height: 66px;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 11px;
    padding: 10px;
    border: 1px solid color-mix(in srgb, var(--mapping-accent) 24%, var(--border));
    border-radius: 10px;
    background: color-mix(in srgb, var(--mapping-accent) 6%, var(--surface-raised));
  }

  .mapped-card-icon {
    display: grid;
    width: 39px;
    height: 39px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--mapping-accent) 30%, var(--border));
    border-radius: 9px;
    color: var(--mapping-accent);
    background: color-mix(in srgb, var(--mapping-accent) 10%, var(--surface-high));
  }

  .mapped-card > span:nth-child(2) {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .mapped-card small,
  .mapped-card em {
    overflow: hidden;
    color: var(--text-subtle);
    font-size: 10px;
    font-style: normal;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mapped-card strong {
    overflow: hidden;
    color: var(--text);
    font-size: 11px;
    font-weight: 640;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .form-section {
    margin-top: 17px;
  }

  .form-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 9px;
    color: var(--text-subtle);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }

  .field,
  .sequence-field {
    display: grid;
    gap: 6px;
    margin-top: 10px;
  }

  .field > span,
  .sequence-field > span {
    color: var(--text-muted);
    font-size: 10px;
    font-weight: 570;
  }

  input,
  select,
  textarea,
  .shortcut-recorder {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    outline: 0;
    color: var(--text);
    background: var(--surface-base);
    font-size: 10px;
  }

  input,
  select {
    height: 34px;
    padding: 0 9px;
  }

  select {
    color-scheme: dark;
  }

  textarea {
    padding: 8px 9px;
    resize: vertical;
    line-height: 1.45;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: color-mix(in srgb, var(--accent) 60%, var(--border));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 9%, transparent);
  }

  input::placeholder,
  textarea::placeholder {
    color: var(--text-disabled);
  }

  .field-with-button {
    display: grid;
    grid-template-columns: 1fr auto;
  }

  .field-with-button input {
    border-radius: 7px 0 0 7px;
  }

  .field-with-button button {
    display: grid;
    width: 35px;
    place-items: center;
    border: 1px solid var(--border);
    border-left: 0;
    border-radius: 0 7px 7px 0;
    color: var(--text-muted);
    background: var(--surface-high);
    cursor: pointer;
  }

  .field-with-button button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .shortcut-recorder {
    display: grid;
    height: 38px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 10px;
    color: var(--text-muted);
    text-align: left;
    cursor: pointer;
  }

  .shortcut-recorder:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .shortcut-recorder.recording {
    border-color: var(--accent-bright);
    color: var(--accent-bright);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .shortcut-recorder kbd {
    padding: 3px 5px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    color: var(--text-subtle);
    background: var(--surface-high);
    font-family: inherit;
    font-size: 10px;
  }

  .sequence-empty {
    display: grid;
    min-height: 42px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 7px;
    padding: 0 8px;
    border: 1px dashed var(--border-strong);
    border-radius: 8px;
    color: var(--text-disabled);
    font-size: 10px;
  }

  .passthrough-row {
    display: grid;
    min-height: 52px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 9px;
    margin-top: 10px;
    padding: 0 9px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-base);
  }

  .passthrough-icon {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border-radius: 7px;
    color: var(--text-muted);
    background: var(--surface-high);
  }

  .passthrough-row > span:nth-child(2) {
    display: grid;
    gap: 2px;
  }

  .passthrough-row strong {
    color: var(--text);
    font-size: 10px;
    font-weight: 600;
  }

  .passthrough-row small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .always-on {
    padding: 3px 5px;
    border-radius: 5px;
    color: var(--positive);
    background: color-mix(in srgb, var(--positive) 10%, var(--surface-high));
    font-size: 10px;
    font-weight: 700;
  }

  .notice-row,
  .empty-note {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    margin-top: 10px;
    padding: 8px;
    border-radius: 7px;
    color: var(--text-subtle);
    background: var(--surface-high);
    font-size: 10px;
    line-height: 1.4;
  }

  .notice-row :global(svg),
  .empty-note :global(svg) {
    flex: 0 0 auto;
    margin-top: 1px;
  }

  .inspector-footer {
    display: grid;
    min-height: 56px;
    grid-template-columns: 36px 1fr;
    align-items: center;
    gap: 8px;
    padding: 0 13px;
    border-top: 1px solid var(--border);
    background: var(--surface-base);
  }

  .inspector-footer button {
    height: 34px;
    border-radius: 8px;
    font-size: 10px;
    font-weight: 620;
    cursor: pointer;
  }

  .remove-button {
    display: grid;
    width: 36px;
    padding: 0;
    place-items: center;
    border: 1px solid var(--border);
    color: var(--text-subtle);
    background: var(--surface-raised);
  }

  .remove-button:hover {
    border-color: color-mix(in srgb, var(--danger) 40%, var(--border));
    color: var(--danger);
  }

  .test-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: 1px solid color-mix(in srgb, var(--accent) 50%, var(--border));
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface-raised));
  }

  .test-button:hover {
    background: color-mix(in srgb, var(--accent) 18%, var(--surface-raised));
  }

  .empty-state {
    display: grid;
    align-content: start;
    gap: 18px;
  }

  .assign-zone {
    display: grid;
    min-height: 150px;
    place-items: center;
    align-content: center;
    gap: 7px;
    padding: 18px;
    border: 1px dashed var(--border-strong);
    border-radius: 11px;
    background: var(--surface-base);
    text-align: center;
    transition:
      border-color 140ms ease,
      background-color 140ms ease;
  }

  .assign-zone.dragging {
    border-color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface-base));
  }

  .assign-glyph {
    display: grid;
    width: 38px;
    height: 38px;
    margin-bottom: 2px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border));
    border-radius: 10px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface-raised));
  }

  .assign-zone strong {
    color: var(--text);
    font-size: 11px;
    font-weight: 630;
  }

  .assign-zone p {
    max-width: 190px;
    margin: 0;
    color: var(--text-subtle);
    font-size: 10px;
    line-height: 1.5;
  }

  .suggestions {
    display: grid;
    gap: 5px;
  }

  .suggestion-label {
    margin: 0 2px 2px;
  }

  .suggestions button {
    display: grid;
    min-height: 51px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 9px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 9px;
    color: var(--text-subtle);
    background: var(--surface-base);
    text-align: left;
    cursor: pointer;
  }

  .suggestions button:hover {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--surface-hover);
  }

  .suggestion-icon {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border-radius: 8px;
    color: var(--action-accent);
    background: color-mix(in srgb, var(--action-accent) 9%, var(--surface-high));
  }

  .suggestions button > span:nth-child(2) {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .suggestions strong,
  .suggestions small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .suggestions strong {
    color: var(--text);
    font-size: 10px;
    font-weight: 600;
  }

  .suggestions small {
    color: var(--text-subtle);
    font-size: 10px;
  }
</style>
