#!/bin/bash
# Stop hook: refuse to end a turn while the Rust crate or the frontend does
# not type-check. Exit 2 feeds the errors back to Claude. `stop_hook_active`
# guards against loops; nothing runs unless sources changed since last check.
set -u
input=$(cat)
active=$(printf '%s' "$input" | python3 -c 'import json,sys; print("1" if json.load(sys.stdin).get("stop_hook_active") else "")' 2>/dev/null || true)
[ "$active" = "1" ] && exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
export PATH="$HOME/.cargo/bin:$PATH"
stamp=.svelte-kit/.heron-stop-check
mkdir -p .svelte-kit
fail=0; msg=""
if [ ! -f "$stamp" ] || [ -n "$(find src-tauri/src src-tauri/Cargo.toml src-tauri/tauri.conf.json -newer "$stamp" 2>/dev/null | head -1)" ]; then
  out=$(cd src-tauri && cargo check --quiet 2>&1)
  if [ $? -ne 0 ]; then fail=1; msg="$msg
cargo check failed:
$(printf '%s\n' "$out" | grep -E '^(error|warning: unused)' -A 6 | head -40)"; fi
fi
if [ ! -f "$stamp" ] || [ -n "$(find src svelte.config.js vite.config.ts tsconfig.json -newer "$stamp" 2>/dev/null | head -1)" ]; then
  out=$(npm run -s check 2>&1)
  if printf '%s' "$out" | grep -Eq '[1-9][0-9]* ERRORS|Error:'; then fail=1; msg="$msg
svelte-check failed:
$(printf '%s\n' "$out" | grep -E 'Error|ERROR' -A 3 | head -40)"; fi
fi
touch "$stamp"
if [ $fail -ne 0 ]; then
  echo "Fix these before finishing:$msg" >&2
  exit 2
fi
exit 0
