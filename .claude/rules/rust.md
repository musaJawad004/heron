---
paths:
  - "src-tauri/**/*.rs"
---
# Rust conventions (src-tauri)

- Edition 2021, stable toolchain, `cargo clippy` clean, `cargo fmt` (repo
  `rustfmt.toml`). No `unwrap()`/`expect()` outside tests and `setup`;
  propagate with `thiserror` enums or log-and-degrade. Never panic on data
  read from `~/.claude` — it drifts between CLI versions.
- Modules never import each other except through `model.rs`, `settings.rs`
  and `state.rs`. Read the `//!` contract at the top of a file before
  changing behaviour.
- Everything the UI sees is a `serde` struct with `rename_all = "camelCase"`
  mirrored in `src/lib/types.ts`. Change both or neither.
- Parse loose Claude Code JSON with `serde_json::Value` and `.get()`s, never
  strict structs (unknown/missing fields must not fail).
- Timestamps from Claude Code are Unix **milliseconds**; keep `u64` ms
  end-to-end (`model::now_ms`).
- Transcripts: read at most the first 64 KB with `File::take`, split on
  `\n`, ignore a trailing partial line.
- Background work runs on named `std::thread`s or `notify` callbacks and
  publishes through `AppState::publish`. Hold `parking_lot::Mutex` guards for
  microseconds; never call into Tauri (`emit`, windows) while holding one.
- Tauri commands are thin wrappers; logic lives in `state`/`launch`/`hooks`.
  Anything that can block (dialogs, `osascript`) runs on a non-main thread
  or an `async` command.
- Platform code is behind `#[cfg(target_os = "...")]` inside the module that
  owns the behaviour (`launch/macos.rs`, `launch/windows.rs`, `launch/linux.rs`);
  every platform must compile even when it just returns `false`/`Err`.
- Logging: `log::{info,warn,debug}` via `tauri-plugin-log`. Never log prompt
  text or full hook payloads.
- Tests: `#[cfg(test)]` next to the code, fixtures in `tempfile::tempdir()`,
  never the real `~/.claude`.
