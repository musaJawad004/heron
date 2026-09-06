---
name: heron-claude-data
description: Reference for the on-disk data Claude Code CLI writes that Heron reads — the live session registry (~/.claude/sessions/<pid>.json), transcript JSONL, history.jsonl, settings.json hooks — with field names, units and pitfalls. Use when parsing, watching or debugging session discovery, status, titles or hook events.
---
# Claude Code data formats Heron depends on

Full detail with examples: `docs/ARCHITECTURE.md` and `docs/HOOKS.md`. Quick facts:

**Live registry** `~/.claude/sessions/<pid>.json` — fields `pid`, `sessionId`,
`cwd`, `name`, `status` (`busy` | `idle` | `needs_input` | `waiting`),
`startedAt`/`updatedAt`/`statusUpdatedAt` in **milliseconds**, `version`,
`kind`, `entrypoint`. Files may outlive dead processes → check `kill(pid, 0)`.
Sibling `<pid>.<hash>.key` is private: never open it.

**Transcripts** `~/.claude/projects/<cwd with / → ->/<sessionId>.jsonl`.
Directory encoding is lossy; take `cwd` from the JSON lines. Title = first
`"type":"user"` line whose `message.content` (string, or array with a
`{"type":"text"}` block) does not start with `<`. Read only the head (64 KB).
Ignore `<sessionId>/subagents/` directories. Mtime = last activity.

**History** `~/.claude/history.jsonl` — `{display, timestamp(ms), project, sessionId}`
per prompt; cheapest way to enumerate recent sessions.

**Hooks** live under `hooks` in `~/.claude/settings.json`; payload fields are in
`docs/HOOKS.md`. Heron keeps `session_id, cwd, transcript_path, hook_event_name,
notification_type, reason, source, tool_name, title, message` and discards
`tool_input` and `user_input`.

Debug helpers
```bash
ls -la ~/.claude/sessions/ ; cat ~/.claude/sessions/*.json | python3 -m json.tool
ps -o pid=,tty=,command= -p <pid>
tail -3 ~/.claude/history.jsonl
ls "$HOME/Library/Application Support/Heron/events"
```
