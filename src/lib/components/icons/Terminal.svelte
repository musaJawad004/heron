<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <rect x="1.75" y="3" width="12.5" height="10" rx="1.5" />
  <path d="M4.5 6.5L6.5 8l-2 1.5M8 10h3.5" />
</Icon>
