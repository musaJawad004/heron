<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    /** Trailing action, e.g. an icon button. */
    children?: Snippet;
  }
  let { title, children }: Props = $props();
</script>

<div class="section-header">
  <h2>{title}</h2>
  {#if children}
    <div class="action">{@render children()}</div>
  {/if}
</div>

<style>
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 28px;
    padding: var(--space-2) var(--space-4) 0;
  }
  h2 {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 600;
    line-height: 16px;
    color: var(--text-secondary);
  }
  .action {
    display: flex;
    margin-right: -4px;
  }
</style>
