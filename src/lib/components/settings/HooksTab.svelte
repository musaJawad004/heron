<script lang="ts">
  import { api } from '$lib/api';
  import { heron } from '$lib/stores.svelte';
  import type { HookStatus } from '$lib/types';
  import Button from '../Button.svelte';
  import EventLog from './EventLog.svelte';

  let pending = $state(false);

  const status = $derived(heron.snapshot.hookStatus);
  const badge: Record<HookStatus, { label: string; tone: string }> = {
    installed: { label: 'Installed', tone: 'busy' },
    partial: { label: 'Partially installed', tone: 'attention' },
    'not-installed': { label: 'Not installed', tone: 'idle' },
  };

  async function run(action: () => Promise<void>) {
    pending = true;
    try {
      await action();
    } finally {
      pending = false;
    }
  }
</script>

<div class="status" aria-live="polite">
  <span class="dot {badge[status].tone}"></span>
  <span class="label">{badge[status].label}</span>
</div>

<p class="explain">
  Installing adds six hook entries to <code>~/.claude/settings.json</code> — SessionStart, SessionEnd, Stop, Notification,
  PermissionRequest and UserPromptSubmit — so Claude Code can tell Heron when a session starts, finishes a turn,
  or needs you. A timestamped backup of settings.json is taken first, and uninstalling removes exactly those entries.
  The hook script only writes small event files under Heron's app-data folder; nothing leaves this machine.
</p>

<div class="actions">
  <Button
    variant={status === 'installed' ? 'default' : 'primary'}
    disabled={pending}
    onclick={() => run(api.installHooks)}
  >
    {status === 'installed' ? 'Reinstall' : 'Install'}
  </Button>
  <Button disabled={pending || status === 'not-installed'} onclick={() => run(api.uninstallHooks)}
    >Uninstall</Button
  >
  <span class="spacer"></span>
  <Button variant="quiet" onclick={() => api.revealClaudeSettings()}>Show settings.json</Button>
</div>

<EventLog events={heron.snapshot.eventLog} />

<style>
  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--status-idle);
  }
  .dot.busy {
    background: var(--status-busy);
  }
  .dot.attention {
    background: var(--status-attention);
  }
  .label {
    font-size: var(--text-md);
    font-weight: 600;
  }
  .explain {
    margin: 0 0 var(--space-3);
    font-size: var(--text-sm);
    line-height: 17px;
    color: var(--text-secondary);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .spacer {
    flex: 1;
  }
</style>
