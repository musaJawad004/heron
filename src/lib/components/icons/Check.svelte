<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}><path d="M3.25 8.5l3 3 6.5-7" /></Icon>
