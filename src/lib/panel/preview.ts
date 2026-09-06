// Browser-only `?state=` overrides so every panel state can be screenshotted
// from `npm run dev` without touching the mock. Inert inside Tauri.
//   /panel?state=empty      no running sessions
//   /panel?state=collapsed  collapsed capsule
//   /panel?state=many       twelve sessions (exercises "+N more")
//   &menu=header|row        open the header menu or the first row's menu
//   &bg                     paint a desktop-like gradient behind the capsule
//   &scheme=light|dark      force the colour scheme (headless Chrome follows the OS)
import { inTauri } from '$lib/api';
import { heron } from '$lib/stores.svelte';

export interface Preview {
  state: 'empty' | 'collapsed' | 'many' | null;
  menu: 'header' | 'row' | null;
}
const states: readonly string[] = ['empty', 'collapsed', 'many'];

/** Call once after `heron.init()`; mutates the mock snapshot in place. */
export function applyPreview(): Preview {
  const none: Preview = { state: null, menu: null };
  if (inTauri || typeof location === 'undefined') return none;
  const params = new URLSearchParams(location.search);
  const scheme = params.get('scheme');
  if (scheme === 'light' || scheme === 'dark') document.documentElement.style.colorScheme = scheme;
  if (params.has('bg')) {
    document.body.style.background =
      'linear-gradient(135deg, light-dark(#8fb3dc, #26324a), light-dark(#d3bfe4, #463a5c))';
  }

  const menu = params.get('menu');
  const state = params.get('state');
  const preview: Preview = {
    state: state && states.includes(state) ? (state as Preview['state']) : null,
    menu: menu === 'header' || menu === 'row' ? menu : null,
  };
  const snap = heron.snapshot;
  switch (preview.state) {
    case 'empty':
      snap.running = [];
      snap.attention = {};
      break;
    case 'collapsed':
      snap.settings.panelExpanded = false;
      break;
    case 'many': {
      const base = snap.running;
      const extra = Array.from({ length: 9 }, (_, i) => ({
        ...base[i % base.length],
        id: `preview-${i}`,
        cwd: `/Users/me/dev/project-${i + 1}`,
        status: 'idle' as const,
        attention: null,
      }));
      snap.running = [...base, ...extra];
      break;
    }
  }
  return preview;
}
