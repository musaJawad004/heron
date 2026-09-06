---
paths:
  - "src/**"
---
# Frontend conventions (SvelteKit + Svelte 5 + TypeScript)

- Svelte 5 **runes only**: `$state`, `$derived`, `$effect`, `$props`. No
  `export let`, no stores from `svelte/store`, no `on:click` (use `onclick`).
- `src/lib/api.ts` is the only file that imports `@tauri-apps/api`. Components
  call `api.*` and read `heron` from `src/lib/stores.svelte.ts`; they never
  `invoke` directly.
- State comes from Rust: after an action, wait for the `snapshot` event
  rather than mutating local copies (except optimistic toggles in Settings).
- Plain CSS with the tokens in `src/app.css`; no Tailwind, no UI kit, no icon
  font. Icons are inline SVG components in `src/lib/components/icons/`.
- Strict TypeScript (`svelte-check` must pass). Types mirror
  `src-tauri/src/model.rs` and `settings.rs` in `src/lib/types.ts`.
- Each window is a route: `/popover`, `/panel`, `/settings`. Routes stay thin
  and compose components from `src/lib/components/`. Components over ~120
  lines get split.
- Text must not be selectable and the cursor stays default except on links
  (`app.css` sets this); inputs opt back in with `user-select: text`.
- Format with prettier (`.prettierrc`); the PostToolUse hook does it on edit.
