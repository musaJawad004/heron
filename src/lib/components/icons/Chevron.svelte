<script lang="ts">
  import Icon from './Icon.svelte';
  type Direction = 'down' | 'up' | 'left' | 'right';
  let { size = 16, direction = 'down' }: { size?: number; direction?: Direction } = $props();
  const rotation: Record<Direction, number> = { down: 0, left: 90, up: 180, right: 270 };
</script>

<Icon {size} style="transform: rotate({rotation[direction]}deg)"><path d="M4.5 6.5L8 10l3.5-3.5" /></Icon>
