# Contributing to Heron

Thanks for helping. Heron is small on purpose; the bar for a change is "does
this make watching Claude Code sessions simpler or safer".

1. Fork, branch from `main`, keep commits focused.
2. `swift build && swift test` must pass; `make app` must produce a launchable
   bundle.
3. Follow `.claude/rules/` (they are short). In particular: **no network code,
   ever**, and nothing that reads `~/.claude/sessions/*.key`.
4. Open a PR describing the user-visible effect and how you verified it on a
   real Mac (`docs/` explains the data Heron reads).

If you use Claude Code, the repo ships a ready `.claude/` setup (rules, hooks,
skills, agents) — `claude` in the repo root picks it up automatically.
