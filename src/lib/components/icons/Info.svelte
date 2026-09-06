<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <circle cx="8" cy="8" r="6" />
  <path d="M8 7.5V11M8 5.2v.1" />
</Icon>
