<script lang="ts">
  interface Props {
    checked: boolean;
    onchange: (next: boolean) => void;
    disabled?: boolean;
    /** Accessible name when the toggle is not wrapped by a labelled Field. */
    label?: string;
    id?: string;
  }
  let { checked, onchange, disabled = false, label, id }: Props = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {id}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    flex: none;
    width: 26px;
    height: 16px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: light-dark(rgba(0, 0, 0, 0.16), rgba(255, 255, 255, 0.22));
    cursor: default;
    transition: background 150ms ease-out;
  }
  .toggle.on {
    background: var(--accent);
  }
  .toggle:disabled {
    opacity: 0.45;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.25),
      0 0 0 0.5px rgba(0, 0, 0, 0.06);
    transition: transform 150ms ease-out;
  }
  .on .knob {
    transform: translateX(10px);
  }
  @media (prefers-reduced-motion: reduce) {
    .toggle,
    .knob {
      transition: none;
    }
  }
</style>
