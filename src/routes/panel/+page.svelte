<script lang="ts">
  import { onMount } from 'svelte';
  import { heron } from '$lib/stores.svelte';
  onMount(() => heron.init());
</script>

<!-- STUB — the panel agent replaces this. -->
<div class="panel" data-tauri-drag-region>
  <span class="count">{heron.runningCount}</span>
  {#each heron.running as s (s.id)}
    <div class="row">{s.name ?? s.cwd}</div>
  {/each}
</div>

<style>
  .panel {
    padding: var(--space-2) var(--space-3);
    background: var(--surface);
    border-radius: var(--radius-window);
  }
</style>
