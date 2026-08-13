<script lang="ts">
  import { TriangleAlert } from '@lucide/svelte';
  import { onMount, tick } from 'svelte';

  interface Props {
    title: string;
    description: string;
    confirmLabel: string;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let { title, description, confirmLabel, onconfirm, oncancel }: Props = $props();
  let dialogElement: HTMLDialogElement;
  let cancelButton: HTMLButtonElement;

  onMount(async () => {
    dialogElement.showModal();
    await tick();
    cancelButton.focus();
  });

  function handleCancel(event: Event) {
    event.preventDefault();
    oncancel();
  }
</script>

<dialog
  bind:this={dialogElement}
  class="dialog-shell"
  role="alertdialog"
  aria-labelledby="confirmation-title"
  aria-describedby="confirmation-description"
  oncancel={handleCancel}
>
  <div class="dialog-card">
    <span class="dialog-icon" aria-hidden="true"><TriangleAlert size={20} /></span>
    <div class="dialog-copy">
      <h2 id="confirmation-title">{title}</h2>
      <p id="confirmation-description">{description}</p>
    </div>
    <div class="dialog-actions">
      <button bind:this={cancelButton} type="button" class="cancel" onclick={oncancel}
        >Cancel</button
      >
      <button type="button" class="confirm" onclick={onconfirm}>{confirmLabel}</button>
    </div>
  </div>
</dialog>

<style>
  .dialog-shell {
    width: 100%;
    max-width: none;
    height: 100%;
    max-height: none;
    margin: 0;
    border: 0;
    color: inherit;
    background: transparent;
  }

  .dialog-shell[open] {
    display: grid;
    padding: 24px;
    place-items: center;
  }

  .dialog-shell::backdrop {
    background: rgb(4 6 9 / 68%);
    backdrop-filter: blur(5px);
  }

  .dialog-card {
    display: grid;
    width: min(420px, 100%);
    grid-template-columns: auto 1fr;
    gap: 13px;
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 13px;
    background: var(--surface-overlay);
    box-shadow: var(--shadow-large);
  }

  .dialog-icon {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--danger) 34%, var(--border));
    border-radius: 10px;
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, var(--surface-high));
  }

  .dialog-copy {
    min-width: 0;
  }

  h2 {
    margin: 1px 0 6px;
    color: var(--text);
    font-size: 15px;
    font-weight: 680;
    letter-spacing: -0.02em;
  }

  p {
    margin: 0;
    color: var(--text-muted);
    font-size: 11px;
    line-height: 1.55;
  }

  .dialog-actions {
    display: flex;
    grid-column: 1 / -1;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 5px;
  }

  button {
    height: 34px;
    padding: 0 13px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 620;
    cursor: pointer;
  }

  .cancel {
    border: 1px solid var(--border);
    color: var(--text-muted);
    background: var(--surface-raised);
  }

  .cancel:hover {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--surface-hover);
  }

  .confirm {
    border: 1px solid color-mix(in srgb, var(--danger) 58%, var(--border));
    color: #fff4f4;
    background: color-mix(in srgb, var(--danger) 64%, #3a2025);
  }

  .confirm:hover {
    background: color-mix(in srgb, var(--danger) 76%, #3a2025);
  }
</style>
