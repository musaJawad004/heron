#!/bin/bash
# SessionStart: give Claude a one-screen picture of where the repo stands.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
echo "Heron repo state:"
echo "  branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null)  last: $(git log -1 --format='%h %s' 2>/dev/null)"
dirty=$(git status --short 2>/dev/null | wc -l | tr -d ' ')
echo "  uncommitted files: $dirty"
[ -d dist/Heron.app ] && echo "  dist/Heron.app present (make app to rebuild)" || echo "  no dist/Heron.app yet (make app)"
pgrep -xq Heron && echo "  Heron.app is currently running" || true
echo "  live Claude sessions: $(ls ~/.claude/sessions/*.json 2>/dev/null | wc -l | tr -d ' ')"
exit 0
