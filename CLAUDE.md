# Heron

Native macOS menu bar app that watches your **Claude Code CLI** sessions:
how many are running, which one is working / idle / waiting for you, start a
new session in any folder, resume a past one, and get a local notification when
Claude needs permission or finishes. Everything stays on this Mac — there is no
network code in this repository at all.

## Stack

Swift 6 toolchain (language mode 5, strict-concurrency warnings), SwiftUI +
AppKit, Observation framework, Swift Package Manager (no `.xcodeproj`),
macOS 14+. Zero third-party dependencies. Ad-hoc code signing; runs locally
without an Apple developer identity.

## Commands

```bash
swift build            # compile
swift test             # unit tests (fixtures only, never touches ~/.claude)
make app               # → dist/Heron.app (ad-hoc signed)
pkill -x Heron; make run
make install           # copy to /Applications
log stream --predicate 'subsystem == "com.glixentech.heron"' --style compact
```

## Layout

```
Sources/Heron/App/      HeronApp (@main: MenuBarExtra + Settings), AppState (the only object views talk to)
Sources/Heron/Core/     Models (shared contract), SessionRegistry (live sessions), TranscriptIndex (history)
Sources/Heron/Hooks/    HookInstaller (settings.json), HookEventWatcher (spool dir), Notifier
Sources/Heron/Launch/   TerminalLauncher (new / resume / focus in Terminal, iTerm2, Ghostty…)
Sources/Heron/Settings/ AppSettings (UserDefaults)
Sources/Heron/UI/       Theme, MenuBar/, Panel/ (floating NSPanel), Settings/
docs/ARCHITECTURE.md    data formats Heron reads (registry, transcripts, history)
docs/HOOKS.md           Claude Code hook payloads and registration
```

Modules talk only through `Core/Models.swift` and `AppState`. Read the doc
comment at the top of a file before changing its behaviour — it is the contract.

## Rules (loaded automatically from `.claude/rules/`)

- `privacy-and-security.md` — no network, read-only on `~/.claude`, never touch
  `*.key`, never persist prompt text, quote every shell argument.
- `swift.md` — main-actor UI, actors for I/O, Observation, no force unwraps,
  head-only transcript reads, ms timestamps.
- `ui.md` — system materials, SF Symbols, semantic colours, 8 pt grid, light and
  dark without branches, sentence-case copy.
- `testing-and-verification.md`, `git-and-workflow.md`.

## Skills (`.claude/skills/`)

Project: `heron-build`, `heron-claude-data`. Imported (MIT, see
`.claude/skills/THIRD-PARTY-LICENSES.md`): `swiftui-pro`, `concurrency-expert`,
`ui-patterns`, `performance-audit`, `view-refactor`, `macos-spm-app-packaging`,
`coding-best-practices`, `architecture-patterns`, `appkit-swiftui-bridge`,
`macos-capabilities`, `macos-tahoe-apis`, `sf-symbols`, `typography`,
`ux-writing`, `secure-code-guardian`. Load the relevant ones before non-trivial
work in that area.

## Agents (`.claude/agents/`)

`swift-reviewer`, `security-auditor`, `ui-polisher`, `macos-tester`.
Workflow: `review-changes` (`.claude/workflows/`) runs a four-dimension review
with adversarial verification.

## Hooks (`.claude/settings.json`)

- SessionStart → prints repo state. PreToolUse(Bash) → blocks commands that
  would damage `~/.claude` data or read `.key` files. PostToolUse(Edit/Write) →
  `swift format`. Stop → refuses to end the turn while `swift build` fails.

## Verifying for real

`make app && open dist/Heron.app`, run `claude` in a Terminal folder → it must
show up in the menu bar within 2 s; quit it → gone within 2 s. Notifications
only work from the `.app` bundle, never from `swift run`.
