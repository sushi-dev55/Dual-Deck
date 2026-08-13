<script lang="ts">
  import {
    Bell,
    Cable,
    Gamepad2,
    Info,
    MonitorUp,
    Power,
    Settings2,
    ShieldCheck,
  } from '@lucide/svelte';
  import type { ControllerDevice, Preferences } from '../models';
  import { formatBatteryStatus } from '../services/battery';
  import Toggle from './Toggle.svelte';

  type SettingsSection = 'general' | 'controller' | 'about';

  interface Props {
    appVersion: string;
    preferences: Preferences;
    device: ControllerDevice;
    onpreference: <K extends keyof Preferences>(key: K, value: Preferences[K]) => void;
    onscan: () => void;
  }

  let { appVersion, preferences, device, onpreference, onscan }: Props = $props();
  let section = $state<SettingsSection>('general');
  const navigation = [
    { id: 'general' as const, label: 'General', icon: Settings2 },
    { id: 'controller' as const, label: 'Controller', icon: Gamepad2 },
    { id: 'about' as const, label: 'About', icon: Info },
  ];
</script>

<main class="settings-shell">
  <aside class="settings-nav">
    <div>
      <span class="eyebrow">Dual Deck</span>
      <h1>Settings</h1>
    </div>
    <nav aria-label="Settings sections">
      {#each navigation as item}
        <button
          type="button"
          class:active={section === item.id}
          onclick={() => (section = item.id)}
        >
          <item.icon size={15} />
          {item.label}
        </button>
      {/each}
    </nav>
    <div class="settings-version">
      <span>Version {appVersion}</span>
      <small>Windows desktop</small>
    </div>
  </aside>

  <div class="settings-content">
    {#if section === 'general'}
      <div class="content-heading">
        <h2>General</h2>
        <p>Choose how Dual Deck starts, closes, and stays available.</p>
      </div>

      <section class="settings-card">
        <div class="card-heading">
          <span class="card-icon"><Power size={16} /></span><span
            ><strong>Startup</strong><small>Windows sign-in behavior</small></span
          >
        </div>
        <div class="setting-row">
          <span
            ><strong>Start with Windows</strong><small
              >Keep controller mappings ready after sign-in</small
            ></span
          >
          <Toggle
            checked={preferences.startWithWindows}
            label="Start with Windows"
            onchange={(value) => onpreference('startWithWindows', value)}
          />
        </div>
        <div class="setting-row">
          <span
            ><strong>Launch minimized</strong><small>Start directly in the notification area</small
            ></span
          >
          <Toggle
            checked={preferences.launchMinimized}
            label="Launch minimized"
            onchange={(value) => onpreference('launchMinimized', value)}
          />
        </div>
      </section>

      <section class="settings-card">
        <div class="card-heading">
          <span class="card-icon"><MonitorUp size={16} /></span><span
            ><strong>Window behavior</strong><small>Keep Dual Deck out of the way</small></span
          >
        </div>
        <div class="setting-row">
          <span
            ><strong>Minimize to notification area</strong><small
              >Continue running mappings when minimized</small
            ></span
          >
          <Toggle
            checked={preferences.minimizeToTray}
            label="Minimize to notification area"
            onchange={(value) => onpreference('minimizeToTray', value)}
          />
        </div>
        <div class="setting-row">
          <span
            ><strong>Close to notification area</strong><small
              >Closing the window keeps mappings active</small
            ></span
          >
          <Toggle
            checked={preferences.closeToTray}
            label="Close to notification area"
            onchange={(value) => onpreference('closeToTray', value)}
          />
        </div>
      </section>

      <section class="settings-card">
        <div class="card-heading">
          <span class="card-icon"><Bell size={16} /></span><span
            ><strong>Feedback</strong><small>Interface responses</small></span
          >
        </div>
        <div class="setting-row">
          <span
            ><strong>Action notifications</strong><small
              >Show a compact notice after an action runs</small
            ></span
          >
          <Toggle
            checked={preferences.actionToasts}
            label="Action notifications"
            onchange={(value) => onpreference('actionToasts', value)}
          />
        </div>
        <div class="setting-row">
          <span
            ><strong>Reduce interface motion</strong><small
              >Limit transitions and animated feedback</small
            ></span
          >
          <Toggle
            checked={preferences.reducedMotion}
            label="Reduce interface motion"
            onchange={(value) => onpreference('reducedMotion', value)}
          />
        </div>
      </section>
    {:else if section === 'controller'}
      <div class="content-heading">
        <h2>Controller</h2>
        <p>Connection details and input behavior for your DualSense.</p>
      </div>

      <section class="device-card">
        <span class:online={device.connected} class="device-art"><Gamepad2 size={32} /></span>
        <span class="device-copy">
          <small>{device.connected ? 'Connected controller' : 'No controller connected'}</small>
          <strong>{device.connected ? device.name : 'DualSense Wireless Controller'}</strong>
          <em
            >{device.connected
              ? `${device.connection} · ${formatBatteryStatus(device)}`
              : 'Connect with USB for the first setup'}</em
          >
        </span>
        <button type="button" onclick={onscan}
          ><Cable size={15} /> {device.connected ? 'Rescan' : 'Scan for controller'}</button
        >
      </section>

      <section class="settings-card">
        <div class="card-heading">
          <span class="card-icon"><Gamepad2 size={16} /></span><span
            ><strong>Input</strong><small>How mapped controls behave</small></span
          >
        </div>
        <div class="setting-row static-row">
          <span
            ><strong>Input mode</strong><small
              >Mapped button presses also reach the active app</small
            ></span
          >
          <span class="setting-value">Pass through</span>
        </div>
      </section>

      <div class="info-banner">
        <ShieldCheck size={16} /><span
          ><strong>No virtual controller driver required</strong><small
            >Pass-through mode avoids controller hiding and remains compatible with games.</small
          ></span
        >
      </div>
    {:else}
      <div class="content-heading">
        <h2>About Dual Deck</h2>
        <p>A focused Windows utility for turning a DualSense into an action deck.</p>
      </div>

      <section class="about-card">
        <span class="about-mark"><Gamepad2 size={28} /></span>
        <div><strong>Dual Deck</strong><small>Version {appVersion}</small></div>
        <p>
          Open-source software licensed under the MIT License. Profiles and preferences stay on this
          computer.
        </p>
        <div class="about-details">
          <span><ShieldCheck size={14} /> No telemetry</span>
          <span><Info size={14} /> Controller photo: Sonson2 · CC BY-SA 4.0</span>
        </div>
      </section>
    {/if}
  </div>
</main>

<style>
  .settings-shell {
    display: grid;
    min-height: 0;
    grid-template-columns: 232px 1fr;
    background: var(--surface-canvas);
  }

  .settings-nav {
    display: grid;
    min-height: 0;
    grid-template-rows: auto 1fr auto;
    padding: 25px 12px 14px;
    border-right: 1px solid var(--border);
    background: var(--surface-panel);
  }

  .settings-nav > div:first-child {
    padding: 0 9px 20px;
  }

  .eyebrow {
    color: var(--accent-bright);
    font-size: 10px;
    font-weight: 720;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  h1 {
    margin: 6px 0 0;
    color: var(--text);
    font-size: 20px;
    letter-spacing: -0.03em;
  }

  nav {
    display: grid;
    align-content: start;
    gap: 3px;
  }

  nav button {
    position: relative;
    display: flex;
    width: 100%;
    height: 36px;
    align-items: center;
    gap: 9px;
    padding: 0 10px;
    border: 0;
    border-radius: 8px;
    color: var(--text-muted);
    background: transparent;
    font-size: 10px;
    font-weight: 570;
    cursor: pointer;
  }

  nav button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  nav button.active {
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 9%, var(--surface-high));
  }

  nav button.active::before {
    position: absolute;
    left: 0;
    width: 2px;
    height: 17px;
    border-radius: 2px;
    background: var(--accent-bright);
    content: '';
  }

  .settings-version {
    display: grid;
    gap: 2px;
    padding: 11px 9px 2px;
    border-top: 1px solid var(--border);
  }

  .settings-version span {
    color: var(--text-muted);
    font-size: 10px;
  }

  .settings-version small {
    color: var(--text-disabled);
    font-size: 10px;
  }

  .settings-content {
    width: min(780px, calc(100% - 60px));
    min-height: 0;
    margin: 0 auto;
    padding: 32px 0 60px;
    overflow: auto;
  }

  .content-heading {
    margin-bottom: 23px;
  }

  .content-heading h2 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 22px;
    font-weight: 680;
    letter-spacing: -0.035em;
  }

  .content-heading p {
    margin: 0;
    color: var(--text-subtle);
    font-size: 10px;
  }

  .settings-card,
  .device-card,
  .about-card {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface-panel);
  }

  .settings-card + .settings-card {
    margin-top: 12px;
  }

  .card-heading {
    display: grid;
    min-height: 59px;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
  }

  .card-icon {
    display: grid;
    width: 31px;
    height: 31px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 8%, var(--surface-high));
  }

  .card-heading > span:nth-child(2),
  .setting-row > span:first-child,
  .device-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .card-heading strong,
  .setting-row strong {
    color: var(--text);
    font-size: 10px;
    font-weight: 620;
  }

  .card-heading small,
  .setting-row small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .setting-row {
    display: grid;
    min-height: 58px;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 20px;
    padding: 8px 14px;
  }

  .setting-row + .setting-row {
    border-top: 1px solid var(--border);
  }

  .setting-value {
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-muted);
    background: var(--surface-high);
    font-size: 10px;
    font-weight: 650;
  }

  .device-card {
    display: grid;
    min-height: 88px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 13px;
    margin-bottom: 12px;
    padding: 13px;
  }

  .device-art {
    display: grid;
    width: 56px;
    height: 56px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 13px;
    color: var(--text-subtle);
    background: var(--surface-high);
  }

  .device-art.online {
    border-color: color-mix(in srgb, var(--positive) 30%, var(--border));
    color: var(--positive);
  }

  .device-copy small {
    color: var(--text-subtle);
    font-size: 10px;
    text-transform: uppercase;
  }

  .device-copy strong {
    color: var(--text);
    font-size: 12px;
    font-weight: 640;
  }

  .device-copy em {
    color: var(--text-subtle);
    font-size: 10px;
    font-style: normal;
  }

  .device-card button {
    display: flex;
    height: 32px;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border: 1px solid color-mix(in srgb, var(--accent) 42%, var(--border));
    border-radius: 7px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 9%, var(--surface-high));
    font-size: 10px;
    font-weight: 620;
    cursor: pointer;
  }

  .info-banner {
    display: flex;
    min-height: 54px;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding: 10px 13px;
    border: 1px solid color-mix(in srgb, var(--positive) 20%, var(--border));
    border-radius: 10px;
    color: var(--positive);
    background: color-mix(in srgb, var(--positive) 5%, var(--surface-panel));
  }

  .info-banner span {
    display: grid;
    gap: 2px;
  }

  .info-banner strong {
    color: var(--text-muted);
    font-size: 10px;
  }

  .info-banner small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .about-card {
    display: grid;
    justify-items: center;
    padding: 32px;
    text-align: center;
  }

  .about-mark {
    display: grid;
    width: 58px;
    height: 58px;
    margin-bottom: 13px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--border));
    border-radius: 15px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface-high));
  }

  .about-card > div:nth-child(2) {
    display: grid;
    gap: 2px;
  }

  .about-card strong {
    color: var(--text);
    font-size: 16px;
  }

  .about-card small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .about-card p {
    max-width: 450px;
    margin: 18px 0;
    color: var(--text-subtle);
    font-size: 10px;
    line-height: 1.6;
  }

  .about-details {
    display: flex;
    gap: 7px;
  }

  .about-details span {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text-muted);
    background: var(--surface-high);
    font-size: 10px;
  }
</style>
