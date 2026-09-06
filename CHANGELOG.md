# Changelog

## 0.1.0 — 2026-09-06

First release. macOS, Windows and Linux.

### Added
- Menu bar / tray icon carrying the number of live Claude Code sessions, with a
  dot when one needs an answer from you.
- Session list: running sessions with live status, project folder, git branch
  and the first prompt as the title; past sessions ready to resume.
- Start a session in any folder, resume a past one, or focus the terminal tab of
  a running one.
- A floating panel pinned to the edge of the active screen that never takes focus.
- Native notifications when Claude asks for permission or finishes a turn.
  Clicking one raises that session's terminal.
- Settings for terminal choice, the `claude` executable, panel placement,
  notifications, and hook installation.
- Two supply-chain scanners, run in CI on every push and daily, that also fail
  the build if any network code appears.

### Notes
- Builds are unsigned. macOS and Windows both warn on first launch.
- Notifications require the hooks, installed from Settings with one click after
  a timestamped backup of `~/.claude/settings.json`.
- Nothing leaves the machine. The only prompt-derived text stored anywhere is a
  session title truncated to 80 characters.
