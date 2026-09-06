# Testing & verification

- `cargo check` and `yarn run check` must pass before a turn ends (Stop hook).
  `cargo test` and `cargo clippy` before committing.
- Core Rust modules (`claude::registry`, `claude::transcripts`,
  `hooks::watcher::parse`, `hooks::installer`, `launch::shell_quote`) have unit
  tests against fixtures in `tempfile::tempdir()`; the fixture line shapes
  live in `docs/ARCHITECTURE.md`.
- Real verification: `yarn tauri dev`, then in a terminal run `claude` in
  some folder → it shows in the popover within 2 s; quit it → gone within 2 s.
- Hook smoke test after "Install hooks": on macOS
  `echo '{"hook_event_name":"Stop","session_id":"test","cwd":"/tmp"}' | "$HOME/Library/Application Support/com.muhammadmusadev.heron/bin/heron-hook"`
  → the event appears in Settings → Hooks. Windows: pipe the same JSON into
  `powershell -File %APPDATA%\com.muhammadmusadev.heron\bin\heron-hook.ps1`.
- Never test against real `~/.claude` data in unit tests; never delete
  anything under `~/.claude`.
