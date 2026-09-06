<script lang="ts">
  // A path on one line: the home dir becomes `~`, the leading part is
  // ellipsised, and the last segment always stays visible.
  import { shortenHome, splitPath } from '$lib/format';

  let { path }: { path: string } = $props();
  const parts = $derived(splitPath(shortenHome(path)));
</script>

<span class="path" title={shortenHome(path)}>
  {#if parts.head}<span class="head">{parts.head}</span>{/if}<span class="tail">{parts.tail}</span>
</span>

<style>
  .path {
    display: inline-flex;
    max-width: 100%;
    min-width: 0;
    white-space: nowrap;
  }
  .head {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tail {
    flex: none;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
