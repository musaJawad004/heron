<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" alt="Heron icon">
</p>

<h1 align="center">Heron</h1>

<p align="center">
  A quiet menu bar / tray app that watches your <b>Claude Code</b> sessions.<br>
  See how many are running, which one needs you, start or resume a session in any folder.<br>
  <b>macOS · Windows · Linux. Everything stays on your machine.</b>
</p>

<p align="center">
  <a href="LICENSE">MIT</a> ·
  Tauri v2 · Rust · Svelte 5 ·
  ~6 MB ·
  zero network
</p>

---

## What it does

- **Tray count** — a heron glyph with the number of live Claude Code sessions (menu bar on macOS, system tray on Windows and Linux). It grows a dot when a session is waiting on you.
- **Session list** — click the icon: running sessions with live status (Working / Idle / Needs you), project folder, git branch, first prompt as the title, and how long ago they were active. Past sessions underneath, ready to resume.
- **New session** — ⌘N / Ctrl+N, pick a folder, and Heron opens your terminal there with `claude` running. Terminal, iTerm2, Ghostty, Warp, kitty, Alacritty, WezTerm, Windows Terminal, PowerShell, GNOME Terminal and Konsole are supported.
- **Resume / Focus** — one click resumes a past session (`claude --resume`) or raises the terminal tab of a running one.
- **The vigil** — a small floating panel pinned to the edge of whichever screen you are on. It shows the count and one line per session, never steals focus, and can be dragged anywhere. Toggle it from the menu bar.
- **Notifications** — installs Claude Code hooks (with your consent, one click) so you get a native notification when Claude asks for permission, needs input, or finishes its turn. Click it to jump to that terminal.
- **Light and dark** — OS materials (vibrancy on macOS, Mica on Windows), semantic colours, system fonts. It looks like it shipped with the OS.

## Privacy

Heron has **no network code**. Not for telemetry, not for updates, not for anything. It reads the files the Claude Code CLI already writes under `~/.claude` (the live session registry, transcripts for titles, prompt history) and writes only to its own app-data folder plus, if you opt in, six hook entries in `~/.claude/settings.json` (backed up first, removed cleanly on uninstall). It never opens the CLI's private `*.key` files, never stores prompt or tool contents, and the hook it installs is a 20-line script you can read. Details in [docs/PRIVACY.md](docs/PRIVACY.md).

## Install

Download the bundle for your OS from the [Releases](https://github.com/glixentech/heron/releases) page (`.dmg`, `.msi`/`.exe`, `.AppImage`/`.deb`), or build it yourself:

```bash
git clone https://github.com/glixentech/heron.git
cd heron
npm install
npm run tauri build        # → src-tauri/target/release/bundle/
```

Prerequisites: Node 20+, Rust stable (`rustup`), and the Tauri system deps for your OS ([tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/)). Builds are **unsigned**: no Apple developer account or notarization is involved. On macOS a downloaded copy needs a one-time right-click → Open; a copy you built yourself launches directly.

Then click the heron in the tray → Settings → Hooks → **Install hooks** to enable notifications. That is the only step that touches `~/.claude/settings.json`.

## Build & hack

```bash
npm run tauri dev                      # dev app with hot reload
cd src-tauri && cargo test && cargo clippy
npm run check                          # svelte-check
```

Rust core in `src-tauri/`, Svelte 5 frontend in `src/`. The layout and the data formats Heron reads are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md); the hook payloads in [docs/HOOKS.md](docs/HOOKS.md).

If you use Claude Code to work on Heron, the repo ships a complete `.claude/` setup: rules, hooks (format on edit, refuse to stop on a broken build, guard against touching `~/.claude` data), four agents, a review workflow and sixteen imported Rust/Tauri/Svelte skills. See [CLAUDE.md](CLAUDE.md).

## How it works

Claude Code writes a small JSON file per running session in `~/.claude/sessions/` with its status. Heron polls that directory, checks that each pid is still alive, and that is the live list. Titles come from the first prompt in each transcript (only the first 64 KB is read). Notifications come from Claude Code hooks: the installed script spools each event as a file into Heron's app-data folder; Heron watches the folder and deletes each file after reading it.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The bar for a change is "does this make watching Claude Code sessions simpler or safer".

## License

[MIT](LICENSE) © Glixen Technologies. Imported Claude Code skills keep their own MIT notices in `.claude/skills/THIRD-PARTY-LICENSES.md`.
