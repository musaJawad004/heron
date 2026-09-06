export const meta = {
  name: 'review-changes',
  description: 'Review the current branch of Heron along four dimensions, then adversarially verify each finding',
  phases: [{ title: 'Review' }, { title: 'Verify' }],
}

const FINDINGS_SCHEMA = {
  type: 'object',
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          title: { type: 'string' },
          file: { type: 'string' },
          line: { type: 'number' },
          severity: { type: 'string', enum: ['critical', 'high', 'medium', 'low'] },
          detail: { type: 'string' },
        },
        required: ['title', 'file', 'severity', 'detail'],
      },
    },
  },
  required: ['findings'],
}

const VERDICT_SCHEMA = {
  type: 'object',
  properties: { isReal: { type: 'boolean' }, reason: { type: 'string' } },
  required: ['isReal', 'reason'],
}

const DIMENSIONS = [
  { key: 'concurrency', prompt: 'Review `git diff main...HEAD` in this Heron checkout for Rust concurrency bugs: mutex guards held across Tauri emit/window calls, threads and notify watchers without a stop path, blocking work on the main thread. Follow .claude/rules/rust.md.' },
  { key: 'privacy', prompt: 'Review `git diff main...HEAD` in this Heron checkout against .claude/rules/privacy-and-security.md: network crates or fetch(), unquoted shell strings in generated scripts, prompt content persisted, unsafe settings.json merging, file modes, Tauri capability/CSP creep.' },
  { key: 'data-formats', prompt: 'Review `git diff main...HEAD` in this Heron checkout for mistakes against docs/ARCHITECTURE.md: ms vs s timestamps, stale pid files, reading whole transcripts, wrong field names from docs/HOOKS.md, Windows path handling, model.rs vs src/lib/types.ts drift.' },
  { key: 'ui', prompt: 'Review `git diff main...HEAD` in this Heron checkout against .claude/rules/ui.md: hard-coded colours, dark-mode breakage, spacing off-grid, missing empty states, non-runes Svelte, direct invoke() outside api.ts.' },
]

const results = await pipeline(
  DIMENSIONS,
  d => agent(d.prompt + ' Return findings only (no praise).', { label: `review:${d.key}`, phase: 'Review', schema: FINDINGS_SCHEMA }),
  review => parallel(review.findings.map(f => () =>
    agent(`Adversarially verify this finding in the Heron checkout. Read the file and decide if it is a real defect: ${JSON.stringify(f)}`,
      { label: `verify:${f.file}`, phase: 'Verify', schema: VERDICT_SCHEMA })
      .then(v => ({ ...f, verdict: v }))
  ))
)

const confirmed = results.flat().filter(Boolean).filter(f => f.verdict?.isReal)
return { confirmed }
