## What this changes

<!-- One or two sentences. Link the issue if there is one. -->

## How you verified it

<!-- Which OS you ran it on, and what you actually saw. -->

## Checklist

- [ ] `cargo test`, `cargo clippy -- -D warnings` and `yarn run check` pass
- [ ] No network code (the scan enforces this)
- [ ] Nothing reads `~/.claude/sessions/*.key`
- [ ] No prompt or tool content is written to disk or logs
