<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Accessible name; also the tooltip. */
    label: string;
    size?: number;
    disabled?: boolean;
    tabindex?: number;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }
  let { label, size = 24, disabled = false, tabindex, onclick, children }: Props = $props();
</script>

<button
  class="icon-button"
  type="button"
  style="--size: {size}px"
  aria-label={label}
  title={label}
  {disabled}
  {tabindex}
  {onclick}
>
  {@render children()}
</button>

<style>
  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: var(--size);
    height: var(--size);
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-secondary);
    cursor: default;
    transition:
      background 150ms ease-out,
      color 150ms ease-out;
  }
  .icon-button:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .icon-button:active:not(:disabled) {
    background: var(--active);
  }
  .icon-button:disabled {
    opacity: 0.45;
  }
</style>
