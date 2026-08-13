<script lang="ts">
  import { Check, Copy, Layers, Plus, Trash2 } from '@lucide/svelte';
  import type { Profile } from '../models';

  interface Props {
    profiles: Profile[];
    activeProfileId: string;
    onactivate: (id: string) => void;
    oncreate: () => void;
    onduplicate: (id: string) => void;
    ondelete: (id: string) => void;
    onupdate: (id: string, changes: Partial<Profile>) => void;
  }

  let { profiles, activeProfileId, onactivate, oncreate, onduplicate, ondelete, onupdate }: Props =
    $props();
</script>

<main class="page-shell">
  <div class="page-heading">
    <div>
      <span class="eyebrow">Workspace</span>
      <h1>Profiles</h1>
      <p>Keep separate controller layouts for different apps and routines.</p>
    </div>
    <button type="button" class="primary-button" onclick={oncreate}
      ><Plus size={15} /> New profile</button
    >
  </div>

  <section class="profile-grid" aria-label="Controller profiles">
    {#each profiles as profile (profile.id)}
      <article
        class:active={profile.id === activeProfileId}
        style={`--profile-accent:${profile.accent}`}
      >
        <div class="card-topline">
          <span class="profile-icon"><Layers size={18} /></span>
          {#if profile.id === activeProfileId}
            <span class="active-label"><Check size={12} /> Active</span>
          {:else}
            <button class="activate-button" type="button" onclick={() => onactivate(profile.id)}
              >Activate</button
            >
          {/if}
        </div>

        <label class="name-field">
          <span>Profile name</span>
          <input
            value={profile.name}
            oninput={(event) => onupdate(profile.id, { name: event.currentTarget.value })}
          />
        </label>

        <div class="profile-stats">
          <span><strong>{Object.keys(profile.mappings).length}</strong> mappings</span>
        </div>

        <footer>
          <button type="button" onclick={() => onduplicate(profile.id)}
            ><Copy size={14} /> Duplicate</button
          >
          <button
            type="button"
            class="delete-button"
            disabled={profiles.length === 1}
            aria-label={`Delete ${profile.name}`}
            onclick={() => ondelete(profile.id)}><Trash2 size={14} /></button
          >
        </footer>
      </article>
    {/each}

    <button type="button" class="new-card" onclick={oncreate}>
      <span><Plus size={20} /></span>
      <strong>Create another profile</strong>
      <small>Start with an empty controller layout</small>
    </button>
  </section>
</main>

<style>
  .page-shell {
    min-height: 0;
    padding: 32px clamp(28px, 5vw, 72px) 48px;
    overflow: auto;
    background:
      linear-gradient(var(--canvas-grid) 1px, transparent 1px),
      linear-gradient(90deg, var(--canvas-grid) 1px, transparent 1px), var(--surface-canvas);
    background-size: 28px 28px;
  }

  .page-heading {
    display: flex;
    max-width: 1120px;
    align-items: end;
    justify-content: space-between;
    gap: 30px;
    margin: 0 auto 28px;
  }

  .eyebrow {
    color: var(--accent-bright);
    font-size: 10px;
    font-weight: 720;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  h1 {
    margin: 7px 0 6px;
    color: var(--text);
    font-size: 24px;
    font-weight: 690;
    letter-spacing: -0.035em;
  }

  p {
    margin: 0;
    color: var(--text-subtle);
    font-size: 11px;
  }

  .primary-button {
    display: flex;
    height: 36px;
    align-items: center;
    gap: 7px;
    padding: 0 13px;
    border: 1px solid color-mix(in srgb, var(--accent) 58%, var(--border));
    border-radius: 8px;
    color: #eef3ff;
    background: color-mix(in srgb, var(--accent) 65%, #222936);
    box-shadow: 0 5px 16px color-mix(in srgb, var(--accent) 13%, transparent);
    font-size: 10px;
    font-weight: 650;
    cursor: pointer;
  }

  .primary-button:hover {
    background: color-mix(in srgb, var(--accent) 75%, #222936);
  }

  .profile-grid {
    display: grid;
    max-width: 1120px;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 13px;
    margin: 0 auto;
  }

  article,
  .new-card {
    min-height: 214px;
    border: 1px solid var(--border);
    border-radius: 13px;
    background: color-mix(in srgb, var(--surface-panel) 96%, transparent);
    box-shadow: 0 12px 30px rgb(0 0 0 / 10%);
  }

  article {
    position: relative;
    display: grid;
    grid-template-rows: auto auto auto 1fr;
    gap: 14px;
    padding: 15px;
    overflow: hidden;
  }

  article::before {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    height: 2px;
    background: transparent;
    content: '';
  }

  article.active {
    border-color: color-mix(in srgb, var(--profile-accent) 42%, var(--border));
  }

  article.active::before {
    background: var(--profile-accent);
  }

  .card-topline,
  .profile-stats,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .profile-icon {
    display: grid;
    width: 36px;
    height: 36px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--profile-accent) 28%, var(--border));
    border-radius: 9px;
    color: var(--profile-accent);
    background: color-mix(in srgb, var(--profile-accent) 9%, var(--surface-high));
  }

  .active-label {
    display: flex;
    height: 24px;
    align-items: center;
    gap: 5px;
    padding: 0 7px;
    border-radius: 6px;
    color: var(--positive);
    background: color-mix(in srgb, var(--positive) 9%, var(--surface-high));
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .activate-button {
    height: 27px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-muted);
    background: var(--surface-high);
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
  }

  .activate-button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .name-field {
    display: grid;
    gap: 5px;
  }

  .name-field span {
    color: var(--text-subtle);
    font-size: 10px;
    font-weight: 640;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  input {
    width: 100%;
    height: 33px;
    padding: 0 9px;
    border: 1px solid var(--border);
    border-radius: 7px;
    outline: 0;
    color: var(--text);
    background: var(--surface-base);
    font-size: 10px;
  }

  .name-field input {
    height: 35px;
    padding: 0;
    border: 0;
    border-bottom: 1px solid transparent;
    border-radius: 0;
    background: transparent;
    font-size: 16px;
    font-weight: 650;
    letter-spacing: -0.025em;
  }

  input:focus {
    border-color: color-mix(in srgb, var(--profile-accent) 58%, var(--border));
  }

  .name-field input:focus {
    border-bottom-color: var(--profile-accent);
  }

  input::placeholder {
    color: var(--text-disabled);
  }

  .profile-stats {
    justify-content: flex-start;
    gap: 18px;
    padding: 8px 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  .profile-stats span {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .profile-stats strong {
    color: var(--text-muted);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  footer {
    align-self: end;
    padding-top: 10px;
  }

  footer button {
    display: flex;
    height: 28px;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-subtle);
    background: var(--surface-high);
    font-size: 10px;
    cursor: pointer;
  }

  footer button:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--text);
  }

  footer .delete-button {
    width: 29px;
    padding: 0;
    justify-content: center;
  }

  footer .delete-button:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--danger) 40%, var(--border));
    color: var(--danger);
  }

  footer button:disabled {
    cursor: not-allowed;
    opacity: 0.35;
  }

  .new-card {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 7px;
    border-style: dashed;
    color: var(--text-subtle);
    cursor: pointer;
  }

  .new-card:hover {
    border-color: color-mix(in srgb, var(--accent) 42%, var(--border));
    background: color-mix(in srgb, var(--accent) 5%, var(--surface-panel));
  }

  .new-card > span {
    display: grid;
    width: 40px;
    height: 40px;
    margin-bottom: 3px;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    color: var(--accent-bright);
    background: var(--surface-high);
  }

  .new-card strong {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 620;
  }

  .new-card small {
    font-size: 10px;
  }

  @media (max-width: 1180px) {
    .profile-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
