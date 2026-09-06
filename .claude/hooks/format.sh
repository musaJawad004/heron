#!/bin/bash
# PostToolUse (Edit|Write|MultiEdit): format the touched file in house style.
# Rust → rustfmt (toolchain), Svelte/TS/CSS/JSON → prettier (repo devDependency).
set -u
input=$(cat)
file=$(printf '%s' "$input" | python3 -c 'import json,sys; d=json.load(sys.stdin).get("tool_input",{}); print(d.get("file_path") or "")' 2>/dev/null || true)
[ -z "$file" ] || [ ! -f "$file" ] && exit 0
export PATH="$HOME/.cargo/bin:$PATH"
root="${CLAUDE_PROJECT_DIR:-.}"
case "$file" in
  *.rs) rustfmt --edition 2021 --config-path "$root/src-tauri/rustfmt.toml" "$file" >/dev/null 2>&1 ;;
  *.svelte|*.ts|*.js|*.css|*.json|*.md)
    case "$file" in */node_modules/*|*/build/*|*/.svelte-kit/*|*/target/*) exit 0 ;; esac
    (cd "$root" && yarn prettier --log-level silent -w "$file" >/dev/null 2>&1) ;;
esac
exit 0
