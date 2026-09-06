<script lang="ts">
  import Chevron from './icons/Chevron.svelte';

  export interface SelectOption {
    value: string;
    label: string;
  }

  interface Props {
    options: SelectOption[];
    value: string;
    onchange: (value: string) => void;
    id?: string;
    label?: string;
    disabled?: boolean;
    width?: string;
  }
  let { options, value, onchange, id, label, disabled = false, width = 'auto' }: Props = $props();
</script>

<span class="select" style="width: {width}">
  <select {id} {value} {disabled} aria-label={label} onchange={(e) => onchange(e.currentTarget.value)}>
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <span class="chevron"><Chevron size={12} /></span>
</span>

<style>
  .select {
    position: relative;
    display: inline-flex;
    min-width: 120px;
  }
  select {
    width: 100%;
    height: 24px;
    padding: 0 26px 0 8px;
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-control);
    background: light-dark(rgba(255, 255, 255, 0.9), rgba(255, 255, 255, 0.1));
    box-shadow: 0 0.5px 1px light-dark(rgba(0, 0, 0, 0.08), rgba(0, 0, 0, 0.3));
    color: var(--text);
    font-size: var(--text-sm);
    appearance: none;
    -webkit-appearance: none;
    cursor: default;
  }
  select:disabled {
    opacity: 0.45;
  }
  .chevron {
    position: absolute;
    top: 0;
    right: 6px;
    display: flex;
    align-items: center;
    height: 100%;
    color: var(--text-secondary);
    pointer-events: none;
  }
</style>
