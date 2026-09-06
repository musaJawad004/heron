<script lang="ts">
  import { api } from '$lib/api';
  import { version } from '$lib/version';
  import Button from '../Button.svelte';
  import External from '../icons/External.svelte';
  import HeronMark from '../icons/HeronMark.svelte';

  const GITHUB = 'https://github.com/muhammadmusadev/heron';
  const PRIVACY = 'https://github.com/muhammadmusadev/heron/blob/main/docs/PRIVACY.md';
</script>

<div class="about">
  <span class="mark"><HeronMark size={64} /></span>
  <h2>Heron</h2>
  <p class="version">Version {version}</p>
  <p class="tagline">Open source · MIT · No network, ever.</p>
  <div class="buttons">
    <Button onclick={() => api.openUrl(GITHUB)}>GitHub <External size={12} /></Button>
    <Button onclick={() => api.openUrl(PRIVACY)}>Privacy <External size={12} /></Button>
    <Button onclick={() => api.revealAppData()}>Show app data folder</Button>
  </div>
</div>

<style>
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: var(--space-5);
    text-align: center;
  }
  .mark {
    display: flex;
    margin-bottom: var(--space-3);
    color: var(--text);
  }
  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 20px;
  }
  .version {
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
  .tagline {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }
</style>
