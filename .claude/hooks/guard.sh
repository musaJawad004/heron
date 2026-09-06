#!/bin/bash
# PreToolUse guard for Bash commands.
# Blocks commands that would damage the Claude Code data Heron depends on, or
# read private per-session key files. Exit 2 = block with the reason on stderr.
set -u
input=$(cat)
cmd=$(printf '%s' "$input" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("tool_input",{}).get("command",""))' 2>/dev/null || true)
[ -z "$cmd" ] && exit 0

deny() { echo "Heron guard: blocked — $1" >&2; exit 2; }

# Never touch the CLI's live registry or private key files.
printf '%s' "$cmd" | grep -Eq '\.claude/sessions/[^ ]*\.key' && deny "reading or writing session .key files"
printf '%s' "$cmd" | grep -Eq '(rm|mv|truncate|>)[^|]*\.claude/(sessions|projects|history\.jsonl)' && deny "modifying ~/.claude session data; Heron is read-only there"
printf '%s' "$cmd" | grep -Eq 'rm +-[a-zA-Z]*r[a-zA-Z]* +(~|\$HOME|/Users/[^/ ]+)/\.claude/?( |$)' && deny "deleting ~/.claude"
# Never publish anything from a hook-driven session.
printf '%s' "$cmd" | grep -Eq '(cargo +publish|npm +publish|gh +release +create)' && deny "publishing releases is a human action"
# Keep history linear on main.
printf '%s' "$cmd" | grep -Eq 'git +push[^|]*(--force|-f)[^|]*( main| origin main|$)' && deny "force-pushing main"
exit 0
