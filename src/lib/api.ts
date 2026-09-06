// The only file that talks to Tauri. Every backend command is wrapped here so
// components stay testable and the IPC surface is visible in one place.
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import type { HookEvent, Session, Settings, Snapshot, TerminalChoice } from './types';
import { mockSnapshot } from './mock';

/** True when the page runs inside the Tauri webview (not a plain browser). */
export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Outside Tauri (plain `npm run dev` in a browser) every command resolves to
// mock data so the UI can be developed and screenshotted without the app.
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  console.debug('[mock invoke]', cmd, args);
  switch (cmd) {
    case 'get_snapshot':
      return mockSnapshot as T;
    case 'get_settings':
      return mockSnapshot.settings as T;
    case 'update_settings':
      Object.assign(mockSnapshot.settings, (args?.patch as object) ?? {});
      return mockSnapshot.settings as T;
    case 'installed_terminals':
      return [
        { id: 'terminal', name: 'Terminal' },
        { id: 'iterm', name: 'iTerm2' },
      ] as T;
    case 'detect_claude':
      return '/Users/me/.local/bin/claude' as T;
    case 'pick_folder':
      return '/Users/me/dev/example' as T;
    default:
      return undefined as T;
  }
}

async function listen<T>(event: string, cb: (e: { payload: T }) => void): Promise<UnlistenFn> {
  if (inTauri) return tauriListen<T>(event, cb);
  return () => {};
}

export const api = {
  // State
  getSnapshot: () => invoke<Snapshot>('get_snapshot'),
  refreshHistory: () => invoke<void>('refresh_history'),
  clearError: () => invoke<void>('clear_error'),

  // Sessions
  /** Opens the folder picker when `folder` is omitted. */
  newSession: (folder?: string) => invoke<void>('new_session', { folder: folder ?? null }),
  resumeSession: (id: string) => invoke<void>('resume_session', { id }),
  focusSession: (id: string) => invoke<void>('focus_session', { id }),
  terminateSession: (id: string) => invoke<void>('terminate_session', { id }),
  revealSession: (id: string) => invoke<void>('reveal_session', { id }),

  // Settings
  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (patch: Partial<Settings>) => invoke<Settings>('update_settings', { patch }),
  installedTerminals: () => invoke<TerminalChoice[]>('installed_terminals'),
  detectClaude: () => invoke<string | null>('detect_claude'),
  pickFolder: () => invoke<string | null>('pick_folder'),
  setLaunchAtLogin: (enabled: boolean) => invoke<void>('set_launch_at_login', { enabled }),

  // Hooks
  installHooks: () => invoke<void>('install_hooks'),
  uninstallHooks: () => invoke<void>('uninstall_hooks'),
  testNotification: () => invoke<void>('test_notification'),
  revealClaudeSettings: () => invoke<void>('reveal_claude_settings'),
  revealAppData: () => invoke<void>('reveal_app_data'),

  // Windows
  hidePopover: () => invoke<void>('hide_popover'),
  togglePanel: () => invoke<void>('toggle_panel'),
  openSettings: () => invoke<void>('open_settings'),
  panelResized: (width: number, height: number) => invoke<void>('panel_resized', { width, height }),
  panelMoved: () => invoke<void>('panel_moved'),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  quit: () => invoke<void>('quit'),

  // Events pushed from Rust
  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>('snapshot', (e) => cb(e.payload)),
  onHookEvent: (cb: (e: HookEvent) => void): Promise<UnlistenFn> =>
    listen<HookEvent>('hook-event', (e) => cb(e.payload)),
};

export type { Session, Snapshot, Settings, HookEvent, TerminalChoice };
