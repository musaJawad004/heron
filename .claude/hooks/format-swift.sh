#!/bin/bash
# PostToolUse (Edit|Write|MultiEdit): format the touched Swift file with the
# toolchain's swift-format so every edit lands in house style.
set -u
input=$(cat)
file=$(printf '%s' "$input" | python3 -c 'import json,sys; d=json.load(sys.stdin).get("tool_input",{}); print(d.get("file_path") or d.get("notebook_path") or "")' 2>/dev/null || true)
case "$file" in
  *.swift) [ -f "$file" ] && swift format --in-place --configuration "${CLAUDE_PROJECT_DIR:-.}/.swift-format" "$file" >/dev/null 2>&1 ;;
esac
exit 0
