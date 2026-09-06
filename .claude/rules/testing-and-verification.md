# Testing & verification

- `swift build` must pass before a turn ends (a Stop hook enforces it).
- Core modules (`SessionRegistry`, `TranscriptIndex`, `HookEventWatcher.parse`,
  `HookInstaller`) get unit tests against fixture files in a temp dir.
- To verify UI or integration for real: `make app && open dist/Heron.app`, then
  open a Terminal and run `claude` in some folder; the session must appear in
  the menu bar within 2 s. Quit the CLI; it must disappear within 2 s.
  Use `pkill -x Heron` before rebuilding.
- Hook path smoke test: after "Install hooks" in Settings, run
  `echo '{"hook_event_name":"Stop","session_id":"test","cwd":"/tmp"}' | "$HOME/Library/Application Support/Heron/bin/heron-hook"`
  and confirm the event shows in the Settings → Hooks event log.
- Never test against real `~/.claude` data in unit tests; never delete
  anything under `~/.claude`.
