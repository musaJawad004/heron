<script lang="ts">
  // One settings row: label (+ caption) on the left, the control on the right.
  // `stacked` puts the control on its own full-width line for text fields.
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    caption?: string;
    /** id of the control, so the label focuses it. */
    for?: string;
    stacked?: boolean;
    children: Snippet;
  }
  let { label, caption, for: htmlFor, stacked = false, children }: Props = $props();
</script>

<div class="field" class:stacked>
  <label class="label" for={htmlFor}>{label}</label>
  <div class="control">{@render children()}</div>
  {#if caption}
    <p class="caption">{caption}</p>
  {/if}
</div>

<style>
  .field {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      'label control'
      'caption control';
    align-items: center;
    column-gap: var(--space-4);
    min-height: 40px;
    padding: var(--space-2) var(--space-3);
  }
  .stacked {
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas:
      'label'
      'control'
      'caption';
    row-gap: var(--space-1);
  }
  .label {
    grid-area: label;
    font-size: var(--text-md);
    line-height: 18px;
  }
  .control {
    grid-area: control;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2);
    min-width: 0;
  }
  .stacked .control {
    justify-content: flex-start;
  }
  .caption {
    grid-area: caption;
    margin: 2px 0 0;
    font-size: var(--text-sm);
    line-height: 16px;
    color: var(--text-secondary);
  }
  .stacked .caption {
    margin-top: 0;
  }
</style>
