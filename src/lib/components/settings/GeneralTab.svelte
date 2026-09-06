<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { heron } from '$lib/stores.svelte';
  import { middleEllipsis, shortenHome } from '$lib/format';
  import type { TerminalChoice } from '$lib/types';
  import { setSetting } from './update';
  import Button from '../Button.svelte';
  import Field from '../Field.svelte';
  import Group from '../Group.svelte';
  import Select from '../Select.svelte';
  import TextField from '../TextField.svelte';
  import Toggle from '../Toggle.svelte';

  let terminals = $state<TerminalChoice[]>([]);
  let detectedClaude = $state<string | null>(null);

  onMount(async () => {
    [terminals, detectedClaude] = await Promise.all([api.installedTerminals(), api.detectClaude()]);
  });

  const s = $derived(heron.settings);
  const terminalOptions = $derived([
    { value: '', label: 'Automatic' },
    ...terminals.map((t) => ({ value: t.id, label: t.name })),
  ]);

  function chooseTerminal(value: string) {
    setSetting('terminalApp', terminals.find((t) => t.id === value)?.id ?? null);
  }

  async function chooseFolder() {
    const folder = await api.pickFolder();
    if (folder) setSetting('defaultProjectsFolder', folder);
  }

  async function setLaunchAtLogin(enabled: boolean) {
    await api.setLaunchAtLogin(enabled);
    setSetting('launchAtLogin', enabled);
  }
</script>

<Group title="Launching">
  <Field label="Terminal" for="terminal">
    <Select id="terminal" options={terminalOptions} value={s.terminalApp ?? ''} onchange={chooseTerminal} />
  </Field>
  <Field label="Claude executable" for="claude-path" stacked caption="Leave empty to auto-detect">
    <TextField
      id="claude-path"
      mono
      value={s.claudePath}
      placeholder={detectedClaude ?? 'claude'}
      onchange={(v) => setSetting('claudePath', v.trim())}
    />
  </Field>
  <Field
    label="Extra arguments"
    for="extra-args"
    stacked
    caption="Appended to every launch, e.g. --model sonnet"
  >
    <TextField
      id="extra-args"
      mono
      value={s.extraClaudeArgs}
      placeholder="none"
      onchange={(v) => setSetting('extraClaudeArgs', v.trim())}
    />
  </Field>
  <Field
    label="Default folder for new sessions"
    caption={s.defaultProjectsFolder ? undefined : 'Asks for a folder each time'}
  >
    {#if s.defaultProjectsFolder}
      <span class="path" title={s.defaultProjectsFolder}>
        {middleEllipsis(shortenHome(s.defaultProjectsFolder), 26)}
      </span>
      <Button variant="quiet" onclick={() => setSetting('defaultProjectsFolder', '')}>Clear</Button>
    {/if}
    <Button onclick={chooseFolder}>Choose…</Button>
  </Field>
</Group>

<Group title="System">
  <Field label="Launch at login" for="launch-at-login">
    <Toggle id="launch-at-login" checked={s.launchAtLogin} onchange={setLaunchAtLogin} />
  </Field>
  <Field label="Show count in tray" for="show-count">
    <Toggle id="show-count" checked={s.showCountInTray} onchange={(v) => setSetting('showCountInTray', v)} />
  </Field>
  <Field label="Recent sessions to show" for="recent-limit">
    <TextField
      id="recent-limit"
      type="number"
      min={5}
      max={50}
      width="56px"
      value={String(s.recentLimit)}
      onchange={(v) => setSetting('recentLimit', Number(v))}
    />
  </Field>
</Group>

<style>
  .path {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    white-space: nowrap;
  }
</style>
