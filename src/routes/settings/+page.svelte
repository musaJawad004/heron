<script lang="ts">
  import { onMount } from 'svelte';
  import { heron } from '$lib/stores.svelte';
  onMount(() => heron.init());
</script>

<!-- STUB — the settings agent replaces this. -->
<main class="settings">
  <h1>Settings</h1>
  <label>
    <input
      type="checkbox"
      checked={heron.settings.showPanel}
      onchange={(e) => heron.update({ showPanel: e.currentTarget.checked })}
    />
    Show floating panel
  </label>
</main>

<style>
  .settings {
    padding: var(--space-5);
    background: var(--surface-solid);
    min-height: 100vh;
  }
</style>
