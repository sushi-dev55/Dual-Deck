<script lang="ts">
  import { ChevronDown, GripVertical, Search, X } from '@lucide/svelte';
  import { actionCatalog } from '../data/catalog';
  import type { ActionCategory, ActionDefinition } from '../models';
  import ActionIcon from './ActionIcon.svelte';

  interface Props {
    onassign: (action: ActionDefinition) => void;
  }

  let { onassign }: Props = $props();
  let query = $state('');
  let collapsed = $state<ActionCategory[]>([]);
  let draggedActionId = $state<string | null>(null);
  const categories: ActionCategory[] = ['Launch', 'Keyboard', 'Media', 'Workflow', 'Streaming'];
  let filtered = $derived(
    actionCatalog.filter((action) =>
      `${action.title} ${action.description} ${action.category}`
        .toLowerCase()
        .includes(query.trim().toLowerCase()),
    ),
  );

  function toggleCategory(category: ActionCategory) {
    collapsed = collapsed.includes(category)
      ? collapsed.filter((item) => item !== category)
      : [...collapsed, category];
  }

  function startDrag(event: DragEvent, action: ActionDefinition) {
    if (!event.dataTransfer) return;
    event.dataTransfer.effectAllowed = 'copy';
    event.dataTransfer.setData('application/x-dual-deck-action', action.id);
    event.dataTransfer.setData('text/plain', action.title);
    draggedActionId = action.id;
  }

  function finishDrag() {
    draggedActionId = null;
  }
</script>

<aside class="library" aria-label="Action library">
  <div class="library-heading">
    <div>
      <span>Action library</span>
      <small>{actionCatalog.length} actions</small>
    </div>
  </div>

  <label class="search-field">
    <Search size={15} aria-hidden="true" />
    <input
      id="action-search"
      bind:value={query}
      type="search"
      placeholder="Search actions"
      aria-label="Search actions"
    />
    {#if query}
      <button type="button" aria-label="Clear search" onclick={() => (query = '')}
        ><X size={14} /></button
      >
    {:else}
      <kbd>Ctrl K</kbd>
    {/if}
  </label>

  <div class="action-scroll">
    {#each categories as category}
      {@const actions = filtered.filter((action) => action.category === category)}
      {#if actions.length}
        <section class="action-group">
          <button
            type="button"
            class="group-heading"
            aria-expanded={!collapsed.includes(category)}
            onclick={() => toggleCategory(category)}
          >
            <span>{category}</span>
            <span class="count">{actions.length}</span>
            <span class:collapsed={collapsed.includes(category)} class="group-chevron">
              <ChevronDown size={14} />
            </span>
          </button>

          {#if !collapsed.includes(category)}
            <div class="action-list">
              {#each actions as action (action.id)}
                <button
                  type="button"
                  class:dragging={draggedActionId === action.id}
                  class="action-row"
                  draggable="true"
                  ondragstart={(event) => startDrag(event, action)}
                  ondragend={finishDrag}
                  onclick={() => onassign(action)}
                  title={`Assign ${action.title} to the selected control`}
                >
                  <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
                  <span class="action-icon" style={`--action-accent:${action.accent}`}>
                    <ActionIcon name={action.icon} size={17} />
                  </span>
                  <span class="action-copy">
                    <strong>{action.title}</strong>
                    <small>{action.description}</small>
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        </section>
      {/if}
    {/each}

    {#if filtered.length === 0}
      <div class="empty-search">
        <Search size={20} />
        <strong>No matching actions</strong>
        <span>Try a broader name or category.</span>
      </div>
    {/if}
  </div>

  <footer>
    <span class="drop-glyph" aria-hidden="true">+</span>
    <p>Drag an action onto any controller button</p>
  </footer>
</aside>

<style>
  .library {
    display: grid;
    min-width: 0;
    min-height: 0;
    grid-template-rows: auto auto 1fr auto;
    border-right: 1px solid var(--border);
    background: var(--surface-panel);
  }

  .library-heading {
    display: flex;
    min-height: 62px;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
  }

  .library-heading > div {
    display: grid;
    gap: 2px;
  }

  .library-heading span {
    color: var(--text);
    font-size: 12px;
    font-weight: 680;
  }

  .library-heading small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .search-field {
    display: grid;
    height: 34px;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 7px;
    margin: 0 12px 11px;
    padding: 0 8px 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-subtle);
    background: var(--surface-base);
    transition:
      border-color var(--motion-base) var(--ease-standard),
      box-shadow var(--motion-base) var(--ease-standard),
      background-color var(--motion-base) var(--ease-standard);
  }

  .search-field:focus-within {
    border-color: color-mix(in srgb, var(--accent) 62%, var(--border));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .search-field input {
    min-width: 0;
    border: 0;
    outline: 0;
    color: var(--text);
    background: transparent;
    font-size: 11px;
  }

  .search-field input::placeholder {
    color: var(--text-disabled);
  }

  .search-field button {
    display: grid;
    width: 22px;
    height: 22px;
    padding: 0;
    place-items: center;
    border: 0;
    border-radius: 5px;
    color: var(--text-subtle);
    background: transparent;
    cursor: pointer;
  }

  .search-field button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  kbd {
    padding: 2px 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-disabled);
    background: var(--surface-raised);
    font-family: inherit;
    font-size: 10px;
  }

  .action-scroll {
    min-height: 0;
    padding: 0 7px 14px;
    overflow: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--surface-high) transparent;
  }

  .action-group + .action-group {
    margin-top: 6px;
  }

  .group-heading {
    display: grid;
    width: 100%;
    height: 30px;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 7px;
    padding: 0 7px 0 9px;
    border: 0;
    border-radius: 6px;
    color: var(--text-subtle);
    background: transparent;
    font-size: 10px;
    font-weight: 720;
    letter-spacing: 0.08em;
    text-align: left;
    text-transform: uppercase;
    cursor: pointer;
    transition:
      color var(--motion-quick) var(--ease-standard),
      background-color var(--motion-quick) var(--ease-standard);
  }

  .group-heading:hover {
    color: var(--text-muted);
    background: var(--surface-hover);
  }

  .count {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
  }

  .group-chevron {
    display: grid;
    place-items: center;
    transform: rotate(0deg);
    transition: transform var(--motion-base) var(--ease-emphasized);
  }

  .group-chevron.collapsed {
    transform: rotate(-90deg);
  }

  .action-list {
    display: grid;
    gap: 2px;
    transform-origin: top;
    animation: action-list-in var(--motion-base) var(--ease-emphasized) both;
  }

  .action-row {
    display: grid;
    width: 100%;
    min-height: 49px;
    grid-template-columns: 10px 32px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    padding: 5px 8px 5px 4px;
    border: 1px solid transparent;
    border-radius: 9px;
    color: var(--text-muted);
    background: transparent;
    text-align: left;
    cursor: grab;
    transition:
      border-color var(--motion-quick) var(--ease-standard),
      background-color var(--motion-quick) var(--ease-standard),
      color var(--motion-quick) var(--ease-standard),
      opacity var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .action-row:hover {
    border-color: var(--border);
    background: var(--surface-raised);
    transform: translateX(2px);
  }

  .action-row:active {
    cursor: grabbing;
    transform: translateX(2px) scale(0.985);
  }

  .action-row.dragging {
    border-color: color-mix(in srgb, var(--accent) 34%, var(--border));
    opacity: 0.52;
    transform: scale(0.975);
  }

  .drag-handle {
    color: transparent;
    transition: color 120ms ease;
  }

  .action-row:hover .drag-handle,
  .action-row:focus-visible .drag-handle {
    color: var(--text-disabled);
  }

  .action-icon {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--action-accent) 22%, var(--border));
    border-radius: 8px;
    color: var(--action-accent);
    background: color-mix(in srgb, var(--action-accent) 8%, var(--surface-high));
    transition:
      border-color var(--motion-base) var(--ease-standard),
      background-color var(--motion-base) var(--ease-standard),
      transform var(--motion-base) var(--ease-emphasized);
  }

  .action-row:hover .action-icon {
    border-color: color-mix(in srgb, var(--action-accent) 42%, var(--border));
    background: color-mix(in srgb, var(--action-accent) 13%, var(--surface-high));
    transform: translateY(-1px) scale(1.04);
  }

  .action-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .action-copy strong,
  .action-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action-copy strong {
    color: var(--text);
    font-size: 11px;
    font-weight: 610;
  }

  .action-copy small {
    color: var(--text-subtle);
    font-size: 10px;
  }

  .empty-search {
    display: grid;
    min-height: 180px;
    place-items: center;
    align-content: center;
    gap: 7px;
    padding: 20px;
    color: var(--text-disabled);
    text-align: center;
  }

  .empty-search strong {
    color: var(--text-muted);
    font-size: 11px;
  }

  .empty-search span {
    color: var(--text-subtle);
    font-size: 10px;
  }

  footer {
    display: flex;
    min-height: 46px;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    color: var(--text-subtle);
    background: var(--surface-base);
  }

  footer p {
    margin: 0;
    font-size: 10px;
    line-height: 1.3;
  }

  .drop-glyph {
    display: grid;
    width: 20px;
    height: 20px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px dashed var(--border-strong);
    border-radius: 5px;
    color: var(--accent-bright);
    font-size: 13px;
  }

  @keyframes action-list-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scaleY(0.985);
    }
  }
</style>
