<script lang="ts">
  import Icon from './Icon.svelte';
  let { size = 16 }: { size?: number } = $props();
</script>

<Icon {size}>
  <path d="M4 10.5V7.3a4 4 0 0 1 8 0v3.2l1.2 1.7H2.8z" />
  <path d="M6.6 13.6a1.5 1.5 0 0 0 2.8 0" />
</Icon>
