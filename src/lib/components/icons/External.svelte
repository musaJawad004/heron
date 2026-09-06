<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <path d="M9.5 2.75h3.75V6.5M13 3L7.75 8.25" />
  <path d="M11 9.5v3a.75.75 0 0 1-.75.75h-6.5A.75.75 0 0 1 3 12.5V6a.75.75 0 0 1 .75-.75h3" />
</Icon>
