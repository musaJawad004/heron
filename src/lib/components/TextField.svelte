<script lang="ts">
  interface Props {
    value: string;
    /** Called when the edit is committed (blur or Enter), not on every keystroke. */
    onchange: (value: string) => void;
    placeholder?: string;
    type?: 'text' | 'number';
    min?: number;
    max?: number;
    mono?: boolean;
    id?: string;
    label?: string;
    width?: string;
  }
  let {
    value,
    onchange,
    placeholder,
    type = 'text',
    min,
    max,
    mono = false,
    id,
    label,
    width = '100%',
  }: Props = $props();

  function commit(e: Event & { currentTarget: HTMLInputElement }) {
    let next = e.currentTarget.value;
    if (type === 'number') {
      const n = Number(next);
      if (next === '' || Number.isNaN(n)) return;
      const clamped = Math.min(max ?? Infinity, Math.max(min ?? -Infinity, Math.round(n)));
      next = String(clamped);
      e.currentTarget.value = next;
    }
    if (next !== value) onchange(next);
  }

  function onkeydown(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key === 'Enter') e.currentTarget.blur();
    if (e.key === 'Escape') {
      e.currentTarget.value = value;
      e.currentTarget.blur();
    }
  }
</script>

<input
  class="field"
  class:mono
  style="width: {width}"
  {type}
  {id}
  {value}
  {placeholder}
  {min}
  {max}
  aria-label={label}
  autocomplete="off"
  spellcheck="false"
  onchange={commit}
  {onkeydown}
/>

<style>
  .field {
    height: 24px;
    padding: 0 8px;
    border: 0.5px solid var(--separator);
    border-radius: var(--radius-control);
    background: light-dark(#fff, rgba(255, 255, 255, 0.06));
    box-shadow: inset 0 0.5px 1px light-dark(rgba(0, 0, 0, 0.06), rgba(0, 0, 0, 0.4));
    color: var(--text);
    font-size: var(--text-sm);
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
  .field::placeholder {
    color: var(--text-tertiary);
  }
  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
  .field[type='number'] {
    text-align: right;
    padding-right: 4px;
  }
</style>
