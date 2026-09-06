#!/bin/bash
# Stop hook: refuse to end a turn while the package does not compile.
# Exit 2 feeds the compiler errors back to Claude so it fixes them first.
# `stop_hook_active` guards against infinite loops.
set -u
input=$(cat)
active=$(printf '%s' "$input" | python3 -c 'import json,sys; print("1" if json.load(sys.stdin).get("stop_hook_active") else "")' 2>/dev/null || true)
[ "$active" = "1" ] && exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
# Only bother when Swift sources changed since the last build check.
stamp=.build/.heron-stop-check
if [ -f "$stamp" ] && [ -z "$(find Sources Tests Package.swift -newer "$stamp" -name '*.swift' 2>/dev/null | head -1)" ]; then exit 0; fi
out=$(swift build 2>&1)
status=$?
mkdir -p .build && touch "$stamp"
if [ $status -ne 0 ]; then
  echo "swift build failed — fix these before finishing:" >&2
  printf '%s\n' "$out" | grep -E 'error:' | head -30 >&2
  exit 2
fi
exit 0
