<script lang="ts" module>
  export type MenuEntry =
    { label: string; onSelect: () => void; destructive?: boolean; disabled?: boolean } | 'separator';
</script>

<script lang="ts">
  // A custom right-click menu positioned at (x, y) and kept inside the window.
  // Closes on Escape, outside click, or when the window loses focus.
  interface Props {
    items: MenuEntry[];
    x: number;
    y: number;
    onClose: () => void;
  }
  let { items, x, y, onClose }: Props = $props();

  let el = $state<HTMLDivElement>();
  let active = $state(-1);
  /** Final position after clamping to the window; null until measured. */
  let pos = $state<{ left: number; top: number } | null>(null);

  const selectable = $derived(items.flatMap((item, i) => (item === 'separator' || item.disabled ? [] : [i])));

  $effect(() => {
    if (!el) return;
    const rect = el.getBoundingClientRect();
    pos = {
      left: Math.max(4, Math.min(x, window.innerWidth - rect.width - 4)),
      top: Math.max(4, Math.min(y, window.innerHeight - rect.height - 4)),
    };
    el.focus();
  });

  $effect(() => {
    const onPointerDown = (e: PointerEvent) => {
      if (el && !el.contains(e.target as Node)) onClose();
    };
    window.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('blur', onClose);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown, true);
      window.removeEventListener('blur', onClose);
    };
  });

  function choose(index: number) {
    const item = items[index];
    if (!item || item === 'separator' || item.disabled) return;
    onClose();
    item.onSelect();
  }

  function step(delta: number) {
    if (selectable.length === 0) return;
    const at = selectable.indexOf(active);
    const next =
      at < 0 ? (delta > 0 ? 0 : selectable.length - 1) : (at + delta + selectable.length) % selectable.length;
    active = selectable[next] ?? -1;
  }

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    switch (e.key) {
      case 'ArrowDown':
        step(1);
        break;
      case 'ArrowUp':
        step(-1);
        break;
      case 'Home':
        active = selectable[0] ?? -1;
        break;
      case 'End':
        active = selectable[selectable.length - 1] ?? -1;
        break;
      case 'Enter':
      case ' ':
        choose(active);
        break;
      case 'Escape':
      case 'Tab':
        onClose();
        break;
      default:
        return;
    }
    e.preventDefault();
  }
</script>

<div
  class="menu"
  role="menu"
  tabindex="-1"
  bind:this={el}
  class:placed={pos !== null}
  style="left: {pos?.left ?? x}px; top: {pos?.top ?? y}px"
  {onkeydown}
  oncontextmenu={(e) => e.preventDefault()}
  onpointerleave={() => (active = -1)}
>
  {#each items as item, i (i)}
    {#if item === 'separator'}
      <div class="separator" role="separator"></div>
    {:else}
      <button
        class="item"
        class:destructive={item.destructive}
        class:active={i === active}
        type="button"
        role="menuitem"
        tabindex="-1"
        disabled={item.disabled}
        onpointerenter={() => (active = i)}
        onclick={() => choose(i)}
      >
        {item.label}
      </button>
    {/if}
  {/each}
</div>

<style>
  .menu {
    --menu-bg: light-dark(rgba(246, 246, 247, 0.96), rgba(44, 44, 46, 0.96));
    position: fixed;
    z-index: 10;
    min-width: 160px;
    padding: var(--space-1);
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-row);
    background: var(--menu-bg);
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.18),
      0 0 0 0.5px light-dark(rgba(0, 0, 0, 0.04), rgba(0, 0, 0, 0.6));
    outline: none;
    visibility: hidden;
  }
  .menu.placed {
    visibility: visible;
  }
  .item {
    display: block;
    width: 100%;
    height: 24px;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    font-size: var(--text-md);
    text-align: left;
    cursor: default;
  }
  .item.active:not(:disabled) {
    background: var(--active);
  }
  .item:disabled {
    color: var(--text-tertiary);
  }
  .destructive {
    color: var(--status-error);
  }
  .separator {
    height: 0.5px;
    margin: var(--space-1) var(--space-2);
    background: var(--separator);
  }
</style>
