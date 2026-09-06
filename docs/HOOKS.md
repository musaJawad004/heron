# Claude Code hook payloads (2.1.x)

Source: https://code.claude.com/docs/en/hooks.md

## Registration (in `~/.claude/settings.json`)

```json
{
  "hooks": {
    "Notification": [
      { "matcher": "permission_prompt|idle_prompt|elicitation_dialog",
        "hooks": [ { "type": "command", "command": "/Users/me/Library/Application Support/Vigil/bin/vigil-hook", "async": true, "timeout": 5 } ] }
    ],
    "Stop":            [ { "hooks": [ { "type": "command", "command": "…/vigil-hook", "async": true, "timeout": 5 } ] } ],
    "SessionStart":    [ { "hooks": [ { "type": "command", "command": "…/vigil-hook", "async": true, "timeout": 5 } ] } ],
    "SessionEnd":      [ { "hooks": [ { "type": "command", "command": "…/vigil-hook", "async": true, "timeout": 5 } ] } ],
    "PermissionRequest": [ { "hooks": [ { "type": "command", "command": "…/vigil-hook", "async": true, "timeout": 5 } ] } ],
    "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "…/vigil-hook", "async": true, "timeout": 5 } ] } ]
  }
}
```

* `matcher` omitted or `"*"` = all. Letters/digits/`_`/`-`/`|` = exact list;
  anything else = regex.
* Hook object fields: `type` ("command"), `command`, `args` (exec form, no shell),
  `timeout` (seconds, default 600), `async` (bool, fire-and-forget), `statusMessage`.
* Command hooks run via `/bin/sh -c` in the session cwd with the login env.
* Exit **0 with no stdout** = pure observer; never exit 2 (blocks the action).
* User-level and project-level hooks are merged additively per event.
* Commands containing spaces (our path has "Application Support") must be
  quoted inside the `command` string — or better, use `"command": "/bin/sh"`
  with `"args": ["<path>"]`? No: prefer the shell form with the path quoted:
  `"command": "\"$HOME/Library/Application Support/Vigil/bin/vigil-hook\""`.
  `$HOME` is expanded by `/bin/sh`, so the entry stays valid if the home moves.

## Payloads (stdin JSON)

Common to every event: `session_id`, `hook_event_name`, `cwd`, `transcript_path`,
`prompt_id` (2.1.196+), `permission_mode`.

| Event | Extra fields |
|---|---|
| `SessionStart` | `source`: `startup` \| `resume` \| `clear` \| `compact` \| `fork`; `agent_id`, `agent_type` when a subagent |
| `SessionEnd` | `reason`: `user_exit` \| `clear` \| `resume` \| `error` \| `interrupt` \| `sigterm` \| `timeout` \| `logout` |
| `Stop` | (none) — Claude finished its turn |
| `SubagentStop` | `agent_id`, `agent_type`, `result`, `tool_use_id` |
| `UserPromptSubmit` | `user_input` (**do not store or display**) |
| `PermissionRequest` | `tool_name`, `tool_input`, `tool_use_id` |
| `Notification` | `notification_type`, `title`, `message` |
| `PreToolUse` / `PostToolUse` | `tool_name`, `tool_input`, `tool_use_id` |

`notification_type` values: `permission_prompt`, `idle_prompt`, `auth_success`,
`elicitation_dialog`, `elicitation_url_dialog`, `elicitation_complete`,
`elicitation_response`, `agent_needs_input`, `agent_completed`,
`quota_auto_resume_fired`, `quota_auto_resume_stale`, `quota_auto_resume_disabled`.

Privacy rule for Vigil: keep `session_id`, `cwd`, `transcript_path`,
`hook_event_name`, `notification_type`/`reason`/`source`, `tool_name`, `title`,
`message`. Drop `tool_input` and `user_input` (they can contain secrets) — the
receiver script spools the raw JSON, but `HookEventWatcher.parse` must only
lift the allowed fields, and the spool file is deleted immediately after parsing.
