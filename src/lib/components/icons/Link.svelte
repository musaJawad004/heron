<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <path d="M6.6 9.4l2.8-2.8" />
  <path d="M8.9 4.6l1.2-1.2a2.2 2.2 0 0 1 3.1 3.1l-1.2 1.2" />
  <path d="M7.1 11.4l-1.2 1.2a2.2 2.2 0 0 1-3.1-3.1L4 8.3" />
</Icon>
