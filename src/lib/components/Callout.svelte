<script lang="ts">
  import IconButton from './IconButton.svelte';
  import X from './icons/X.svelte';

  interface Props {
    tone?: 'warning' | 'quiet';
    message: string;
    /** Inline text action after the message, e.g. "Install hooks". */
    actionLabel?: string;
    onAction?: () => void;
    /** Shows a ✕ button when provided. */
    onDismiss?: () => void;
  }
  let { tone = 'quiet', message, actionLabel, onAction, onDismiss }: Props = $props();
</script>

<div class="callout {tone}" role={tone === 'warning' ? 'alert' : 'status'}>
  <p>
    {message}
    {#if actionLabel && onAction}
      <span class="dash" aria-hidden="true">—</span>
      <button class="link" type="button" onclick={onAction}>{actionLabel}</button>
    {/if}
  </p>
  {#if onDismiss}
    <IconButton label="Dismiss" size={20} onclick={onDismiss}><X size={14} /></IconButton>
  {/if}
</div>

<style>
  .callout {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    margin: var(--space-2) var(--space-2) 0;
    padding: 6px var(--space-2) 6px var(--space-3);
    border-radius: var(--radius-row);
    font-size: var(--text-sm);
    line-height: 16px;
  }
  .quiet {
    background: var(--hover);
    color: var(--text-secondary);
  }
  .warning {
    background: color-mix(in srgb, var(--status-attention) 16%, transparent);
    color: var(--text);
  }
  p {
    flex: 1;
    margin: 0;
    padding: 2px 0;
    overflow-wrap: anywhere;
  }
  .dash {
    margin: 0 2px;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-weight: 500;
    cursor: default;
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
