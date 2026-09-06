# Heron architecture

Heron is a Tauri v2 app (Rust core + Svelte 5 frontend) that watches Claude
Code CLI sessions on the local machine. **Nothing leaves the machine: no
network code exists in this repository.** State is read from the CLI's own
files under `~/.claude` and Heron's app-data directory.

```
src-tauri/src
├── lib.rs / main.rs   Tauri builder, plugins, window events
├── model.rs           shared types (mirrored in src/lib/types.ts)
├── settings.rs        Settings + JSON store
├── state.rs           AppState hub: polls registry, watches hooks, publishes Snapshot
├── commands.rs        IPC surface
├── tray.rs            tray icon / count / menu
├── windows.rs         popover, panel, settings placement & behaviour
├── claude/            paths, registry (live), transcripts (history + titles)
├── hooks/             installer, watcher, notify
└── launch/            terminals, launch scripts, focus, find_claude
src
├── app.css            tokens
├── lib/api.ts         invoke/listen wrappers (only Tauri import)
├── lib/stores.svelte.ts  runes store
├── lib/types.ts       TS mirror
└── routes/{popover,panel,settings}
```

Data flow: background threads → `AppState` (mutex) → `publish()` → tray
update + `snapshot` event → every window's store. Actions go the other way
through `commands.rs`.

## Where Claude Code keeps its files

| | macOS / Linux | Windows |
|---|---|---|
| root | `~/.claude` | `%USERPROFILE%\.claude` |
| override | `CLAUDE_CONFIG_DIR` | same |
| live registry | `sessions/<pid>.json` | same |
| transcripts | `projects/<encoded cwd>/<sessionId>.jsonl` | same (`C:\Users\me\proj` → `C--Users-me-proj`) |
| history | `history.jsonl` | same |
| settings + hooks | `settings.json` | same |
| executable | `~/.local/bin/claude` (also Homebrew, npm global) | `%USERPROFILE%\.local\bin\claude.exe` (also npm global) |

Heron's own files live in the Tauri app-data dir: macOS
`~/Library/Application Support/com.glixentech.heron`, Windows
`%APPDATA%\com.glixentech.heron`, Linux `~/.local/share/com.glixentech.heron`:
`settings.json`, `events/` (hook spool), `bin/heron-hook[.ps1]`, `cache/`,
`launch/`.

## Data sources (observed on Claude Code 2.1.263)

### Live registry — `sessions/<pid>.json`
One file per running CLI process, rewritten on every status change.

```json
{"pid":3412,"sessionId":"4e035c9a-1ab6-4026-aed1-0fa8731e7792","cwd":"/Users/adz/civl-mobile-app",
 "startedAt":1788684708093,"procStart":"Sun Sep  6 08:51:47 2026","version":"2.1.263",
 "peerProtocol":1,"peerFeatures":["notify_idle","reply_across_default_dirs","artifact_yield"],
 "kind":"interactive","entrypoint":"cli","pidDomain":"darwin",
 "messagingSocketPath":"/tmp/cc-socks/3412.sock","name":"civl-mobile-app-1b","nameSource":"derived",
 "nameSince":1788684708094,"updatedAt":1788684844819,"status":"idle","statusUpdatedAt":1788684844819,
 "bridgeSessionId":"session_013zRkg3XpHjccY8Bo4FPPmo"}
```

* `status`: `busy`, `idle`, `needs_input`, `waiting` (strings found in the binary).
* Timestamps are Unix **milliseconds**.
* A sibling `<pid>.<hash>.key` holds a private token — **never read it**.
* Files can outlive a crashed process: verify the pid is alive **and** its
  command line contains `claude` (pid reuse).
* The controlling tty is not in the file; on Unix get it from the process
  table (`ps -o tty= -p <pid>`). Windows has no tty; focus by window instead.

### Transcripts — `projects/<encoded-cwd>/<sessionId>.jsonl`
The directory name replaces non-alphanumerics with `-` (lossy). Read `cwd`
from the lines. Relevant line shapes:

```json
{"type":"mode","mode":"normal","sessionId":"…"}
{"type":"permission-mode","permissionMode":"auto","sessionId":"…"}
{"parentUuid":null,"isSidechain":false,"promptId":"…","type":"user",
 "message":{"role":"user","content":"Please …"},"timestamp":"2026-09-04T13:37:41.586Z",
 "uuid":"…","cwd":"/Users/adz","sessionId":"…","version":"2.1.260","gitBranch":"main"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"…"}]},"cwd":"…"}
{"type":"system","subtype":"local_command","content":"<command-name>/resume</command-name>…"}
{"type":"cost-state","sessionId":"…","totalCostUSD":0}
```

* User `message.content` is a string or an array of blocks
  (`{"type":"text","text":…}`, `{"type":"tool_result",…}`). The **title** is
  the first user line whose text does not start with `<` (skips
  `<command-name>`, `<local-command-stdout>`, `<system-reminder>`) and is not a
  `tool_result`. Strip a leading `[Pasted text #N +M lines]` marker.
* Sub-agent transcripts live in `<sessionId>/subagents/` — ignore.
* Files can be many MB: read only the first 64 KB; mtime = recency.

### Prompt history — `history.jsonl`
`{"display":"…","pastedContents":{},"timestamp":1788685051331,"project":"/Users/adz","sessionId":"…"}`
per prompt. Cheap index of sessionId → project, last time.

### User settings — `settings.json`
Where hooks are registered. Merge carefully; other tools own keys here.

## Hook receiver
`hooks::installer` writes a receiver script into the app-data `bin/` dir and
registers it (exec form, `async: true`) for six events. The script only
spools stdin to `events/<ms>-<pid>.json`; `hooks::watcher` parses and deletes
each file. Payloads: `docs/HOOKS.md`.
