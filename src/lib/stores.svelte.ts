// Runes-based app state shared by every window. Each window has its own
// webview, so each calls `heron.init()` once; Rust pushes a full `snapshot`
// event on every change, which keeps the three windows trivially in sync.
import { api } from './api';
import type { Settings, Snapshot } from './types';

const empty: Snapshot = {
  running: [],
  recent: [],
  attention: {},
  eventLog: [],
  hookStatus: 'not-installed',
  claudePath: null,
  lastError: null,
  isRefreshingHistory: false,
  settings: {
    terminalApp: null,
    claudePath: '',
    extraClaudeArgs: '',
    defaultProjectsFolder: '',
    showPanel: true,
    panelEdge: 'left',
    panelFollowsActiveScreen: true,
    panelOpacity: 1,
    panelExpanded: true,
    panelOrigin: null,
    notifyOnPermission: true,
    notifyOnDone: true,
    notifyOnIdle: false,
    playSound: true,
    recentLimit: 20,
    showCountInTray: true,
    launchAtLogin: false,
  },
};

class HeronStore {
  snapshot = $state<Snapshot>(empty);
  ready = $state(false);

  running = $derived(this.snapshot.running);
  recent = $derived(this.snapshot.recent.filter((s) => !this.snapshot.running.some((r) => r.id === s.id)));
  settings = $derived(this.snapshot.settings);
  runningCount = $derived(this.snapshot.running.length);
  attentionCount = $derived(
    this.snapshot.running.filter(
      (s) => s.status === 'needs_input' || s.status === 'waiting' || s.id in this.snapshot.attention,
    ).length,
  );
  busyCount = $derived(this.snapshot.running.filter((s) => s.status === 'busy').length);

  private unlisten: (() => void) | null = null;

  async init() {
    if (this.unlisten) return;
    this.unlisten = await api.onSnapshot((s) => {
      this.snapshot = s;
    });
    try {
      this.snapshot = await api.getSnapshot();
    } finally {
      this.ready = true;
    }
  }

  async update(patch: Partial<Settings>) {
    const settings = await api.updateSettings(patch);
    this.snapshot = { ...this.snapshot, settings };
  }
}

export const heron = new HeronStore();
