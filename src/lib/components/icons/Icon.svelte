<script lang="ts">
  // Base for every line icon: 16 px grid, currentColor, 1.5 stroke, round joins.
  import type { Snippet } from 'svelte';
  import type { SVGAttributes } from 'svelte/elements';

  interface Props extends SVGAttributes<SVGSVGElement> {
    size?: number;
    children: Snippet;
  }
  let { size = 16, children, ...rest }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
  {...rest}
>
  {@render children()}
</svg>
