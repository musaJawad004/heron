<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}><path d="M8 2v6M4.3 4.6a5.2 5.2 0 1 0 7.4 0" /></Icon>
