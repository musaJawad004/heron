# Privacy & security (applies everywhere)

Heron is a local-only, open-source observer of Claude Code sessions. These are
hard rules, not preferences.

- **No network.** Never import or call `URLSession`, `Network`, sockets, or
  spawn `curl`/`wget`. No analytics, crash reporting, update checks, or
  telemetry. If a feature seems to need the network, it does not belong in Heron.
- **Read-only on `~/.claude`.** Heron reads `sessions/*.json`, `projects/**/*.jsonl`,
  `history.jsonl` and `settings.json`. The only write is `HookInstaller`
  merging its own hook entries into `settings.json` (after a timestamped
  backup, atomic write, removable byte-for-byte on uninstall).
- **Never read `~/.claude/sessions/*.key`** or the `messagingSocketPath`
  socket. Those are the CLI's private credentials.
- **Never persist prompt content.** `user_input`, `tool_input`, transcript
  message bodies and pasted text stay out of logs, caches, notifications and
  UserDefaults. A session *title* is the first prompt truncated to 80 chars
  and is the only prompt-derived text stored (in the cache under App Support).
- **Files Heron creates** under `~/Library/Application Support/Heron` are
  created with mode `0700` (dirs) / `0600` (files).
- **Shell safety.** Anything passed to a shell is quoted with a single
  `shellQuote()` helper (single quotes, `'` → `'\''`). Never build a shell
  string by interpolating a path, session id or argument directly. Prefer
  `Process` with an argument array over `sh -c`.
- **Hooks are observers.** The installed hook script must exit 0 with empty
  stdout in every case, never exit 2, and never block (`async: true`,
  short timeout).
- **No entitlements that grant broad access.** Accessibility, Full Disk
  Access, Screen Recording are never requested. Apple Events are used only to
  raise a terminal tab and only if the user opts in (TCC prompt).
- Dependencies: none. Foundation, AppKit, SwiftUI, UserNotifications,
  ServiceManagement only. Do not add a Swift package dependency without
  discussing it in an issue first.
