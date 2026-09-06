// The only file that talks to Tauri. Every backend command is wrapped here so
// components stay testable and the IPC surface is visible in one place.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { HookEvent, Session, Settings, Snapshot, TerminalChoice } from './types';

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
