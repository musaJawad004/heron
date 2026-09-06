# Vigil architecture

Vigil is a macOS menu bar app (SwiftUI + AppKit, Swift Package, macOS 14+) that
watches Claude Code CLI sessions on the local machine. **Nothing leaves the
Mac: no network code exists in this repository.** All state is read from
`~/.claude` (written by the Claude Code CLI itself) and from
`~/Library/Application Support/Vigil`.

```
Sources/Vigil
├── App/        VigilApp (@main, MenuBarExtra + Settings scenes), AppState (hub)
├── Core/       Models (shared contract), SessionRegistry (live), TranscriptIndex (history)
├── Hooks/      HookInstaller (settings.json), HookEventWatcher (spool dir), Notifier
├── Launch/     TerminalLauncher (new / resume / focus in a terminal app)
├── Settings/   AppSettings (UserDefaults)
└── UI/         Theme, MenuBar/, Panel/ (floating NSPanel), Settings/
```

`AppState` is the only object the UI talks to. Modules never import each other
except through `Models.swift`. See the doc comment at the top of each stub for
the contract it must fulfil.

## Data sources (observed on Claude Code 2.1.263, macOS 26)

### Live registry — `~/.claude/sessions/<pid>.json`
One file per running CLI process, rewritten on every status change. Delete
nothing here. Example:

```json
{"pid":3412,"sessionId":"4e035c9a-1ab6-4026-aed1-0fa8731e7792","cwd":"/Users/adz/civl-mobile-app",
 "startedAt":1788684708093,"procStart":"Sun Sep  6 08:51:47 2026","version":"2.1.263",
 "peerProtocol":1,"peerFeatures":["notify_idle","reply_across_default_dirs","artifact_yield"],
 "kind":"interactive","entrypoint":"cli","pidDomain":"darwin",
 "messagingSocketPath":"/tmp/cc-socks/3412.sock","name":"civl-mobile-app-1b","nameSource":"derived",
 "nameSince":1788684708094,"updatedAt":1788684844819,"status":"idle","statusUpdatedAt":1788684844819,
 "bridgeSessionId":"session_013zRkg3XpHjccY8Bo4FPPmo"}
```

* `status` values seen in the binary: `busy`, `idle`, `needs_input`, `waiting`.
* Timestamps are Unix **milliseconds**.
* A sibling `<pid>.<hash>.key` file holds a private token — **never read it**.
* Files can outlive a crashed process: always verify `pid` with `kill(pid, 0)`
  (and ideally that the process command line contains `claude`).
* The controlling tty is not in the file; get it via `ps -o tty= -p <pid>`.

### Transcripts — `~/.claude/projects/<encoded-cwd>/<sessionId>.jsonl`
`<encoded-cwd>` is the cwd with `/` (and other non-alphanumerics) replaced by
`-`, so it is ambiguous — read `cwd` from the lines instead. Lines are JSON
objects. Relevant shapes:

```json
{"type":"mode","mode":"normal","sessionId":"…"}
{"type":"permission-mode","permissionMode":"auto","sessionId":"…"}
{"parentUuid":null,"isSidechain":false,"promptId":"…","type":"user",
 "message":{"role":"user","content":"Please …"},"timestamp":"2026-09-04T13:37:41.586Z",
 "uuid":"…","cwd":"/Users/adz","sessionId":"…","version":"2.1.260","gitBranch":"main"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"…"}]},"cwd":"…",…}
{"type":"system","subtype":"local_command","content":"<command-name>/resume</command-name>…",…}
{"type":"cost-state","sessionId":"…","totalCostUSD":0,…}
```

* `message.content` for user lines is either a string or an array of blocks
  (`{"type":"text","text":…}`, `{"type":"tool_result",…}`). The **title** is
  the first user line whose text does not start with `<` (skip
  `<command-name>`, `<local-command-stdout>`, `<system-reminder>` wrappers) and
  is not a `tool_result`.
* Sub-agent transcripts live in `<sessionId>/subagents/` — ignore those dirs.
* Files can be many MB. Read only the head (first 64 KB is plenty for
  cwd/title); use mtime for recency.

### Prompt history — `~/.claude/history.jsonl`
One line per prompt: `{"display":"…","pastedContents":{},"timestamp":1788685051331,
"project":"/Users/adz","sessionId":"…"}`. Cheap index of (sessionId → project,
last prompt, last time). Use it to discover sessions quickly; fall back to the
transcript for the title.

### User settings — `~/.claude/settings.json`
Where hooks get registered. Merge carefully; other tools may own keys here.

## Hook receiver
`HookInstaller` writes `~/Library/Application Support/Vigil/bin/vigil-hook`
and registers it in settings.json. The script only spools stdin to
`~/Library/Application Support/Vigil/events/<ts>-<pid>.json`. See
`docs/HOOKS.md` for the payloads.
