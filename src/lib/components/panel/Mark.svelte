<script lang="ts">
  // The heron mark from design/heron-mark.svg, cropped to its own bounds and
  // with heavier strokes (no legs) so it still reads at 14 px. Inherits
  // `currentColor`; ignores the pointer so the wrapping drag region gets the
  // mousedown.
  let { height = 14 }: { height?: number } = $props();
</script>

<svg viewBox="7 5 42 58" width={Math.round(height * 1.15)} {height} aria-hidden="true" focusable="false">
  <g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round">
    <path d="M24 50 C 24 38, 34 36, 36 26 C 37 20, 34 16, 32 13" stroke-width="7" />
    <path d="M32 13 L 46 9" stroke-width="6" />
  </g>
  <circle cx="31" cy="13" r="6" fill="currentColor" />
  <path
    fill="currentColor"
    d="M10 46 C 10 40, 18 38, 26 40 C 36 42, 40 48, 40 52 C 38 58, 26 60, 18 56 C 12 54, 10 50, 10 46 Z"
  />
</svg>

<style>
  svg {
    display: block;
    flex: none;
    pointer-events: none;
  }
</style>
