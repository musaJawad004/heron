// Mirrors src-tauri/src/model.rs and settings.rs field for field (camelCase).

export type SessionStatus = 'busy' | 'idle' | 'needs_input' | 'waiting' | 'unknown';

export type HookEventKind =
  | 'SessionStart'
  | 'SessionEnd'
  | 'Stop'
  | 'SubagentStop'
  | 'Notification'
  | 'PermissionRequest'
  | 'UserPromptSubmit'
  | 'PreToolUse'
  | 'PostToolUse'
  | 'Other';

export interface HookEvent {
  id: string;
  kind: HookEventKind;
  sessionId: string;
  cwd: string | null;
  transcriptPath: string | null;
  subtype: string | null;
  message: string | null;
  toolName: string | null;
  receivedAt: number;
}

export interface Session {
  id: string;
  name: string | null;
  title: string | null;
  cwd: string;
  pid: number | null;
  status: SessionStatus;
  startedAt: number | null;
  lastActiveAt: number;
  version: string | null;
  gitBranch: string | null;
  transcriptPath: string | null;
  tty: string | null;
  attention: HookEvent | null;
}

export type TerminalApp =
  | 'terminal'
  | 'iterm'
  | 'ghostty'
  | 'warp'
  | 'kitty'
  | 'alacritty'
  | 'wezterm'
  | 'windows-terminal'
  | 'powershell'
  | 'cmd'
  | 'gnome-terminal'
  | 'konsole'
  | 'xterm';

export interface TerminalChoice {
  id: TerminalApp;
  name: string;
}

export type HookStatus = 'installed' | 'not-installed' | 'partial';
export type PanelEdge = 'left' | 'right';

export interface PanelOrigin {
  x: number;
  y: number;
  monitor: string | null;
}

export interface Settings {
  terminalApp: TerminalApp | null;
  claudePath: string;
  extraClaudeArgs: string;
  defaultProjectsFolder: string;
  showPanel: boolean;
  panelEdge: PanelEdge;
  panelFollowsActiveScreen: boolean;
  panelOpacity: number;
  panelExpanded: boolean;
  panelOrigin: PanelOrigin | null;
  notifyOnPermission: boolean;
  notifyOnDone: boolean;
  notifyOnIdle: boolean;
  playSound: boolean;
  recentLimit: number;
  showCountInTray: boolean;
  launchAtLogin: boolean;
}

export interface Snapshot {
  running: Session[];
  recent: Session[];
  attention: Record<string, HookEvent>;
  eventLog: HookEvent[];
  hookStatus: HookStatus;
  claudePath: string | null;
  lastError: string | null;
  isRefreshingHistory: boolean;
  settings: Settings;
}

/** True when the human has to look at this session. */
export function needsAttention(s: Session): boolean {
  return s.status === 'needs_input' || s.status === 'waiting' || s.attention !== null;
}

/** Name, then title, then project folder. */
export function displayName(s: Session): string {
  return s.name || s.title || projectName(s.cwd);
}

export function projectName(cwd: string): string {
  const parts = cwd.replace(/[\\/]+$/, '').split(/[\\/]/);
  return parts[parts.length - 1] || cwd;
}
