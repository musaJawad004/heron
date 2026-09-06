<script lang="ts">
  import IconButton from '../IconButton.svelte';
  import HeronMark from '../icons/HeronMark.svelte';
  import Plus from '../icons/Plus.svelte';
  import { shortcut } from '$lib/keys';

  interface Props {
    summary: string;
    onNew: () => void;
  }
  let { summary, onNew }: Props = $props();
</script>

<header class="header">
  <span class="mark"><HeronMark size={18} /></span>
  <h1>Heron</h1>
  <span class="summary">{summary}</span>
  <IconButton label="New session ({shortcut('N')})" onclick={onNew}><Plus /></IconButton>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: none;
    height: 44px;
    padding: 0 var(--space-2) 0 var(--space-4);
  }
  .mark {
    display: flex;
    flex: none;
    color: var(--text);
  }
  h1 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 20px;
  }
  .summary {
    flex: 1 1 auto;
    min-width: 0;
    margin-left: 2px;
    font-size: var(--text-sm);
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
