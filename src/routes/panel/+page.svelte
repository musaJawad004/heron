<script lang="ts">
  // The floating "vigil" capsule. The window is non-focusable, transparent
  // and sized by Rust to whatever this page measures, so everything (menus
  // included) happens inside the one capsule element.
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import { slide } from 'svelte/transition';
  import { api } from '$lib/api';
  import { heron } from '$lib/stores.svelte';
  import { dragEnd } from '$lib/panel/drag';
  import { measure } from '$lib/panel/measure';
  import { duration } from '$lib/panel/motion';
  import { applyPreview } from '$lib/panel/preview';
  import { panelKind, sortForPanel } from '$lib/panel/sessions';
  import PanelHeader from '$lib/components/panel/PanelHeader.svelte';
  import PanelMenu, { type MenuItem } from '$lib/components/panel/PanelMenu.svelte';
  import PanelRow from '$lib/components/panel/PanelRow.svelte';

  const MAX_ROWS = 8;
  const newSessionKey = navigator.userAgent.includes('Mac') ? '⌘N' : 'Ctrl+N';

  type Menu = { kind: 'header' } | { kind: 'row'; id: string };
  let menu = $state<Menu | null>(null);
  let leaveTimer: ReturnType<typeof setTimeout> | null = null;

  const expanded = $derived(heron.settings.panelExpanded);
  const sorted = $derived(sortForPanel(heron.running, heron.snapshot.attention));
  const visible = $derived(sorted.slice(0, MAX_ROWS));
  const overflow = $derived(sorted.length - visible.length);
  const rowMenuId = $derived(menu?.kind === 'row' ? menu.id : null);

  onMount(async () => {
    await heron.init();
    const preview = applyPreview();
    if (preview.menu === 'header') menu = { kind: 'header' };
    if (preview.menu === 'row' && sorted[0]) menu = { kind: 'row', id: sorted[0].id };
  });

  // Close on any left click that is not inside the open menu or on its anchor.
  // Right clicks are left alone so they can move the menu to another row.
  $effect(() => {
    if (!menu) return;
    const close = (e: MouseEvent) => {
      if (e.button !== 0) return;
      if (e.target instanceof Element && e.target.closest('[role="menu"], [data-menu-anchor]')) return;
      menu = null;
    };
    window.addEventListener('mousedown', close, true);
    return () => window.removeEventListener('mousedown', close, true);
  });

  function toggle() {
    menu = null;
    heron.update({ panelExpanded: !expanded });
  }
  function toggleHeaderMenu() {
    menu = menu?.kind === 'header' ? null : { kind: 'header' };
  }
  function toggleRowMenu(id: string) {
    menu = rowMenuId === id ? null : { kind: 'row', id };
  }
  const closeMenu = () => (menu = null);

  // Nothing outside the window can tell us to close, so leaving the capsule
  // (with a little grace) dismisses an open menu.
  function mouseleave() {
    if (!menu) return;
    leaveTimer = setTimeout(closeMenu, 500);
  }
  function mouseenter() {
    if (leaveTimer !== null) clearTimeout(leaveTimer);
    leaveTimer = null;
  }

  const headerItems = $derived<MenuItem[]>([
    { label: expanded ? 'Collapse' : 'Expand', run: toggle },
    { label: 'Hide panel', run: () => api.togglePanel() },
    { label: 'New session…', run: () => api.newSession() },
    { label: 'Move to left edge', run: () => heron.update({ panelEdge: 'left', panelOrigin: null }) },
    { label: 'Move to right edge', run: () => heron.update({ panelEdge: 'right', panelOrigin: null }) },
  ]);
  const rowItems = (id: string): MenuItem[] => [
    { label: 'Focus', run: () => api.focusSession(id) },
    { label: 'Reveal folder', run: () => api.revealSession(id) },
    { label: 'Quit session…', confirm: 'Confirm quit', run: () => api.terminateSession(id) },
  ];
</script>

<!-- No native context menu anywhere in this window, gutter included. -->
<svelte:document oncontextmenu={(e) => e.preventDefault()} />

<div
  class="capsule"
  class:expanded
  data-tauri-drag-region
  role="presentation"
  style:opacity={heron.settings.panelOpacity}
  {@attach measure(api.panelResized)}
  {@attach dragEnd(api.panelMoved)}
  onmouseleave={mouseleave}
  onmouseenter={mouseenter}
>
  <PanelHeader
    count={heron.runningCount}
    attention={heron.attentionCount}
    {expanded}
    menuOpen={menu?.kind === 'header'}
    ontoggle={toggle}
    onmenu={toggleHeaderMenu}
  />
  {#if menu?.kind === 'header'}
    <PanelMenu items={headerItems} onclose={closeMenu} />
  {/if}

  {#if heron.ready}
    {#if expanded}
      <div class="body" data-tauri-drag-region transition:slide={{ duration }}>
        {#if sorted.length === 0}
          <button type="button" class="empty" onclick={() => api.newSession()}>
            No sessions · {newSessionKey}
          </button>
        {:else}
          {#each visible as session (session.id)}
            <div class="item" transition:slide={{ duration }} animate:flip={{ duration }}>
              <PanelRow
                {session}
                kind={panelKind(session, heron.snapshot.attention)}
                onactivate={() => api.focusSession(session.id)}
                onmenu={() => toggleRowMenu(session.id)}
              />
              {#if rowMenuId === session.id}
                <PanelMenu items={rowItems(session.id)} onclose={closeMenu} inset />
              {/if}
            </div>
          {/each}
          {#if overflow > 0}
            <div class="more" data-tauri-drag-region>+{overflow} more</div>
          {/if}
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .capsule {
    /* header menu labels line up with the count (8 + 16 mark + 8) */
    --menu-indent: 28px;
    width: max-content;
    max-width: 220px;
    overflow: hidden;
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-window);
    /* The window is sized to this element, so a CSS drop shadow would be
       clipped at the edge and read as a dark rim; the window's own shadow
       does the lifting instead. */
    background: var(--surface-floating);
  }
  .capsule.expanded {
    width: 220px;
  }
  .body {
    /* row menu labels line up with the project names (4 + 8 + 8 dot + 8) */
    --menu-indent: 20px;
    padding: var(--space-1);
    border-top: 0.5px solid var(--separator);
  }
  .empty {
    display: block;
    width: 100%;
    height: 24px;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-secondary);
    font-size: var(--text-xs);
    line-height: 16px;
    text-align: left;
    white-space: nowrap;
    transition: background 150ms ease-out;
  }
  .empty:hover {
    background: var(--hover);
    color: var(--text);
  }
  .empty:active {
    background: var(--active);
  }
  .more {
    height: 24px;
    padding: 0 var(--space-2) 0 var(--space-5);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    line-height: 24px;
  }
  @media (prefers-reduced-motion: reduce) {
    .empty {
      transition: none;
    }
  }
</style>
