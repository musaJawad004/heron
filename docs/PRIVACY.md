# Privacy

Heron's promise is simple: **nothing leaves this Mac.** This document lists
everything Heron reads and writes so you can verify the promise yourself.

## What Heron reads

| Path | What | Why |
|---|---|---|
| `~/.claude/sessions/*.json` | live session registry written by the Claude Code CLI (pid, session id, cwd, name, status, timestamps) | the running-session list and status |
| `~/.claude/projects/**/*.jsonl` | transcripts — **first 64 KB only** | cwd, git branch, version and the first prompt (used as the title) |
| `~/.claude/history.jsonl` | prompt history index | cheap discovery of recent sessions |
| `~/.claude/settings.json` | user settings | to know whether Heron's hooks are installed |
| process table (`ps`) | tty of a session's pid | to raise the right terminal tab |

Heron **never** reads `~/.claude/sessions/*.key` (the CLI's private tokens),
never connects to `/tmp/cc-socks/*.sock`, and never reads a transcript past
its head.

## What Heron writes

| Path | What |
|---|---|
| `~/Library/Application Support/Heron/bin/heron-hook` | the hook receiver script (mode 0700) |
| `~/Library/Application Support/Heron/events/*.json` | hook events spooled by that script (mode 0600); deleted as soon as Heron reads them |
| `~/Library/Application Support/Heron/cache/` | title/cwd cache keyed by transcript path and mtime (mode 0600) |
| `~/Library/Application Support/Heron/launch/*.command` | one-shot launch scripts that delete themselves on run |
| `~/.claude/settings.json` | **only if you click Install hooks**: six `hooks` entries pointing at the script above. A timestamped backup is written next to it first. Uninstall removes exactly those entries. |
| `~/Library/Preferences/com.glixentech.heron.plist` | your preferences (terminal app, panel position, toggles) |

## What Heron stores from your conversations

Only a session **title**: the first prompt, whitespace-collapsed and truncated
to 80 characters. Hook payload fields `tool_input` and `user_input` are
discarded at parse time and never written anywhere. Notification text is
limited to a project name and a generic message such as "Claude is waiting for
permission" plus the tool name.

## Permissions Heron may ask for

- **Notifications** — to show "needs you" / "finished" banners.
- **Automation (Apple Events) for Terminal / iTerm2** — only when you click
  Focus on a running session, to select its tab. Decline and Heron will just
  bring the terminal app forward instead.

Heron does not request Accessibility, Full Disk Access, Screen Recording,
Location, Contacts or anything else, and it is not sandboxed only because
reading `~/.claude` requires it — there is no network entitlement to grant
because there is no network code.

## Verifying

```bash
grep -rn "URLSession\|Network\|socket(\|curl\|wget" Sources/   # → no results
cat "$HOME/Library/Application Support/Heron/bin/heron-hook"   # read the hook
```
