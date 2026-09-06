<script module lang="ts">
  export interface MenuItem {
    label: string;
    run: () => void;
    /** When set, the first click swaps the label for this and waits for a second click. */
    confirm?: string;
  }
</script>

<script lang="ts">
  // A small menu that opens *inside* the capsule, in flow, so the window
  // (sized to the capsule) never clips it. The parent decides when it closes.
  import { slide } from 'svelte/transition';
  import { duration } from '$lib/panel/motion';

  let {
    items,
    onclose,
    inset = false,
  }: {
    items: MenuItem[];
    onclose: () => void;
    /** True when the menu sits between rows, so it closes with a hairline too. */
    inset?: boolean;
  } = $props();

  // Label of the item waiting for its second click (labels are the keys).
  let confirming = $state<string | null>(null);

  function pick(item: MenuItem) {
    if (item.confirm && confirming !== item.label) {
      confirming = item.label;
      return;
    }
    onclose();
    item.run();
  }
</script>

<div class="menu" class:inset role="menu" transition:slide={{ duration }}>
  {#each items as item (item.label)}
    <button
      type="button"
      role="menuitem"
      class="item"
      class:confirm={confirming === item.label}
      onclick={() => pick(item)}
    >
      {confirming === item.label ? item.confirm : item.label}
    </button>
  {/each}
</div>

<style>
  .menu {
    display: flex;
    flex-direction: column;
    padding: var(--space-1);
    border-top: 0.5px solid var(--separator);
  }
  .inset {
    border-bottom: 0.5px solid var(--separator);
  }
  .item {
    display: block;
    width: 100%;
    height: 24px;
    /* The wrapper sets --menu-indent so labels line up with the text above them. */
    padding: 0 var(--space-2) 0 var(--menu-indent, var(--space-5));
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text);
    font-size: var(--text-sm);
    line-height: 16px;
    text-align: left;
    white-space: nowrap;
    transition: background 150ms ease-out;
  }
  .item:hover {
    background: var(--hover);
  }
  .item:active {
    background: var(--active);
  }
  .item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .confirm {
    color: var(--status-error);
    font-weight: 600;
  }
  @media (prefers-reduced-motion: reduce) {
    .item {
      transition: none;
    }
  }
</style>
