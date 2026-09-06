<script lang="ts">
  // The floating panel: a screen with a small capsule docked at its edge.
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <rect x="1.75" y="3" width="12.5" height="10" rx="1.5" />
  <rect x="3.75" y="5" width="3" height="4" rx="1.5" fill="currentColor" stroke="none" />
</Icon>
