<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'default' | 'primary' | 'quiet' | 'destructive';
    size?: 'sm' | 'md';
    disabled?: boolean;
    title?: string;
    tabindex?: number;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }
  let {
    variant = 'default',
    size = 'md',
    disabled = false,
    title,
    tabindex,
    onclick,
    children,
  }: Props = $props();
</script>

<button class="btn {variant} {size}" type="button" {disabled} {title} {tabindex} {onclick}>
  {@render children()}
</button>

<style>
  .btn {
    --btn-bg: light-dark(rgba(255, 255, 255, 0.9), rgba(255, 255, 255, 0.1));
    --btn-bg-hover: light-dark(rgba(255, 255, 255, 1), rgba(255, 255, 255, 0.14));
    --btn-bg-active: light-dark(rgba(0, 0, 0, 0.06), rgba(255, 255, 255, 0.2));
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 24px;
    padding: 0 10px;
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-control);
    background: var(--btn-bg);
    box-shadow: 0 0.5px 1px light-dark(rgba(0, 0, 0, 0.08), rgba(0, 0, 0, 0.3));
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    cursor: default;
    transition:
      background 150ms ease-out,
      opacity 150ms ease-out;
  }
  .btn:hover:not(:disabled) {
    background: var(--btn-bg-hover);
  }
  .btn:active:not(:disabled) {
    background: var(--btn-bg-active);
  }
  .btn:disabled {
    opacity: 0.45;
  }
  .sm {
    height: 20px;
    padding: 0 8px;
    font-size: var(--text-xs);
  }
  .primary {
    background: var(--accent);
    border-color: transparent;
    color: var(--accent-contrast);
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 88%, var(--text));
  }
  .primary:active:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 76%, var(--text));
  }
  .quiet {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }
  .quiet:hover:not(:disabled) {
    background: var(--hover);
  }
  .quiet:active:not(:disabled) {
    background: var(--active);
  }
  .destructive {
    color: var(--status-error);
  }
</style>
