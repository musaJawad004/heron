<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <path
    d="M12.7 6.7L14.4 7L14.4 9L12.7 9.3L12.2 10.5L13.2 11.9L11.9 13.2L10.5 12.2L9.3 12.7L9 14.4L7 14.4L6.7 12.7L5.5 12.2L4.1 13.2L2.8 11.9L3.8 10.5L3.3 9.3L1.6 9L1.6 7L3.3 6.7L3.8 5.5L2.8 4.1L4.1 2.8L5.5 3.8L6.7 3.3L7 1.6L9 1.6L9.3 3.3L10.5 3.8L11.9 2.8L13.2 4.1L12.2 5.5Z"
  />
  <circle cx="8" cy="8" r="2.2" />
</Icon>
