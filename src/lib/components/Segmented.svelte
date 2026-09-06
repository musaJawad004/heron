<script lang="ts">
  export interface SegmentOption<T extends string> {
    value: T;
    label: string;
  }

  interface Props<T extends string> {
    options: SegmentOption<T>[];
    value: T;
    onchange: (value: T) => void;
    label?: string;
  }
  let { options, value, onchange, label }: Props<string> = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    {@const selected = option.value === value}
    <button
      class="segment"
      class:selected
      type="button"
      role="radio"
      aria-checked={selected}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-control);
    background: light-dark(rgba(0, 0, 0, 0.06), rgba(255, 255, 255, 0.08));
  }
  .segment {
    min-width: 56px;
    height: 20px;
    padding: 0 10px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: 500;
    line-height: 1;
    cursor: default;
    transition:
      background 150ms ease-out,
      color 150ms ease-out;
  }
  .segment:hover {
    color: var(--text);
  }
  .segment.selected {
    background: light-dark(#fff, rgba(255, 255, 255, 0.16));
    box-shadow: 0 0.5px 1.5px light-dark(rgba(0, 0, 0, 0.15), rgba(0, 0, 0, 0.4));
    color: var(--text);
  }
</style>
