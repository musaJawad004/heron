<script lang="ts">
  // One running session: dot · project · status word. Click focuses the
  // terminal; right-click asks the parent for the row menu.
  import Dot from './Dot.svelte';
  import { statusWord, type PanelKind } from '$lib/panel/sessions';
  import { projectName, type Session } from '$lib/types';

  let {
    session,
    kind,
    onactivate,
    onmenu,
  }: {
    session: Session;
    kind: PanelKind;
    onactivate: () => void;
    onmenu: () => void;
  } = $props();

  function contextmenu(e: MouseEvent) {
    e.preventDefault();
    onmenu();
  }
</script>

<button type="button" class="row" onclick={onactivate} oncontextmenu={contextmenu}>
  <Dot {kind} />
  <span class="name">{projectName(session.cwd)}</span>
  <span class="status">{statusWord(kind)}</span>
</button>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    height: 24px;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text);
    text-align: left;
    transition: background 150ms ease-out;
  }
  .row:hover {
    background: var(--hover);
  }
  .row:active {
    background: var(--active);
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    line-height: 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status {
    flex: none;
    font-size: var(--text-xs);
    line-height: 16px;
    color: var(--text-secondary);
  }
  @media (prefers-reduced-motion: reduce) {
    .row {
      transition: none;
    }
  }
</style>
