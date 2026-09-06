# Git & workflow

- Small, focused commits; imperative subject; body explains *why*.
- Branch per feature/module; `main` stays green (`cargo check`, `cargo test`,
  `cargo clippy`, `yarn run check`).
- Do not commit `build/`, `node_modules/`, `src-tauri/target/`,
  `src-tauri/gen/`, or anything from your home directory.
- Version lives in three places and must match: `package.json`,
  `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`. Add a `CHANGELOG.md`
  entry for user-visible changes.
- Releases are built by CI (`.github/workflows/`) for macOS, Windows and
  Linux; never publish from a local machine.
