<script lang="ts">
  // Vertical sidebar tabs (Settings). Arrow keys move, Home/End jump.
  import type { Component } from 'svelte';

  export interface TabItem {
    id: string;
    label: string;
    icon: Component<{ size?: number }>;
  }

  interface Props {
    items: readonly TabItem[];
    value: string;
    onchange: (id: string) => void;
  }
  let { items, value, onchange }: Props = $props();

  function onkeydown(e: KeyboardEvent) {
    const index = items.findIndex((t) => t.id === value);
    let next = index;
    if (e.key === 'ArrowDown') next = Math.min(items.length - 1, index + 1);
    else if (e.key === 'ArrowUp') next = Math.max(0, index - 1);
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = items.length - 1;
    else return;
    e.preventDefault();
    const target = items[next];
    if (!target || next === index) return;
    onchange(target.id);
    (e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[role="tab"]')[next]?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-orientation="vertical" tabindex="-1" {onkeydown}>
  {#each items as item (item.id)}
    {@const selected = item.id === value}
    <button
      class="tab"
      class:selected
      type="button"
      role="tab"
      aria-selected={selected}
      aria-controls="tabpanel-{item.id}"
      id="tab-{item.id}"
      tabindex={selected ? 0 : -1}
      onclick={() => onchange(item.id)}
    >
      <item.icon size={16} />
      <span>{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 28px;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text);
    font-size: var(--text-md);
    text-align: left;
    cursor: default;
    transition: background 150ms ease-out;
  }
  .tab :global(svg) {
    color: var(--text-secondary);
  }
  .tab:hover {
    background: var(--hover);
  }
  .tab.selected {
    background: var(--active);
  }
  .tab.selected :global(svg) {
    color: var(--text);
  }
</style>
