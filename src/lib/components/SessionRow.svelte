<script lang="ts">
  // One session in the popover. Running rows show a status dot, a status
  // word and a hover-revealed "Focus"; recent rows show the path and "Resume".
  import { displayName, needsAttention, projectName, type Session } from '$lib/types';
  import { relativeTime } from '$lib/format';
  import Button from './Button.svelte';
  import PathText from './PathText.svelte';
  import StatusDot from './StatusDot.svelte';

  interface Props {
    session: Session;
    kind: 'running' | 'recent';
    /** Re-render trigger for relative times (pass a ticking timestamp). */
    now: number;
    /** The row is asking "Quit this session?" */
    confirming?: boolean;
    onActivate: () => void;
    onMenu: (at: { x: number; y: number }) => void;
    onRequestQuit?: () => void;
    onQuit?: () => void;
    onCancelQuit?: () => void;
  }
  let {
    session,
    kind,
    now,
    confirming = false,
    onActivate,
    onMenu,
    onRequestQuit,
    onQuit,
    onCancelQuit,
  }: Props = $props();

  const attention = $derived(needsAttention(session));
  const name = $derived(
    kind === 'running' ? displayName(session) : session.title || projectName(session.cwd),
  );
  const subtitleText = $derived(kind === 'running' ? session.title : null);
  const statusWord = $derived(attention ? 'Needs you' : session.status === 'busy' ? 'Working' : 'Idle');
  const time = $derived(relativeTime(session.lastActiveAt, now));
  const actionLabel = $derived(kind === 'running' ? 'Focus' : 'Resume');

  function onkeydown(e: KeyboardEvent) {
    if (confirming) {
      if (e.key === 'Enter') onQuit?.();
      else if (e.key === 'Escape') onCancelQuit?.();
      else return;
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onActivate();
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && onRequestQuit) {
      e.preventDefault();
      onRequestQuit();
    } else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const r = e.currentTarget as HTMLElement;
      const rect = r.getBoundingClientRect();
      onMenu({ x: rect.left + 40, y: rect.top + rect.height / 2 });
    }
  }

  function oncontextmenu(e: MouseEvent) {
    e.preventDefault();
    if (!confirming) onMenu({ x: e.clientX, y: e.clientY });
  }
</script>

<div
  class="row"
  class:confirming
  role="button"
  tabindex="0"
  data-row
  aria-label={confirming
    ? `Quit ${name}?`
    : `${name}, ${kind === 'running' ? statusWord : 'recent'}, ${time}`}
  onclick={() => !confirming && onActivate()}
  {oncontextmenu}
  {onkeydown}
>
  {#if confirming}
    <span class="confirm-text">Quit “{name}”?</span>
    <span class="confirm-actions">
      <Button size="sm" tabindex={-1} onclick={(e) => (e.stopPropagation(), onCancelQuit?.())}>Cancel</Button>
      <Button
        size="sm"
        variant="destructive"
        tabindex={-1}
        onclick={(e) => (e.stopPropagation(), onQuit?.())}
      >
        Quit
      </Button>
    </span>
  {:else}
    <span class="lead">
      {#if kind === 'running'}
        <StatusDot status={session.status} {attention} />
      {/if}
    </span>
    <span class="text">
      <span class="name">{name}</span>
      <span class="sub">
        {#if subtitleText}
          <span class="title">{subtitleText}</span>
        {:else}
          <PathText path={session.cwd} />
        {/if}
      </span>
    </span>
    <span class="trail">
      <span class="meta">
        {#if kind === 'running'}
          <span class="status" class:attention>{statusWord}</span>
        {/if}
        <span class="time">{time}</span>
      </span>
      <span class="action">
        <Button size="sm" tabindex={-1} onclick={(e) => (e.stopPropagation(), onActivate())}
          >{actionLabel}</Button
        >
      </span>
    </span>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 44px;
    margin: 0 var(--space-2);
    padding: 6px var(--space-2);
    border-radius: var(--radius-row);
    transition: background 150ms ease-out;
  }
  .row:hover,
  .row:focus-visible,
  .row.confirming {
    background: var(--hover);
  }
  .row:active:not(.confirming) {
    background: var(--active);
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .lead {
    display: flex;
    justify-content: center;
    flex: none;
    width: 8px;
  }
  .text {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-width: 0;
  }
  .name {
    font-size: var(--text-md);
    font-weight: 600;
    line-height: 17px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    display: flex;
    min-width: 0;
    font-size: var(--text-sm);
    line-height: 16px;
    color: var(--text-secondary);
  }
  .title,
  .sub > :global(.path) {
    flex: 0 1 auto;
    min-width: 0;
  }
  .title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .trail {
    position: relative;
    display: flex;
    justify-content: flex-end;
    flex: none;
    min-width: 56px;
  }
  .meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    font-size: var(--text-xs);
    line-height: 16px;
    color: var(--text-secondary);
    transition: opacity 150ms ease-out;
  }
  .status.attention {
    color: var(--status-attention);
    font-weight: 600;
  }
  .time {
    color: var(--text-tertiary);
  }
  .action {
    position: absolute;
    top: 50%;
    right: 0;
    transform: translateY(-50%);
    opacity: 0;
    visibility: hidden;
    transition:
      opacity 150ms ease-out,
      visibility 150ms;
  }
  .row:hover .action,
  .row:focus-visible .action {
    opacity: 1;
    visibility: visible;
  }
  .row:hover .meta,
  .row:focus-visible .meta {
    opacity: 0;
  }
  .confirm-text {
    flex: 1;
    min-width: 0;
    padding-left: var(--space-4);
    font-size: var(--text-sm);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .confirm-actions {
    display: flex;
    gap: 6px;
    flex: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .row,
    .meta,
    .action {
      transition: none;
    }
  }
</style>
