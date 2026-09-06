<script lang="ts">
  import { onMount } from 'svelte';
  import { heron } from '$lib/stores.svelte';
  import { api } from '$lib/api';
  onMount(() => heron.init());
</script>

<!-- STUB — the popover agent replaces this. -->
<main class="popover">
  <header>
    <strong>Heron</strong>
    <span>{heron.runningCount} running</span>
  </header>
  <ul>
    {#each heron.running as s (s.id)}
      <li>{s.name ?? s.cwd} · {s.status}</li>
    {/each}
  </ul>
  <footer>
    <button onclick={() => api.newSession()}>New session…</button>
    <button onclick={() => api.openSettings()}>Settings…</button>
    <button onclick={() => api.quit()}>Quit</button>
  </footer>
</main>

<style>
  .popover {
    padding: var(--space-3);
    background: var(--surface);
    min-height: 100vh;
  }
  header {
    display: flex;
    justify-content: space-between;
  }
</style>
