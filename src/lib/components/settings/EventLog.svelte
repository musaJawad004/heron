<script lang="ts">
  // The last hook events, newest first. Shows only kind, subtype, project and
  // time: never the message, prompt or tool input.
  import { onMount } from 'svelte';
  import { relativeTime } from '$lib/format';
  import { projectName, type HookEvent } from '$lib/types';

  let { events }: { events: HookEvent[] } = $props();

  let now = $state(Date.now());
  onMount(() => {
    const tick = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(tick);
  });

  const rows = $derived([...events].sort((a, b) => b.receivedAt - a.receivedAt).slice(0, 50));
</script>

<section class="log">
  <h3>Event log</h3>
  <div class="list" role="list">
    {#if rows.length === 0}
      <p class="empty">No events yet. Install the hooks, then use Claude Code in a terminal.</p>
    {:else}
      {#each rows as e (e.id)}
        <div class="row" role="listitem">
          <span class="kind">{e.kind}{e.subtype ? ` · ${e.subtype}` : ''}</span>
          <span class="project">{e.cwd ? projectName(e.cwd) : ''}</span>
          <span class="time">{relativeTime(e.receivedAt, now)}</span>
        </div>
      {/each}
    {/if}
  </div>
  <p class="caption">Events never leave this machine.</p>
</section>

<style>
  h3 {
    margin: 0 0 6px var(--space-3);
    font-size: var(--text-sm);
    font-weight: 600;
    line-height: 16px;
    color: var(--text-secondary);
  }
  .list {
    max-height: 168px;
    overflow-y: auto;
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-row);
    background: light-dark(rgba(255, 255, 255, 0.7), rgba(255, 255, 255, 0.045));
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
  .list::-webkit-scrollbar {
    width: 6px;
  }
  .list::-webkit-scrollbar-thumb {
    border-radius: 3px;
    background: var(--text-tertiary);
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 104px) 48px;
    gap: var(--space-2);
    padding: 5px var(--space-3);
    line-height: 14px;
  }
  .row + .row {
    border-top: 0.5px solid var(--separator);
  }
  .kind,
  .project {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .project {
    color: var(--text-secondary);
  }
  .time {
    color: var(--text-tertiary);
    text-align: right;
  }
  .empty {
    margin: 0;
    padding: var(--space-3);
    font-family: var(--font-ui);
    font-size: var(--text-sm);
    color: var(--text-tertiary);
  }
  .caption {
    margin: 6px 0 0 var(--space-3);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
</style>
