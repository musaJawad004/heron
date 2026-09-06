# Privacy & security (applies everywhere)

Heron is a local-only, open-source observer of Claude Code sessions. These are
hard rules, not preferences.

- **No network.** No `reqwest`, `hyper`, `tokio::net`, `std::net`, WebSocket,
  `fetch()` in the webview, no `tauri-plugin-http`/`updater`, no analytics,
  crash reporting or update checks. The CSP in `tauri.conf.json` allows only
  `'self'` and the IPC origin; keep it that way. The only "network-ish" act is
  handing an `https://` URL to the OS browser from the About page.
- **Read-only on `~/.claude`.** Heron reads `sessions/*.json`,
  `projects/**/*.jsonl` (head only), `history.jsonl` and `settings.json`. The
  only write is `hooks::installer` merging its own hook entries into
  `settings.json` after a timestamped backup, atomically, removable exactly.
- **Never read `~/.claude/sessions/*.key`** or connect to the CLI's
  messaging socket. Those are private credentials.
- **Never persist prompt content.** `user_input`, `tool_input`, transcript
  bodies and pasted text stay out of logs, caches, notifications, settings and
  the event log. A session *title* (first prompt, 80 chars) is the only
  prompt-derived text stored, in the title cache.
- **Files Heron creates** under its app-data dir are `0700` (dirs) / `0600`
  (files) on Unix.
- **No shell strings.** Spawn processes with `std::process::Command` and
  argument arrays. When a value must go inside a generated script, pass it
  through `launch::shell_quote` (POSIX) or the PowerShell equivalent. Never
  format a path, session id or argument into a shell string unquoted.
- **Hooks are observers.** The installed hook script exits 0 with no stdout in
  every case, never blocks (`async: true`, short timeout), never exits 2.
- **Tauri capabilities stay minimal.** The frontend gets only the permissions
  in `src-tauri/capabilities/default.json`; commands validate their inputs
  (`open_url` accepts `https://` only). No `shell:allow-execute`, no `fs`
  plugin exposed to the webview.
- **Dependencies** are added only with a reason in the PR; prefer std. Run
  `cargo tree` after adding one and make sure nothing network-capable came in.
