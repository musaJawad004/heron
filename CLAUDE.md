# Heron

Cross-platform menu bar / system tray app that watches your **Claude Code
CLI** sessions: how many are running, which one is working / idle / waiting
for you, start a new session in any folder, resume a past one, and get a native
notification when Claude needs permission or finishes. Everything stays on the
machine — there is no network code in this repository at all.

## Stack

Tauri v2 (Rust core, native webview) + SvelteKit static / Svelte 5 runes /
TypeScript. Plain CSS with design tokens, no UI kit. Targets macOS 12+,
Windows 10/11, Linux. Unsigned local builds; no Apple developer identity needed.

## Commands

```bash
yarn install                       # once
yarn tauri dev                 # dev app with hot reload (tray appears)
cd src-tauri && cargo check && cargo test && cargo clippy && cargo fmt
yarn run check                     # svelte-check (strict TS)
yarn prettier -w src               # format frontend
yarn tauri build               # release bundle → src-tauri/target/release/bundle/
```

## Layout

```
src-tauri/src/
  lib.rs            Tauri builder: plugins, setup (tray, state, panel), window events
  main.rs           entry
  model.rs          shared types (Session, HookEvent, Snapshot, TerminalApp…) — mirrored in src/lib/types.ts
  settings.rs       Settings + SettingsStore (JSON in app-data dir)
  state.rs          AppState hub: registry poll, hook watcher, history refresh, actions, publish()
  commands.rs       #[tauri::command] IPC surface (thin)
  tray.rs           tray icon, count, menu
  windows.rs        popover / panel / settings window behaviour and placement
  claude/           paths.rs (per-OS ~/.claude), registry.rs (live sessions), transcripts.rs (history + titles)
  hooks/            installer.rs (settings.json), watcher.rs (spool dir), notify.rs
  launch/           terminals per OS, launch scripts, focus, find_claude
src/
  app.css           design tokens (light/dark via light-dark())
  lib/api.ts        the only file that talks to Tauri
  lib/stores.svelte.ts  runes store fed by the `snapshot` event
  lib/types.ts      TS mirror of model.rs / settings.rs
  lib/components/   shared UI
  routes/popover, routes/panel, routes/settings   one route per window
docs/ARCHITECTURE.md   data formats Heron reads · docs/HOOKS.md   hook payloads · docs/PRIVACY.md
```

Modules talk only through `model.rs`, `settings.rs` and `state.rs`. Read the
`//!` contract at the top of a file before changing its behaviour.

## Rules (auto-loaded from `.claude/rules/`)

- `privacy-and-security.md` — no network, read-only on `~/.claude`, never
  `*.key`, never persist prompt text, no shell strings, minimal capabilities.
- `rust.md` — lenient JSON parsing, ms timestamps, head-only transcripts,
  never hold a mutex across Tauri calls, cfg for all three OSes.
- `frontend.md` — Svelte 5 runes only, `api.ts` is the IPC boundary, tokens.
- `ui.md` — the design language. `testing-and-verification.md`, `git-and-workflow.md`.

## Skills (`.claude/skills/`)

Project: `heron-build`, `heron-claude-data`, `heron-tray-windows`.
Imported (see `THIRD-PARTY-LICENSES.md`): `rust-engineer`, `typescript-pro`,
`secure-code-guardian`, `code-reviewer`, `test-master`, `svelte5-init`,
`svelte5-review`, `frontend-design`, `tauri`, `tauri-app-system-tray`,
`tauri-window`, `tauri-app-notification`, `tauri-security`, `tauri-build`,
`tauri-app-positioner`, `tauri-app-autostart`. Load the relevant ones before
non-trivial work in that area.

## Agents (`.claude/agents/`)

`rust-reviewer`, `security-auditor`, `ui-polisher`, `app-tester`. Workflow
`review-changes` (`.claude/workflows/`) runs a four-dimension review with
adversarial verification.

## Hooks (`.claude/settings.json`)

SessionStart → repo state. PreToolUse(Bash) → blocks damage to `~/.claude`,
`.key` reads, publishing, force-push. PostToolUse(Edit/Write) → rustfmt /
prettier. Stop → refuses to end the turn while `cargo check` or `svelte-check` fail.

## Verifying for real

`yarn tauri dev`, run `claude` in a terminal folder → it shows in the
popover within 2 s; quit → gone within 2 s. Install hooks from Settings →
Hooks, then trigger a permission prompt in a session → notification.
