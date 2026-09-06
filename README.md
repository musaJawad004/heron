<p align="center">
  <img src="Resources/AppIcon-256.png" width="128" alt="Heron icon">
</p>

<h1 align="center">Heron</h1>

<p align="center">
  A quiet macOS menu bar app that watches your <b>Claude Code</b> sessions.<br>
  See how many are running, which one needs you, start or resume a session in any folder.<br>
  <b>Everything stays on your Mac.</b>
</p>

<p align="center">
  <a href="LICENSE">MIT</a> ·
  macOS 14+ ·
  Swift 6 / SwiftUI ·
  zero dependencies ·
  zero network
</p>

---

## What it does

- **Menu bar count** — a heron glyph with the number of live Claude Code sessions. It fills in while Claude is working and grows a dot when a session is waiting on you.
- **Session list** — click the icon: running sessions with live status (Working / Idle / Needs you), project folder, git branch, first prompt as the title, and how long ago they were active. Past sessions underneath, ready to resume.
- **New session** — ⌘N, pick a folder, and Heron opens your terminal there with `claude` running. Terminal, iTerm2, Ghostty, Warp, kitty, Alacritty and WezTerm are supported.
- **Resume / Focus** — one click resumes a past session (`claude --resume`) or raises the terminal tab of a running one.
- **The vigil** — a small floating panel pinned to the edge of whichever screen you are on. It shows the count and one line per session, never steals focus, and can be dragged anywhere. Toggle it from the menu bar.
- **Notifications** — installs Claude Code hooks (with your consent, one click) so you get a native notification when Claude asks for permission, needs input, or finishes its turn. Click it to jump to that terminal.
- **Light and dark** — system materials, SF Symbols, semantic colours. It looks like it shipped with macOS.

## Privacy

Heron has **no network code**. Not for telemetry, not for updates, not for anything. It reads the files the Claude Code CLI already writes under `~/.claude` (the live session registry, transcripts for titles, prompt history) and writes only to `~/Library/Application Support/Heron` plus, if you opt in, six hook entries in `~/.claude/settings.json` (backed up first, removed cleanly on uninstall). It never opens the CLI's private `*.key` files, never stores prompt or tool contents, and the hook it installs is a 20-line shell script you can read. Details in [docs/PRIVACY.md](docs/PRIVACY.md).

## Install

Heron is built from source and **ad-hoc signed** — no Apple developer account, no notarization, nothing uploaded. You need Xcode 16+ (for the Swift 6 toolchain).

```bash
git clone https://github.com/glixentech/heron.git
cd heron
make install        # builds dist/Heron.app and copies it to /Applications
open /Applications/Heron.app
```

Because the app is not notarized, the first launch of a copy you did **not** build yourself needs a right-click → Open. A copy you built locally launches directly.

Then click the heron in the menu bar → Settings → Hooks → **Install hooks** to enable notifications. That is the only step that touches `~/.claude/settings.json`.

## Build & hack

```bash
swift build          # compile
swift test           # unit tests (fixtures only; never touches ~/.claude)
make app             # dist/Heron.app
make run             # build and launch
log stream --predicate 'subsystem == "com.glixentech.heron"' --style compact
```

No Xcode project: it is a plain Swift package. `Scripts/build-app.sh` assembles the bundle. The layout and the data formats Heron reads are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md); the hook payloads in [docs/HOOKS.md](docs/HOOKS.md).

If you use Claude Code to work on Heron, the repo ships a complete `.claude/` setup: rules, hooks (format on edit, refuse to stop on a broken build, guard against touching `~/.claude` data), four agents, a review workflow and fifteen imported Swift/macOS skills. See [CLAUDE.md](CLAUDE.md).

## How it works

Claude Code writes a small JSON file per running session in `~/.claude/sessions/` with its status. Heron watches that directory, checks that each pid is still alive, and that is the live list. Titles come from the first prompt in each transcript (only the first 64 KB is read). Notifications come from Claude Code hooks: the installed script spools each event as a file into Heron's Application Support folder; Heron watches the folder and deletes each file after reading it.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The bar for a change is "does this make watching Claude Code sessions simpler or safer".

## License

[MIT](LICENSE) © Glixen Technologies. Imported Claude Code skills keep their own MIT notices in `.claude/skills/THIRD-PARTY-LICENSES.md`.
