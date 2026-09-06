<script lang="ts">
  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    /** Fires while dragging. */
    oninput?: (value: number) => void;
    /** Fires when the drag ends. */
    onchange: (value: number) => void;
    id?: string;
    label?: string;
    disabled?: boolean;
    width?: string;
  }
  let {
    value,
    min,
    max,
    step = 1,
    oninput,
    onchange,
    id,
    label,
    disabled = false,
    width = '140px',
  }: Props = $props();
</script>

<input
  class="slider"
  type="range"
  style="width: {width}; --fill: {((value - min) / (max - min)) * 100}%"
  {id}
  {min}
  {max}
  {step}
  {value}
  {disabled}
  aria-label={label}
  oninput={(e) => oninput?.(Number(e.currentTarget.value))}
  onchange={(e) => onchange(Number(e.currentTarget.value))}
/>

<style>
  .slider {
    height: 16px;
    margin: 0;
    background: transparent;
    appearance: none;
    -webkit-appearance: none;
    cursor: default;
  }
  .slider:disabled {
    opacity: 0.45;
  }
  .slider::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      var(--accent) var(--fill),
      light-dark(rgba(0, 0, 0, 0.14), rgba(255, 255, 255, 0.2)) var(--fill)
    );
  }
  .slider::-webkit-slider-thumb {
    width: 16px;
    height: 16px;
    margin-top: -6px;
    border: 0;
    border-radius: 50%;
    background: #fff;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.3),
      0 0 0 0.5px rgba(0, 0, 0, 0.08);
    appearance: none;
    -webkit-appearance: none;
  }
</style>
