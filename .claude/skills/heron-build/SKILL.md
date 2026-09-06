---
name: heron-build
description: Build, run, test, lint and package Heron (Tauri v2 + Svelte 5 + Rust) on macOS, Windows and Linux — dev loop, release bundles, logs, and the gotchas of a tray app with transparent windows. Use for "run it", "build the app", "why won't it start", or before verifying a change for real.
---
# Building and running Heron

Heron is a Tauri v2 app: Rust core in `src-tauri/`, SvelteKit (static, no SSR)
frontend in `src/`. No Apple developer identity or notarization is needed for
local use; release bundles are unsigned unless you add your own keys.

| Task | Command |
|---|---|
| Dev loop (hot reload, tray appears) | `npm run tauri dev` |
| Rust type-check / tests / lint | `cd src-tauri && cargo check && cargo test && cargo clippy` |
| Frontend type-check | `npm run check` |
| Format | `cd src-tauri && cargo fmt` · `npx prettier -w src` |
| Release bundle for this OS | `npm run tauri build` → `src-tauri/target/release/bundle/` |
| macOS logs | `log stream --predicate 'process == "heron"' --style compact` or the log file under `~/Library/Logs/com.glixentech.heron/` |
| Windows/Linux logs | `%APPDATA%\com.glixentech.heron\logs\` / `~/.local/share/com.glixentech.heron/logs/` |

Gotchas
- The tray is the app: there is no main window. Quit via the tray menu or
  `pkill -x heron` (dev) / `pkill -x Heron` (bundle).
- Transparent + `windowEffects` windows require `macOSPrivateApi: true`
  (already set). The webview body must stay `background: transparent`.
- The popover hides on blur; while debugging keep DevTools attached
  (`Cmd+Opt+I` in dev) or it will vanish when you click elsewhere.
- The panel window is `focusable: false`; buttons inside still get clicks,
  but inputs never get keyboard focus by design.
- Windows: `npm run tauri build` needs the MSVC toolchain and WebView2 (preinstalled on Win 11).
- Linux: needs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev` for tray + bundling.
- If a stale `heron` dev process is running, the single-instance plugin just
  raises it; kill it first when testing startup paths.
