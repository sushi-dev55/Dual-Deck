<script lang="ts">
  import {
    Battery,
    Cable,
    ChevronDown,
    Gamepad2,
    Layers,
    Pause,
    PanelsTopLeft,
    Play,
    RefreshCw,
    Settings2,
    Unplug,
    Usb,
    X,
  } from '@lucide/svelte';
  import type { ControllerDevice, Profile, WorkspaceView } from '../models';
  import { formatBatteryStatus, formatCompactBatteryStatus } from '../services/battery';

  interface Props {
    profiles: Profile[];
    activeProfileId: string;
    view: WorkspaceView;
    device: ControllerDevice;
    mappingsPaused: boolean;
    onviewchange: (view: WorkspaceView) => void;
    onprofilechange: (id: string) => void;
    onrefreshdevice: () => void;
    onpausetoggle: () => void;
  }

  let {
    profiles,
    activeProfileId,
    view,
    device,
    mappingsPaused,
    onviewchange,
    onprofilechange,
    onrefreshdevice,
    onpausetoggle,
  }: Props = $props();
  let deviceOpen = $state(false);
  let deviceMenu: HTMLDivElement;

  function handleWindowClick(event: MouseEvent) {
    if (!deviceOpen || !(event.target instanceof Node) || deviceMenu.contains(event.target)) return;
    deviceOpen = false;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') deviceOpen = false;
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<header class="topbar">
  <div class="brand" aria-label="Dual Deck">
    <span class="brand-mark" aria-hidden="true"><Gamepad2 size={18} strokeWidth={2.1} /></span>
    <span>Dual Deck</span>
  </div>

  <nav aria-label="Main navigation">
    <button class:active={view === 'editor'} type="button" onclick={() => onviewchange('editor')}>
      <PanelsTopLeft size={15} />
      Editor
    </button>
    <button
      class:active={view === 'profiles'}
      type="button"
      onclick={() => onviewchange('profiles')}
    >
      <Layers size={15} />
      Profiles
    </button>
    <button
      class:active={view === 'settings'}
      type="button"
      onclick={() => onviewchange('settings')}
    >
      <Settings2 size={15} />
      Settings
    </button>
  </nav>

  <div class="top-actions">
    <button
      type="button"
      class:paused={mappingsPaused}
      class="pause-mappings"
      aria-pressed={mappingsPaused}
      onclick={onpausetoggle}
    >
      {#if mappingsPaused}<Play size={14} /> Resume mappings{:else}<Pause size={14} /> Pause mappings{/if}
    </button>

    <label class="profile-switcher" aria-label="Active profile">
      <span
        class="profile-dot"
        style={`background:${profiles.find((profile) => profile.id === activeProfileId)?.accent ?? '#84a7ff'}`}
      ></span>
      <select
        value={activeProfileId}
        onchange={(event) => onprofilechange(event.currentTarget.value)}
      >
        {#each profiles as profile}
          <option value={profile.id}>{profile.name}</option>
        {/each}
      </select>
      <ChevronDown size={14} aria-hidden="true" />
    </label>

    <div bind:this={deviceMenu} class="device-menu">
      <button
        type="button"
        class:connected={device.connected}
        class="device-trigger"
        aria-expanded={deviceOpen}
        aria-label={device.connected
          ? `${device.name}, connected, ${formatBatteryStatus(device)}`
          : 'Controller disconnected'}
        onclick={() => (deviceOpen = !deviceOpen)}
      >
        <span class="status-dot"></span>
        <span>{device.connected ? 'DualSense' : 'No controller'}</span>
        {#if device.connected}
          <span class="battery">{formatCompactBatteryStatus(device)}</span>
        {/if}
        <span class:open={deviceOpen} class="device-chevron"><ChevronDown size={14} /></span>
      </button>

      {#if deviceOpen}
        <div class="device-popover">
          <div class="popover-title">
            <span class:online={device.connected} class="device-picture">
              {#if device.connected}<Gamepad2 size={23} />{:else}<Unplug size={21} />{/if}
            </span>
            <span>
              <strong>{device.connected ? device.name : 'Controller not found'}</strong>
              <small
                >{device.connected
                  ? `${device.connection} connection`
                  : 'Connect a DualSense with USB'}</small
              >
            </span>
            <button
              type="button"
              class="close-popover"
              aria-label="Close controller details"
              onclick={() => (deviceOpen = false)}
            >
              <X size={15} />
            </button>
          </div>
          {#if device.connected}
            <div class="device-stats">
              <span><Usb size={14} /> {device.connection}</span>
              <span><Battery size={14} /> {formatBatteryStatus(device)}</span>
            </div>
            <button
              class="popover-action"
              type="button"
              onclick={() => {
                onrefreshdevice();
                deviceOpen = false;
              }}
            >
              <RefreshCw size={15} />
              Refresh status
            </button>
          {:else}
            <button
              class="popover-action primary"
              type="button"
              onclick={() => {
                onrefreshdevice();
                deviceOpen = false;
              }}
            >
              <Cable size={15} />
              Scan for controller
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</header>

<style>
  .topbar {
    position: relative;
    z-index: 30;
    display: grid;
    grid-template-columns: 260px 1fr auto;
    height: 60px;
    align-items: center;
    border-bottom: 1px solid var(--border);
    background: var(--surface-base);
    user-select: none;
  }

  .brand {
    display: flex;
    height: 100%;
    align-items: center;
    gap: 10px;
    padding: 0 18px;
    color: var(--text);
    font-size: 14px;
    font-weight: 720;
    letter-spacing: -0.015em;
  }

  .brand-mark {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 36%, var(--border));
    border-radius: 9px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface-raised));
    transition:
      border-color var(--motion-base) var(--ease-standard),
      background-color var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .brand:hover .brand-mark {
    border-color: color-mix(in srgb, var(--accent) 58%, var(--border));
    background: color-mix(in srgb, var(--accent) 18%, var(--surface-raised));
    transform: translateY(-1px) rotate(-2deg);
  }

  nav {
    display: flex;
    height: 100%;
    align-items: center;
    justify-content: center;
    gap: 3px;
  }

  nav button {
    position: relative;
    display: flex;
    height: 34px;
    align-items: center;
    gap: 7px;
    padding: 0 12px;
    border: 0;
    border-radius: 8px;
    color: var(--text-muted);
    background: transparent;
    font-size: 12px;
    font-weight: 580;
    cursor: pointer;
    transition:
      color var(--motion-quick) var(--ease-standard),
      background-color var(--motion-quick) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  nav button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  nav button.active {
    color: var(--text);
    background: var(--surface-high);
  }

  nav button::after {
    position: absolute;
    right: 12px;
    bottom: -14px;
    left: 12px;
    height: 2px;
    border-radius: 2px 2px 0 0;
    background: var(--accent-bright);
    content: '';
    opacity: 0;
    transform: scaleX(0.35);
    transform-origin: center;
    transition:
      opacity var(--motion-base) var(--ease-standard),
      transform var(--motion-slow) var(--ease-emphasized);
  }

  nav button.active::after {
    opacity: 1;
    transform: scaleX(1);
  }

  nav button:active {
    transform: scale(0.97);
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 14px;
  }

  .profile-switcher,
  .device-trigger {
    display: flex;
    height: 34px;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-raised);
    transition:
      border-color var(--motion-quick) var(--ease-standard),
      background-color var(--motion-quick) var(--ease-standard),
      color var(--motion-quick) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized),
      box-shadow var(--motion-base) var(--ease-standard);
  }

  .pause-mappings {
    display: flex;
    height: 34px;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-muted);
    background: var(--surface-raised);
    font-size: 10px;
    font-weight: 620;
    cursor: pointer;
    transition:
      border-color var(--motion-quick) var(--ease-standard),
      background-color var(--motion-quick) var(--ease-standard),
      color var(--motion-quick) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .pause-mappings:hover {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--surface-hover);
    transform: translateY(-1px);
  }

  .pause-mappings.paused {
    border-color: color-mix(in srgb, var(--warning) 38%, var(--border));
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 8%, var(--surface-raised));
  }

  .pause-mappings:active {
    transform: scale(0.97);
  }

  .profile-switcher {
    position: relative;
    padding: 0 8px 0 10px;
  }

  .profile-switcher:hover,
  .device-trigger:hover {
    box-shadow: 0 7px 18px rgb(0 0 0 / 18%);
    transform: translateY(-1px);
  }

  .profile-switcher select {
    max-width: 126px;
    height: 100%;
    padding: 0 4px 0 0;
    border: 0;
    outline: 0;
    color: var(--text);
    background: transparent;
    font-size: 12px;
    font-weight: 600;
    appearance: none;
    cursor: pointer;
  }

  .profile-switcher option {
    color: var(--text);
    background: var(--surface-raised);
  }

  .profile-dot,
  .status-dot {
    width: 7px;
    height: 7px;
    flex: 0 0 auto;
    border-radius: 50%;
    transition:
      background-color var(--motion-base) var(--ease-standard),
      box-shadow var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .device-menu {
    position: relative;
  }

  .device-trigger {
    min-width: 154px;
    justify-content: flex-start;
    padding: 0 9px 0 11px;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
  }

  .device-trigger:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .status-dot {
    background: var(--text-disabled);
  }

  .connected .status-dot {
    background: var(--positive);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--positive) 12%, transparent);
    transform: scale(1.06);
  }

  .battery {
    margin-left: auto;
    color: var(--text-subtle);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .device-chevron {
    display: grid;
    margin-left: 1px;
    place-items: center;
    transform: rotate(0deg);
    transition: transform var(--motion-base) var(--ease-emphasized);
  }

  .device-chevron.open {
    transform: rotate(180deg);
  }

  .device-popover {
    position: absolute;
    top: calc(100% + 9px);
    right: 0;
    width: 290px;
    padding: 10px;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    background: var(--surface-overlay);
    box-shadow: var(--shadow-large);
    animation: popover-in var(--motion-base) var(--ease-emphasized) both;
  }

  .popover-title {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 3px 2px 10px;
  }

  .popover-title > span:nth-child(2) {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .popover-title strong {
    overflow: hidden;
    font-size: 12px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .popover-title small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .device-picture {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text-subtle);
    background: var(--surface-high);
  }

  .device-picture.online {
    color: var(--accent-bright);
  }

  .close-popover {
    display: grid;
    width: 27px;
    height: 27px;
    padding: 0;
    place-items: center;
    border: 0;
    border-radius: 7px;
    color: var(--text-subtle);
    background: transparent;
    cursor: pointer;
  }

  .close-popover:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .device-stats {
    display: flex;
    gap: 6px;
    padding: 8px 0 10px;
    border-top: 1px solid var(--border);
  }

  .device-stats span {
    display: flex;
    height: 27px;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border-radius: 7px;
    color: var(--text-muted);
    background: var(--surface-high);
    font-size: 10px;
  }

  .popover-action {
    display: flex;
    width: 100%;
    height: 34px;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-muted);
    background: var(--surface-raised);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .popover-action:hover {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--surface-hover);
  }

  .popover-action.primary {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface-raised));
  }

  @keyframes popover-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.985);
    }
  }

  @media (max-width: 1220px) {
    .topbar {
      grid-template-columns: 230px 1fr auto;
    }

    .profile-switcher select {
      max-width: 92px;
    }
  }
</style>
