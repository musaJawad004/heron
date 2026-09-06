<script lang="ts">
  import IconButton from '../IconButton.svelte';
  import Gear from '../icons/Gear.svelte';
  import Panel from '../icons/Panel.svelte';
  import Power from '../icons/Power.svelte';
  import { shortcut } from '$lib/keys';

  interface Props {
    panelShown: boolean;
    onTogglePanel: () => void;
    onSettings: () => void;
    onQuit: () => void;
  }
  let { panelShown, onTogglePanel, onSettings, onQuit }: Props = $props();
</script>

<footer class="footer">
  <button class="panel-toggle" type="button" onclick={onTogglePanel} aria-pressed={panelShown}>
    <Panel />
    <span>{panelShown ? 'Hide panel' : 'Show panel'}</span>
  </button>
  <span class="spacer"></span>
  <IconButton label="Settings ({shortcut(',')})" onclick={onSettings}><Gear /></IconButton>
  <IconButton label="Quit Heron ({shortcut('Q')})" onclick={onQuit}><Power /></IconButton>
</footer>

<style>
  .footer {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
    height: 36px;
    padding: 0 var(--space-2);
    border-top: 0.5px solid var(--separator);
    background: light-dark(rgba(0, 0, 0, 0.025), rgba(255, 255, 255, 0.03));
  }
  .panel-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px 0 6px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: default;
    transition:
      background 150ms ease-out,
      color 150ms ease-out;
  }
  .panel-toggle:hover {
    background: var(--hover);
    color: var(--text);
  }
  .panel-toggle:active {
    background: var(--active);
  }
  .spacer {
    flex: 1;
  }
</style>
