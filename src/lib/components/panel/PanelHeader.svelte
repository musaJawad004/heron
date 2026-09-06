<script lang="ts">
  // The capsule's top row: mark · count · attention dot · (…) · chevron.
  // Everything except the two buttons is a drag region. Double-click is
  // detected on mousedown (`detail === 2`) because Tauri swallows the
  // mousedown of a drag region and `dblclick` never arrives.
  import Dot from './Dot.svelte';
  import Mark from './Mark.svelte';

  let {
    count,
    attention,
    expanded,
    menuOpen,
    ontoggle,
    onmenu,
  }: {
    count: number;
    attention: number;
    expanded: boolean;
    menuOpen: boolean;
    ontoggle: () => void;
    onmenu: () => void;
  } = $props();

  function mousedown(e: MouseEvent) {
    if (e.button !== 0 || e.detail !== 2) return;
    if (e.target instanceof Element && e.target.closest('button')) return;
    e.preventDefault();
    ontoggle();
  }
  function contextmenu(e: MouseEvent) {
    e.preventDefault();
    onmenu();
  }
</script>

<div
  class="header"
  data-tauri-drag-region
  role="presentation"
  onmousedown={mousedown}
  oncontextmenu={contextmenu}
>
  <span class="mark" data-tauri-drag-region><Mark /></span>
  <span class="count" data-tauri-drag-region>{count}{expanded && count > 0 ? ' running' : ''}</span>
  {#if attention > 0}
    <Dot kind="attention" size={6} pulse />
  {/if}
  <span class="actions" data-tauri-drag-region>
    {#if expanded}
      <button
        type="button"
        class="icon more"
        class:open={menuOpen}
        data-menu-anchor
        aria-label="Panel menu"
        aria-expanded={menuOpen}
        onclick={onmenu}
      >
        <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
          <circle cx="2" cy="6" r="1.25" fill="currentColor" />
          <circle cx="6" cy="6" r="1.25" fill="currentColor" />
          <circle cx="10" cy="6" r="1.25" fill="currentColor" />
        </svg>
      </button>
    {/if}
    <button type="button" class="icon" aria-label={expanded ? 'Collapse' : 'Expand'} onclick={ontoggle}>
      <svg viewBox="0 0 11 11" width="11" height="11" aria-hidden="true" class="chevron" class:up={expanded}>
        <path
          d="M2 4 L5.5 7.5 L9 4"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </span>
</div>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 32px;
    padding: 0 var(--space-2);
  }
  .mark {
    display: flex;
    flex: none;
    color: var(--text);
  }
  .count {
    font-size: var(--text-md);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    line-height: 16px;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    flex: none;
    gap: var(--space-1);
    margin-left: auto;
  }
  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--text-secondary);
    transition:
      background 150ms ease-out,
      color 150ms ease-out,
      opacity 150ms ease-out;
  }
  .icon:hover {
    background: var(--hover);
    color: var(--text);
  }
  .icon:active {
    background: var(--active);
  }
  .icon:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .icon svg {
    pointer-events: none;
  }
  .more {
    opacity: 0;
  }
  .header:hover .more,
  .more.open {
    opacity: 1;
  }
  .more.open {
    background: var(--hover);
    color: var(--text);
  }
  .chevron {
    transition: transform 150ms ease-out;
  }
  .chevron.up {
    transform: rotate(180deg);
  }
  @media (prefers-reduced-motion: reduce) {
    .icon,
    .chevron {
      transition: none;
    }
  }
</style>
