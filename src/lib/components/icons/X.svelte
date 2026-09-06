<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></Icon>
