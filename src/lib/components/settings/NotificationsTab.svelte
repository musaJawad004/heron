<script lang="ts">
  import { api } from '$lib/api';
  import { heron } from '$lib/stores.svelte';
  import { setSetting } from './update';
  import Button from '../Button.svelte';
  import Callout from '../Callout.svelte';
  import Field from '../Field.svelte';
  import Group from '../Group.svelte';
  import Toggle from '../Toggle.svelte';

  let { onShowHooks }: { onShowHooks: () => void } = $props();

  const s = $derived(heron.settings);
  const hooksMissing = $derived(heron.ready && heron.snapshot.hookStatus !== 'installed');
</script>

{#if hooksMissing}
  <div class="callout">
    <Callout
      message="Notifications are off until the hooks are installed"
      actionLabel="Open Hooks"
      onAction={onShowHooks}
    />
  </div>
{/if}

<Group title="Notify me about">
  <Field
    label="Permission requests"
    for="notify-permission"
    caption="Claude is waiting for you to approve a tool"
  >
    <Toggle
      id="notify-permission"
      checked={s.notifyOnPermission}
      onchange={(v) => setSetting('notifyOnPermission', v)}
    />
  </Field>
  <Field label="Finished turns" for="notify-done" caption="Claude stopped and is waiting for your reply">
    <Toggle id="notify-done" checked={s.notifyOnDone} onchange={(v) => setSetting('notifyOnDone', v)} />
  </Field>
  <Field label="Idle prompts" for="notify-idle" caption="Claude waited 60 s for input">
    <Toggle id="notify-idle" checked={s.notifyOnIdle} onchange={(v) => setSetting('notifyOnIdle', v)} />
  </Field>
  <Field label="Play sound" for="play-sound">
    <Toggle id="play-sound" checked={s.playSound} onchange={(v) => setSetting('playSound', v)} />
  </Field>
</Group>

<div class="test">
  <Button onclick={() => api.testNotification()}>Send test notification</Button>
  <p class="caption">Notifications need the hooks to be installed.</p>
</div>

<style>
  .callout {
    margin: 0 calc(-1 * var(--space-2)) var(--space-3);
  }
  .test {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-3);
  }
  .caption {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
</style>
