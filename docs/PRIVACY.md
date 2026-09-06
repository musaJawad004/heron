# Privacy

Heron's promise is simple: **nothing leaves this machine.** This document lists
everything Heron reads and writes so you can verify the promise yourself.

## What Heron reads

| Path | What | Why |
|---|---|---|
| `~/.claude/sessions/*.json` | live session registry written by the Claude Code CLI (pid, session id, cwd, name, status, timestamps) | the running-session list and status |
| `~/.claude/projects/**/*.jsonl` | transcripts — **first 64 KB only** | cwd, git branch, version and the first prompt (used as the title) |
| `~/.claude/history.jsonl` | prompt history index | cheap discovery of recent sessions |
| `~/.claude/settings.json` | user settings | to know whether Heron's hooks are installed |
| process table | liveness, command line and tty of a session's pid | drop stale entries; raise the right terminal |

Heron **never** reads `~/.claude/sessions/*.key` (the CLI's private tokens),
never connects to the CLI's messaging socket, and never reads a transcript
past its head. (`~/.claude` is `%USERPROFILE%\.claude` on Windows.)

## What Heron writes

App-data dir: `~/Library/Application Support/com.glixentech.heron` (macOS),
`%APPDATA%\com.glixentech.heron` (Windows), `~/.local/share/com.glixentech.heron` (Linux).

| Path | What |
|---|---|
| `<app-data>/settings.json` | your preferences |
| `<app-data>/bin/heron-hook` (`.ps1` on Windows) | the hook receiver script (0700) |
| `<app-data>/events/*.json` | hook events spooled by that script (0600); deleted as soon as Heron reads them |
| `<app-data>/cache/` | title/cwd cache keyed by transcript path and mtime (0600) |
| `<app-data>/launch/` | one-shot launch scripts that delete themselves on run |
| `<app-data>/logs/` | Heron's own log; never contains prompt text |
| `~/.claude/settings.json` | **only if you click Install hooks**: six `hooks` entries pointing at the script above. A timestamped backup is written next to it first. Uninstall removes exactly those entries. |

## What Heron stores from your conversations

Only a session **title**: the first prompt, whitespace-collapsed and truncated
to 80 characters. Hook payload fields `tool_input` and `user_input` are
discarded at parse time and never written anywhere. Notification text is
limited to a project name and a generic message such as "Claude is waiting for
permission" plus the tool name.

## Permissions Heron may ask for

- **Notifications** — to show "needs you" / "finished" banners.
- **macOS Automation (Apple Events) for Terminal / iTerm2** — only when you
  click Focus on a running session, to select its tab. Decline and Heron will
  just bring the terminal app forward instead.

Heron does not request Accessibility, Full Disk Access, Screen Recording,
Location, Contacts or anything else. The webview's Content Security Policy
allows only the app's own files.

## Verifying

```bash
grep -rnE "reqwest|hyper|std::net|TcpStream|WebSocket" src-tauri/src   # → no results
grep -rnE "fetch\(|XMLHttpRequest|WebSocket" src                        # → no results
cat "$HOME/Library/Application Support/com.glixentech.heron/bin/heron-hook"   # read the hook
```
