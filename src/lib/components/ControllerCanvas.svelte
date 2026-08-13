<script lang="ts">
  import { Cable, ChevronRight, CircleHelp, MousePointerClick, Usb } from '@lucide/svelte';
  import { onDestroy } from 'svelte';
  import controllerImage from '../../assets/dualsense-controller.webp';
  import { controlNames, controlShortNames } from '../data/catalog';
  import type { ControlId, ControllerDevice, MappedAction, Profile } from '../models';
  import ActionIcon from './ActionIcon.svelte';

  interface Props {
    profile: Profile;
    selectedControl: ControlId;
    device: ControllerDevice;
    pressedControls: ControlId[];
    onselect: (control: ControlId) => void;
    ondropaction: (control: ControlId, actionId: string) => void;
    ondropfiles: (control: ControlId, files: File[]) => void;
    onreconnect: () => void;
  }

  interface Hotspot {
    id: ControlId;
    x: number;
    y: number;
    kind: 'face' | 'dpad' | 'trigger' | 'utility' | 'touchpad' | 'stick' | 'center';
  }

  let {
    profile,
    selectedControl,
    device,
    pressedControls,
    onselect,
    ondropaction,
    ondropfiles,
    onreconnect,
  }: Props = $props();
  let dragTarget = $state<ControlId | null>(null);
  let controllerBoard: HTMLDivElement;
  let tiltFrame: number | undefined;
  let tiltX = $state(0);
  let tiltY = $state(0);
  let lightX = $state(50);
  let lightY = $state(34);

  const hotspots: Hotspot[] = [
    { id: 'l2', x: 18.5, y: 4.1, kind: 'trigger' },
    { id: 'l1', x: 27.7, y: 7.1, kind: 'trigger' },
    { id: 'r2', x: 81.5, y: 4.1, kind: 'trigger' },
    { id: 'r1', x: 72.3, y: 7.1, kind: 'trigger' },
    { id: 'create', x: 31.4, y: 12.7, kind: 'utility' },
    { id: 'options', x: 69.8, y: 12.7, kind: 'utility' },
    { id: 'touchpad', x: 50.7, y: 18.3, kind: 'touchpad' },
    { id: 'dpad-up', x: 24.7, y: 17.7, kind: 'dpad' },
    { id: 'dpad-right', x: 29.1, y: 24.8, kind: 'dpad' },
    { id: 'dpad-down', x: 24.7, y: 32.5, kind: 'dpad' },
    { id: 'dpad-left', x: 20.2, y: 24.8, kind: 'dpad' },
    { id: 'triangle', x: 76.5, y: 15.9, kind: 'face' },
    { id: 'circle', x: 82.3, y: 23.7, kind: 'face' },
    { id: 'cross', x: 76.6, y: 33.7, kind: 'face' },
    { id: 'square', x: 70.3, y: 24.1, kind: 'face' },
    { id: 'l3', x: 38, y: 40.5, kind: 'stick' },
    { id: 'r3', x: 64, y: 40.5, kind: 'stick' },
    { id: 'ps', x: 50.6, y: 41.3, kind: 'center' },
    { id: 'mute', x: 50.6, y: 49.4, kind: 'utility' },
  ];

  let mappingCount = $derived(Object.keys(profile.mappings).length);
  let selectedMapping = $derived(profile.mappings[selectedControl]);

  function drop(event: DragEvent, control: ControlId) {
    event.preventDefault();
    dragTarget = null;
    const actionId = event.dataTransfer?.getData('application/x-dual-deck-action');
    if (actionId) {
      ondropaction(control, actionId);
      return;
    }
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length) ondropfiles(control, files);
  }

  function dragOver(event: DragEvent, control: ControlId) {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    dragTarget = control;
  }

  function motionDisabled() {
    return (
      Boolean(controllerBoard?.closest('.motion-reduced')) ||
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    );
  }

  function resetTilt() {
    if (tiltFrame !== undefined) cancelAnimationFrame(tiltFrame);
    tiltFrame = undefined;
    tiltX = 0;
    tiltY = 0;
    lightX = 50;
    lightY = 34;
  }

  function moveController(event: PointerEvent) {
    if (!device.connected || dragTarget || motionDisabled()) {
      resetTilt();
      return;
    }
    const bounds = controllerBoard.getBoundingClientRect();
    const horizontal = Math.max(
      -1,
      Math.min(1, ((event.clientX - bounds.left) / bounds.width) * 2 - 1),
    );
    const vertical = Math.max(
      -1,
      Math.min(1, ((event.clientY - bounds.top) / bounds.height) * 2 - 1),
    );
    if (tiltFrame !== undefined) cancelAnimationFrame(tiltFrame);
    tiltFrame = requestAnimationFrame(() => {
      tiltX = vertical * -0.85;
      tiltY = horizontal * 1.15;
      lightX = 50 + horizontal * 24;
      lightY = 34 + vertical * 18;
      tiltFrame = undefined;
    });
  }

  onDestroy(() => {
    if (tiltFrame !== undefined) cancelAnimationFrame(tiltFrame);
  });
</script>

<main class="canvas-shell" aria-label="Controller mapping workspace">
  <header class="canvas-header">
    <div>
      <span class="section-label">Controller map</span>
      <h1>{profile.name}</h1>
    </div>
    <div class="canvas-meta">
      <span class:online={device.connected} class="connection-state">
        <span></span>
        {device.connected ? `${device.connection} connected` : 'Controller disconnected'}
      </span>
      <span class="mapping-count">{mappingCount} {mappingCount === 1 ? 'mapping' : 'mappings'}</span
      >
    </div>
  </header>

  <section class:disconnected={!device.connected} class="controller-stage">
    {#if !device.connected}
      <div class="connection-callout">
        <span class="callout-icon"><Usb size={17} /></span>
        <span>
          <strong>DualSense is not connected</strong>
          <small>You can keep editing. Connect by USB to use your mappings.</small>
        </span>
        <button type="button" onclick={onreconnect}><Cable size={14} /> Scan</button>
      </div>
    {/if}

    <div
      bind:this={controllerBoard}
      class="controller-board"
      role="group"
      aria-label="DualSense controller controls"
      onpointermove={moveController}
      onpointerleave={resetTilt}
    >
      <div
        class="controller-tilt"
        style={`--tilt-x:${tiltX}deg;--tilt-y:${tiltY}deg;--light-x:${lightX}%;--light-y:${lightY}%;--controller-mask:url("${controllerImage}")`}
      >
        <span class="controller-aura" aria-hidden="true"></span>
        <img
          class="controller-art"
          src={controllerImage}
          alt=""
          draggable="false"
          aria-hidden="true"
        />
        <span class="controller-specular" aria-hidden="true"></span>

        {#each hotspots as hotspot (hotspot.id)}
          {@const mapping = profile.mappings[hotspot.id]}
          <button
            type="button"
            class:mapped={Boolean(mapping)}
            class:pressed={pressedControls.includes(hotspot.id)}
            class:selected={selectedControl === hotspot.id}
            class:dragover={dragTarget === hotspot.id}
            class:face={hotspot.kind === 'face'}
            class:touchpad={hotspot.kind === 'touchpad'}
            class:trigger={hotspot.kind === 'trigger'}
            class:stick={hotspot.kind === 'stick'}
            class:dpad={hotspot.kind === 'dpad'}
            class:utility={hotspot.kind === 'utility'}
            class:center={hotspot.kind === 'center'}
            class="hotspot"
            data-control={hotspot.id}
            style={`left:${hotspot.x}%;top:${hotspot.y}%;--mapping-accent:${mapping?.accent ?? profile.accent}`}
            aria-label={`${controlNames[hotspot.id]}${mapping ? `, mapped to ${mapping.title}` : ', unassigned'}`}
            aria-pressed={selectedControl === hotspot.id}
            onclick={() => onselect(hotspot.id)}
            ondragover={(event) => dragOver(event, hotspot.id)}
            ondragleave={() => (dragTarget = null)}
            ondrop={(event) => drop(event, hotspot.id)}
          >
            {#if mapping}
              <span class="mapped-icon"
                ><ActionIcon
                  name={mapping.icon}
                  size={hotspot.kind === 'touchpad' ? 18 : 14}
                  strokeWidth={2}
                /></span
              >
              {#if hotspot.kind === 'touchpad'}<span class="touchpad-title">{mapping.title}</span
                >{/if}
              <span class="mapping-pip"></span>
            {:else}
              <span class="control-glyph">{controlShortNames[hotspot.id]}</span>
            {/if}
            <span class="drop-ring">+</span>
          </button>
        {/each}

        {#if mappingCount === 0 && device.connected}
          <div class="empty-hint">
            <MousePointerClick size={15} />
            <span>Drop your first action onto a control</span>
          </div>
        {/if}
      </div>
    </div>

    <div class="selection-strip">
      <span
        class="selection-control"
        style={`--selection-accent:${selectedMapping?.accent ?? profile.accent}`}
      >
        {controlShortNames[selectedControl]}
      </span>
      <span class="selection-copy">
        <small>Selected control</small>
        <strong>{controlNames[selectedControl]}</strong>
      </span>
      <ChevronRight size={14} />
      {#if selectedMapping}
        <span class="selected-action-icon" style={`--selection-accent:${selectedMapping.accent}`}>
          <ActionIcon name={selectedMapping.icon} size={15} />
        </span>
        <span class="selection-copy action-copy">
          <small>{selectedMapping.trigger}</small>
          <strong>{selectedMapping.title}</strong>
        </span>
      {:else}
        <span class="selection-empty"><CircleHelp size={14} /> Unassigned</span>
      {/if}
    </div>
  </section>
</main>

<style>
  .canvas-shell {
    display: grid;
    min-width: 0;
    min-height: 0;
    grid-template-rows: auto 1fr;
    overflow: hidden;
    background:
      radial-gradient(
        circle at 50% 46%,
        color-mix(in srgb, var(--accent) 5%, transparent),
        transparent 38%
      ),
      linear-gradient(var(--canvas-grid) 1px, transparent 1px),
      linear-gradient(90deg, var(--canvas-grid) 1px, transparent 1px), var(--surface-canvas);
    background-size: 28px 28px;
  }

  .canvas-header {
    position: relative;
    z-index: 4;
    display: flex;
    min-height: 62px;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 0 20px;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface-canvas) 92%, transparent);
    backdrop-filter: blur(10px);
  }

  .canvas-header > div:first-child {
    display: grid;
    gap: 2px;
  }

  .section-label {
    color: var(--text-subtle);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    color: var(--text);
    font-size: 14px;
    font-weight: 660;
    letter-spacing: -0.015em;
  }

  .canvas-meta {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .connection-state,
  .mapping-count {
    display: flex;
    height: 26px;
    align-items: center;
    gap: 7px;
    padding: 0 9px;
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text-subtle);
    background: var(--surface-raised);
    font-size: 10px;
    font-weight: 560;
  }

  .connection-state > span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-disabled);
    transition:
      background-color var(--motion-base) var(--ease-standard),
      box-shadow var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .connection-state.online > span {
    background: var(--positive);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--positive) 12%, transparent);
    transform: scale(1.06);
  }

  .controller-stage {
    position: relative;
    display: grid;
    min-height: 0;
    grid-template-rows: 1fr auto;
    place-items: center;
    padding: 30px 18px 18px;
    overflow: auto;
    perspective: 1100px;
  }

  .controller-board {
    position: relative;
    width: min(94%, 820px);
    min-width: 340px;
    aspect-ratio: 1440 / 968;
    transform: translateY(-1.5%);
  }

  .controller-tilt {
    position: absolute;
    inset: 0;
    transform: perspective(1000px) rotateX(var(--tilt-x)) rotateY(var(--tilt-y));
    transform-style: preserve-3d;
    transition: transform var(--motion-slow) var(--ease-emphasized);
    animation: controller-arrive 420ms var(--ease-emphasized) both;
    will-change: transform;
  }

  .controller-aura {
    position: absolute;
    z-index: -1;
    top: 14%;
    right: 13%;
    bottom: 11%;
    left: 13%;
    border-radius: 42%;
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    filter: blur(34px);
    opacity: 0.42;
    transform: translateZ(-20px) scale(0.92);
  }

  .controller-art,
  .controller-specular {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .controller-art {
    z-index: 0;
    object-fit: contain;
    pointer-events: none;
    filter: drop-shadow(0 24px 21px rgb(0 0 0 / 38%));
    transform: translateZ(2px);
    user-select: none;
    transition:
      opacity var(--motion-slow) var(--ease-standard),
      filter var(--motion-slow) var(--ease-standard);
  }

  .controller-specular {
    z-index: 1;
    background: radial-gradient(
      circle at var(--light-x) var(--light-y),
      rgb(255 255 255 / 15%),
      rgb(145 175 248 / 4%) 23%,
      transparent 48%
    );
    mask: var(--controller-mask) center / contain no-repeat;
    pointer-events: none;
    mix-blend-mode: screen;
    opacity: 0.52;
    transform: translateZ(5px);
    transition: opacity var(--motion-base) var(--ease-standard);
  }

  .disconnected .controller-art {
    opacity: 0.68;
    filter: saturate(0.48) brightness(0.84) drop-shadow(0 18px 18px rgb(0 0 0 / 30%));
  }

  .disconnected .controller-aura,
  .disconnected .controller-specular {
    opacity: 0.12;
  }

  .hotspot {
    position: absolute;
    z-index: 2;
    display: grid;
    width: 34px;
    height: 34px;
    padding: 0;
    place-items: center;
    border: 1px solid rgb(255 255 255 / 13%);
    border-radius: 50%;
    color: #a4abb7;
    background: rgb(11 14 19 / 48%);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 5%),
      0 4px 11px rgb(0 0 0 / 30%);
    transform: translate(-50%, -50%) translateZ(18px);
    cursor: pointer;
    transition:
      border-color var(--motion-quick) var(--ease-standard),
      box-shadow var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized),
      color var(--motion-quick) var(--ease-standard),
      background-color var(--motion-quick) var(--ease-standard);
    will-change: transform;
  }

  .hotspot:hover {
    z-index: 4;
    border-color: rgb(255 255 255 / 28%);
    color: #d8dadd;
    background: rgb(28 31 36 / 92%);
    transform: translate(-50%, -50%) translateZ(24px) scale(1.055);
  }

  .hotspot:focus-visible {
    z-index: 5;
    outline: 2px solid var(--focus);
    outline-offset: 3px;
  }

  .hotspot.selected {
    z-index: 3;
    border-color: var(--accent-bright);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--accent) 19%, transparent),
      0 5px 12px rgb(0 0 0 / 36%);
    animation: hotspot-select 220ms var(--ease-emphasized);
  }

  .hotspot.mapped {
    border-color: color-mix(in srgb, var(--mapping-accent) 65%, #202328);
    color: var(--mapping-accent);
    background: color-mix(in srgb, var(--mapping-accent) 15%, #1d2025);
  }

  .hotspot.pressed {
    border-color: var(--positive);
    box-shadow: 0 0 0 5px color-mix(in srgb, var(--positive) 18%, transparent);
    transform: translate(-50%, -50%) translateZ(12px) scale(0.89);
    transition-duration: 70ms;
  }

  .hotspot.dragover {
    z-index: 8;
    border-color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 34%, #1d2025);
    box-shadow: 0 0 0 8px color-mix(in srgb, var(--accent) 18%, transparent);
    transform: translate(-50%, -50%) translateZ(28px) scale(1.13);
  }

  .hotspot.dpad,
  .hotspot.utility {
    width: 32px;
    height: 32px;
  }

  .hotspot.stick {
    width: 62px;
    height: 62px;
    border-color: rgb(255 255 255 / 8%);
    background: rgb(34 37 43 / 86%);
  }

  .hotspot.trigger {
    width: 64px;
    height: 27px;
    border-radius: 9px;
  }

  .hotspot.touchpad {
    width: 32%;
    height: 22%;
    border-radius: 15px;
    background: rgb(18 22 30 / 38%);
  }

  .hotspot.center {
    width: 32px;
    height: 28px;
    border-radius: 10px;
  }

  .face:not(.mapped) .control-glyph,
  .dpad:not(.mapped) .control-glyph {
    opacity: 0;
  }

  .control-glyph {
    overflow: hidden;
    max-width: 100%;
    padding: 0 4px;
    font-size: 10px;
    font-weight: 720;
    letter-spacing: -0.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .face .control-glyph {
    font-size: 17px;
    font-weight: 450;
  }

  .mapped-icon {
    display: grid;
    place-items: center;
    transition: transform var(--motion-base) var(--ease-emphasized);
  }

  .hotspot:hover .mapped-icon {
    transform: scale(1.08);
  }

  .mapping-pip {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 8px;
    height: 8px;
    border: 2px solid #1b1e23;
    border-radius: 50%;
    background: var(--mapping-accent);
  }

  .touchpad-title {
    overflow: hidden;
    max-width: 80%;
    font-size: 10px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .touchpad .mapped-icon {
    position: absolute;
    top: 9px;
  }

  .touchpad .touchpad-title {
    position: absolute;
    bottom: 8px;
  }

  .drop-ring {
    position: absolute;
    display: grid;
    width: 19px;
    height: 19px;
    place-items: center;
    border-radius: 50%;
    color: #11151c;
    background: var(--accent-bright);
    font-size: 14px;
    font-weight: 700;
    opacity: 0;
    transform: scale(0.55);
    visibility: hidden;
    transition:
      opacity var(--motion-quick) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized),
      visibility 0s linear var(--motion-base);
  }

  .dragover > :not(.drop-ring) {
    opacity: 0;
  }

  .dragover .drop-ring {
    opacity: 1;
    transform: scale(1);
    visibility: visible;
    transition-delay: 0s;
  }

  .empty-hint {
    position: absolute;
    bottom: 4%;
    left: 50%;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-subtle);
    background: color-mix(in srgb, var(--surface-raised) 90%, transparent);
    box-shadow: 0 8px 22px rgb(0 0 0 / 20%);
    font-size: 10px;
    white-space: nowrap;
    transform: translateX(-50%);
    animation: hint-arrive 300ms var(--ease-emphasized) 120ms both;
  }

  .connection-callout {
    position: absolute;
    z-index: 12;
    top: 18px;
    left: 50%;
    display: grid;
    width: min(440px, calc(100% - 36px));
    min-height: 54px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 8px 9px;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: color-mix(in srgb, var(--surface-raised) 94%, transparent);
    box-shadow: 0 12px 30px rgb(0 0 0 / 28%);
    backdrop-filter: blur(12px);
    transform: translateX(-50%);
    animation: callout-arrive var(--motion-slow) var(--ease-emphasized) both;
  }

  .callout-icon {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border-radius: 8px;
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 11%, var(--surface-high));
  }

  .connection-callout > span:nth-child(2) {
    display: grid;
    gap: 2px;
  }

  .connection-callout strong {
    color: var(--text);
    font-size: 10px;
    font-weight: 650;
  }

  .connection-callout small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .connection-callout button {
    display: flex;
    height: 30px;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    border-radius: 7px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 9%, var(--surface-high));
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
  }

  .selection-strip {
    display: grid;
    width: min(520px, calc(100% - 20px));
    min-height: 54px;
    grid-template-columns: auto 1fr auto auto 1fr;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 11px;
    background: color-mix(in srgb, var(--surface-raised) 90%, transparent);
    box-shadow: 0 10px 28px rgb(0 0 0 / 18%);
    backdrop-filter: blur(12px);
    transition:
      border-color var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized),
      box-shadow var(--motion-base) var(--ease-standard);
  }

  .selection-strip:hover {
    border-color: var(--border-strong);
    box-shadow: 0 14px 34px rgb(0 0 0 / 23%);
    transform: translateY(-1px);
  }

  .selection-control,
  .selected-action-icon {
    display: grid;
    width: 35px;
    height: 35px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--selection-accent) 48%, var(--border));
    border-radius: 9px;
    color: var(--selection-accent);
    background: color-mix(in srgb, var(--selection-accent) 10%, var(--surface-high));
    font-size: 10px;
    font-weight: 720;
    transition:
      border-color var(--motion-base) var(--ease-standard),
      background-color var(--motion-base) var(--ease-standard),
      color var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .selection-strip:hover .selection-control,
  .selection-strip:hover .selected-action-icon {
    transform: translateY(-1px) scale(1.03);
  }

  @keyframes controller-arrive {
    from {
      opacity: 0;
      transform: perspective(1000px) translateY(10px) scale(0.975);
    }
  }

  @keyframes hotspot-select {
    from {
      box-shadow:
        0 0 0 10px color-mix(in srgb, var(--accent) 0%, transparent),
        0 5px 12px rgb(0 0 0 / 28%);
    }
  }

  @keyframes hint-arrive {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
  }

  @keyframes callout-arrive {
    from {
      opacity: 0;
      transform: translate(-50%, -6px) scale(0.985);
    }
  }

  .selection-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .selection-copy small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .selection-copy strong {
    overflow: hidden;
    color: var(--text);
    font-size: 10px;
    font-weight: 620;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .selection-empty {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-subtle);
    font-size: 10px;
  }

  @media (max-height: 780px) {
    .controller-stage {
      padding-top: 14px;
    }

    .controller-board {
      width: min(90%, 660px);
    }
  }
</style>
