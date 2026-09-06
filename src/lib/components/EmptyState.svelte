<script lang="ts">
  let { message }: { message: string } = $props();
</script>

<p class="empty">{message}</p>

<style>
  .empty {
    margin: 0;
    padding: var(--space-2) var(--space-4) var(--space-3);
    font-size: var(--text-sm);
    line-height: 16px;
    color: var(--text-secondary);
  }
</style>
