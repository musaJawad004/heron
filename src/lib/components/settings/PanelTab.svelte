<script lang="ts">
  import { heron } from '$lib/stores.svelte';
  import type { PanelEdge } from '$lib/types';
  import { setSetting } from './update';
  import Field from '../Field.svelte';
  import Group from '../Group.svelte';
  import Segmented from '../Segmented.svelte';
  import Slider from '../Slider.svelte';
  import Toggle from '../Toggle.svelte';

  const s = $derived(heron.settings);
  const off = $derived(!s.showPanel);
  const edges: { value: PanelEdge; label: string }[] = [
    { value: 'left', label: 'Left' },
    { value: 'right', label: 'Right' },
  ];
  const isEdge = (v: string): v is PanelEdge => v === 'left' || v === 'right';
</script>

<Group>
  <Field label="Show floating panel" for="show-panel">
    <Toggle id="show-panel" checked={s.showPanel} onchange={(v) => setSetting('showPanel', v)} />
  </Field>
  <Field label="Edge">
    <span class:dim={off}>
      <Segmented
        label="Edge"
        options={edges}
        value={s.panelEdge}
        onchange={(v) => isEdge(v) && setSetting('panelEdge', v)}
      />
    </span>
  </Field>
  <Field
    label="Follow the active screen"
    for="follow-screen"
    caption="Moves to whichever screen your cursor is on"
  >
    <Toggle
      id="follow-screen"
      disabled={off}
      checked={s.panelFollowsActiveScreen}
      onchange={(v) => setSetting('panelFollowsActiveScreen', v)}
    />
  </Field>
  <Field label="Opacity" for="opacity">
    <span class="value" class:dim={off}>{Math.round(s.panelOpacity * 100)} %</span>
    <Slider
      id="opacity"
      min={50}
      max={100}
      disabled={off}
      value={Math.round(s.panelOpacity * 100)}
      oninput={(v) => (heron.snapshot.settings.panelOpacity = v / 100)}
      onchange={(v) => setSetting('panelOpacity', v / 100)}
    />
  </Field>
  <Field
    label="Start expanded"
    for="start-expanded"
    caption="Show a row per session instead of just the count"
  >
    <Toggle
      id="start-expanded"
      disabled={off}
      checked={s.panelExpanded}
      onchange={(v) => setSetting('panelExpanded', v)}
    />
  </Field>
</Group>

<style>
  .value {
    min-width: 40px;
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    text-align: right;
  }
  .dim {
    opacity: 0.45;
    pointer-events: none;
  }
</style>
