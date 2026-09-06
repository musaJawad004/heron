#!/bin/bash
# SessionStart: one-screen picture of where the repo stands.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
echo "Heron repo state:"
echo "  branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null)  last: $(git log -1 --format='%h %s' 2>/dev/null)"
echo "  uncommitted files: $(git status --short 2>/dev/null | wc -l | tr -d ' ')"
[ -d node_modules ] && echo "  node_modules present" || echo "  run: npm install"
[ -d src-tauri/target ] && echo "  rust target cache present" || echo "  first cargo build will take a few minutes"
pgrep -xq heron && echo "  a dev 'heron' process is running (single-instance will reuse it)" || true
echo "  live Claude sessions on this machine: $(ls ~/.claude/sessions/*.json 2>/dev/null | wc -l | tr -d ' ')"
exit 0
