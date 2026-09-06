---
name: heron-claude-data
description: Reference for the on-disk data the Claude Code CLI writes that Heron reads — live session registry (~/.claude/sessions/<pid>.json), transcript JSONL, history.jsonl, settings.json hooks — with field names, units, per-OS paths and pitfalls. Use when parsing, watching or debugging session discovery, status, titles or hook events.
---
# Claude Code data formats Heron depends on

Full detail with examples: `docs/ARCHITECTURE.md` and `docs/HOOKS.md`. Quick facts:

**Root** `~/.claude` (macOS/Linux) · `%USERPROFILE%\.claude` (Windows) ·
`CLAUDE_CONFIG_DIR` overrides. `claude::paths::ClaudePaths::detect()` handles it.

**Live registry** `sessions/<pid>.json` — `pid`, `sessionId`, `cwd`, `name`,
`status` (`busy` | `idle` | `needs_input` | `waiting`), `startedAt` /
`updatedAt` / `statusUpdatedAt` in **milliseconds**, `version`, `kind`,
`entrypoint`. Files may outlive dead processes → verify the pid (`sysinfo`).
Sibling `<pid>.<hash>.key` is private: never open it.

**Transcripts** `projects/<encoded cwd>/<sessionId>.jsonl` — encoding replaces
non-alphanumerics with `-` (lossy; Windows drive `C:\` → `C--`): take `cwd`
from the JSON lines. Title = first `"type":"user"` line whose `message.content`
(string, or array whose first `{"type":"text"}` block has text) does not start
with `<`. Read only the head (64 KB). Skip `<sessionId>/subagents/`.

**History** `history.jsonl` — `{display, timestamp(ms), project, sessionId}`.

**Hooks** live under `hooks` in `settings.json`; payload fields are in
`docs/HOOKS.md`. Heron keeps `session_id, cwd, transcript_path, hook_event_name,
notification_type, reason, source, result, tool_name, message` and discards
`tool_input`, `user_input`. Exec-form registration (`command` + `args`, no
shell) works on every OS; on Windows the command is `powershell.exe` running
the installed `.ps1`.

Debug helpers
```bash
ls -la ~/.claude/sessions/ ; python3 -m json.tool ~/.claude/sessions/*.json
ps -o pid=,tty=,command= -p <pid>
tail -3 ~/.claude/history.jsonl
ls "$HOME/Library/Application Support/com.glixentech.heron/events"   # macOS
```
