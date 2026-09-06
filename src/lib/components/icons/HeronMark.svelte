<script lang="ts">
  // The Heron mark from design/heron-mark.svg, as a single-colour glyph.
  let { size = 16 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" focusable="false">
  <g fill="none" stroke="currentColor" stroke-width="5" stroke-linecap="round" stroke-linejoin="round">
    <path d="M24 50 C 24 38, 34 36, 36 26 C 37 20, 34 16, 32 13" />
    <path d="M32 13 L 46 9" />
  </g>
  <circle cx="31" cy="13" r="5" fill="currentColor" />
  <path
    fill="currentColor"
    d="M10 46 C 10 40, 18 38, 26 40 C 36 42, 40 48, 40 52 C 38 58, 26 60, 18 56 C 12 54, 10 50, 10 46 Z"
  />
  <g fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round">
    <path d="M24 58 L 24 62" />
    <path d="M32 58 L 34 62" />
  </g>
</svg>
