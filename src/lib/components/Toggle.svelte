<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, disabled = false, onchange }: Props = $props();
</script>

<button
  type="button"
  class:checked
  class="toggle"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 38px;
    height: 22px;
    flex: 0 0 auto;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--surface-high);
    cursor: pointer;
    transition:
      border-color 160ms ease,
      background-color 160ms ease;
  }

  .toggle span {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-subtle);
    box-shadow: 0 1px 3px rgb(0 0 0 / 35%);
    transition:
      transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
      background-color 160ms ease;
  }

  .toggle.checked {
    border-color: color-mix(in srgb, var(--accent) 70%, white 5%);
    background: color-mix(in srgb, var(--accent) 34%, var(--surface-high));
  }

  .toggle.checked span {
    transform: translateX(16px);
    background: var(--accent-bright);
  }

  .toggle:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 3px;
  }

  .toggle:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
</style>
