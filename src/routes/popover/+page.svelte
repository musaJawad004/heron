<script lang="ts">
  import { onMount } from 'svelte';
  import { heron } from '$lib/stores.svelte';
  import { api } from '$lib/api';
  import { needsAttention, type Session } from '$lib/types';
  import { focusSibling, isMod, isTyping, shortcut } from '$lib/keys';
  import Callout from '$lib/components/Callout.svelte';
  import ContextMenu, { type MenuEntry } from '$lib/components/ContextMenu.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import PopoverFooter from '$lib/components/popover/PopoverFooter.svelte';
  import PopoverHeader from '$lib/components/popover/PopoverHeader.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';
  import SessionRow from '$lib/components/SessionRow.svelte';
  import Refresh from '$lib/components/icons/Refresh.svelte';

  const MIN_HEIGHT = 120;
  const MAX_HEIGHT = 560;

  let now = $state(Date.now());
  let menu = $state<{ x: number; y: number; items: MenuEntry[] } | null>(null);
  let confirmingId = $state<string | null>(null);
  let root = $state<HTMLDivElement>();
  let content = $state<HTMLDivElement>();

  onMount(() => {
    heron.init();
    const tick = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(tick);
  });

  // Report the natural height so Rust can size the window; the list scrolls past MAX_HEIGHT.
  $effect(() => {
    const rootEl = root;
    const contentEl = content;
    const scrollEl = contentEl?.parentElement;
    if (!rootEl || !contentEl || !scrollEl) return;
    let last = 0;
    const observer = new ResizeObserver(() => {
      // Everything except the scroll area (header, footer, borders) plus the unclipped list height.
      const chrome = rootEl.offsetHeight - scrollEl.clientHeight;
      const wanted = Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, Math.ceil(chrome + contentEl.offsetHeight)));
      if (wanted !== last) {
        last = wanted;
        api.popoverResized(wanted).catch(() => {});
      }
    });
    observer.observe(contentEl);
    return () => observer.disconnect();
  });

  const rank = (s: Session) => (needsAttention(s) ? 0 : s.status === 'busy' ? 1 : 2);
  const running = $derived([...heron.running].sort((a, b) => rank(a) - rank(b)));
  const recent = $derived(heron.recent.slice(0, heron.settings.recentLimit));

  const summary = $derived.by(() => {
    const n = heron.runningCount;
    if (n === 0) return 'No sessions';
    const a = heron.attentionCount;
    return a ? `${n} running · ${a} ${a === 1 ? 'needs' : 'need'} you` : `${n} running`;
  });

  const copy = (text: string) => navigator.clipboard.writeText(text).catch(() => {});

  const runningMenu = (s: Session): MenuEntry[] => [
    { label: 'Focus', onSelect: () => api.focusSession(s.id) },
    { label: 'Reveal folder', onSelect: () => api.revealSession(s.id) },
    { label: 'Copy session ID', onSelect: () => copy(s.id) },
    { label: 'Copy path', onSelect: () => copy(s.cwd) },
    'separator',
    { label: 'Quit session…', destructive: true, onSelect: () => (confirmingId = s.id) },
  ];

  const recentMenu = (s: Session): MenuEntry[] => [
    { label: 'Resume', onSelect: () => api.resumeSession(s.id) },
    { label: 'Reveal folder', onSelect: () => api.revealSession(s.id) },
    { label: 'Copy session ID', onSelect: () => copy(s.id) },
  ];

  function openMenu(at: { x: number; y: number }, items: MenuEntry[]) {
    menu = { ...at, items };
  }

  function quit(id: string) {
    confirmingId = null;
    api.terminateSession(id);
  }

  function onkeydown(e: KeyboardEvent) {
    if (menu) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      if (confirmingId) confirmingId = null;
      else api.hidePopover();
      return;
    }
    if (isMod(e)) {
      const key = e.key.toLowerCase();
      if (key === 'n') api.newSession();
      else if (key === ',') api.openSettings();
      else if (key === 'q') api.quit();
      else return;
      e.preventDefault();
      return;
    }
    if (isTyping(e) || !root) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      focusSibling(
        Array.from(root.querySelectorAll<HTMLElement>('[data-row]')),
        e.key === 'ArrowDown' ? 1 : -1,
      );
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="popover" bind:this={root}>
  <PopoverHeader {summary} onNew={() => api.newSession()} />

  <div class="scroll">
    <div class="content" bind:this={content}>
      {#if heron.snapshot.lastError}
        <Callout tone="warning" message={heron.snapshot.lastError} onDismiss={() => api.clearError()} />
      {/if}
      {#if heron.ready && heron.snapshot.hookStatus !== 'installed'}
        <Callout
          message="Notifications are off"
          actionLabel="Install hooks"
          onAction={() => api.installHooks()}
        />
      {/if}

      <SectionHeader title="Running" />
      {#if running.length === 0}
        <EmptyState message="No Claude sessions running. {shortcut('N')} to start one in a folder." />
      {:else}
        {#each running as s (s.id)}
          <SessionRow
            session={s}
            kind="running"
            {now}
            confirming={confirmingId === s.id}
            onActivate={() => api.focusSession(s.id)}
            onMenu={(at) => openMenu(at, runningMenu(s))}
            onRequestQuit={() => (confirmingId = s.id)}
            onQuit={() => quit(s.id)}
            onCancelQuit={() => (confirmingId = null)}
          />
        {/each}
      {/if}

      <SectionHeader title="Recent">
        <span class="refresh" class:spinning={heron.snapshot.isRefreshingHistory}>
          <IconButton
            label="Refresh history"
            size={20}
            disabled={heron.snapshot.isRefreshingHistory}
            onclick={() => api.refreshHistory()}
          >
            <Refresh size={14} />
          </IconButton>
        </span>
      </SectionHeader>
      {#if recent.length === 0}
        <EmptyState message="No past sessions yet." />
      {:else}
        {#each recent as s (s.id)}
          <SessionRow
            session={s}
            kind="recent"
            {now}
            onActivate={() => api.resumeSession(s.id)}
            onMenu={(at) => openMenu(at, recentMenu(s))}
          />
        {/each}
      {/if}
    </div>
  </div>

  <PopoverFooter
    panelShown={heron.settings.showPanel}
    onTogglePanel={() => api.togglePanel()}
    onSettings={() => api.openSettings()}
    onQuit={() => api.quit()}
  />

  {#if menu}
    <ContextMenu items={menu.items} x={menu.x} y={menu.y} onClose={() => (menu = null)} />
  {/if}
</div>

<style>
  .popover {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-window);
    background: var(--surface-floating);
    box-shadow: inset 0 0.5px 0 var(--glass-edge);
    overflow: hidden;
  }
  .scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .scroll::-webkit-scrollbar {
    width: 6px;
  }
  .scroll::-webkit-scrollbar-thumb {
    border-radius: 3px;
    background: var(--text-tertiary);
    background-clip: content-box;
    border: 1px solid transparent;
  }
  .scroll::-webkit-scrollbar-track {
    background: transparent;
  }
  .content {
    padding-bottom: var(--space-2);
  }
  .refresh {
    display: flex;
  }
  .spinning :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinning :global(svg) {
      animation: none;
    }
  }
  :global(:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
