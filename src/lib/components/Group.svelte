<script lang="ts">
  // A rounded container of settings rows separated by hairlines.
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    children: Snippet;
  }
  let { title, children }: Props = $props();
</script>

<section class="group">
  {#if title}
    <h3>{title}</h3>
  {/if}
  <div class="rows">{@render children()}</div>
</section>

<style>
  .group {
    --group-bg: light-dark(rgba(255, 255, 255, 0.7), rgba(255, 255, 255, 0.045));
    margin-bottom: var(--space-3);
  }
  h3 {
    margin: 0 0 6px var(--space-3);
    font-size: var(--text-sm);
    font-weight: 600;
    line-height: 16px;
    color: var(--text-secondary);
  }
  .rows {
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-row);
    background: var(--group-bg);
  }
  .rows > :global(* + *) {
    border-top: 0.5px solid var(--separator);
  }
</style>
