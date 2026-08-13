<script lang="ts">
  import { Check, Info, TriangleAlert, X } from '@lucide/svelte';
  import type { ToastMessage } from '../models';

  interface Props {
    toasts: ToastMessage[];
    ondismiss: (id: number) => void;
  }

  let { toasts, ondismiss }: Props = $props();
</script>

<div class="toast-region" aria-live="polite" aria-label="Notifications">
  {#each toasts as toast (toast.id)}
    <article
      class:success={toast.tone === 'success'}
      class:warning={toast.tone === 'warning'}
      class="toast"
    >
      <span class="toast-icon" aria-hidden="true">
        {#if toast.tone === 'success'}
          <Check size={15} strokeWidth={2.2} />
        {:else if toast.tone === 'warning'}
          <TriangleAlert size={15} strokeWidth={2} />
        {:else}
          <Info size={15} strokeWidth={2} />
        {/if}
      </span>
      <span class="toast-copy">
        <strong>{toast.title}</strong>
        {#if toast.detail}<small>{toast.detail}</small>{/if}
      </span>
      <button type="button" aria-label="Dismiss notification" onclick={() => ondismiss(toast.id)}>
        <X size={15} />
      </button>
    </article>
  {/each}
</div>

<style>
  .toast-region {
    position: fixed;
    z-index: 100;
    right: 18px;
    bottom: 18px;
    display: grid;
    width: min(360px, calc(100vw - 36px));
    gap: 8px;
    pointer-events: none;
  }

  .toast {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    min-height: 54px;
    gap: 11px;
    padding: 10px 10px 10px 12px;
    border: 1px solid var(--border-strong);
    border-radius: 11px;
    background: color-mix(in srgb, var(--surface-raised) 94%, transparent);
    box-shadow: 0 14px 35px rgb(0 0 0 / 38%);
    backdrop-filter: blur(14px);
    pointer-events: auto;
    animation: toast-in 220ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  .toast-icon {
    display: grid;
    width: 26px;
    height: 26px;
    place-items: center;
    border-radius: 7px;
    color: var(--accent-bright);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .toast.success .toast-icon {
    color: var(--positive);
    background: color-mix(in srgb, var(--positive) 14%, transparent);
  }

  .toast.warning .toast-icon {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 14%, transparent);
  }

  .toast-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  strong {
    overflow: hidden;
    color: var(--text);
    font-size: 12px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  small {
    overflow: hidden;
    color: var(--text-muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button {
    display: grid;
    width: 28px;
    height: 28px;
    padding: 0;
    place-items: center;
    border: 0;
    border-radius: 7px;
    color: var(--text-subtle);
    background: transparent;
    cursor: pointer;
  }

  button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
</style>
