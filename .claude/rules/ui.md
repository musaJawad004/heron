---
paths:
  - "src/**/*.svelte"
  - "src/app.css"
---
# UI design language

Heron should feel like a piece of the OS, not a web page. Minimal, quiet, sharp.

- **Materials and semantic colour only.** Windows are transparent; the OS
  material (`windowEffects`) shows through `--surface`. Text uses `--text`,
  `--text-secondary`, `--text-tertiary`; the only chromatic colours are the
  status tokens (`--status-busy` green, `--status-attention` orange,
  `--status-idle` grey, `--status-done` blue) and `--accent` for primary
  actions. Light and dark come from `light-dark()`; never branch on a class.
- **Popover** (340 px wide, max ~480 px tall, lists scroll): header "Heron" +
  summary ("3 running · 1 needs you") + `+` button (⌘N). Sections "Running"
  and "Recent". Row = 8 px status dot · name (13 px, 600) · subtitle (12 px,
  secondary: title or path, single line, middle-ellipsis for paths) · trailing
  status word + relative time (11 px) · hover-revealed action ("Focus" /
  "Resume"). Footer bar: Panel toggle, Settings, Quit. Escape hides.
- **Panel**: a 12 px-radius capsule. Collapsed = glyph + count + attention dot.
  Expanded = one row per running session (dot · project · status word), max
  width 220 px, attention rows first. Whole surface is a drag region except
  buttons. Never grows past ~8 rows; beyond that "+N more".
- **Settings**: sidebar tabs (General, Panel, Notifications, Hooks, About) at
  600×520; grouped rows with a caption under any control whose effect is not
  obvious. Native-looking toggles (custom, 26×16, animated), selects styled
  minimally, monospace for paths.
- **Spacing** on a 4/8 px grid (`--space-*`), radii `--radius-*`, type sizes
  `--text-*` (11/12/13/15). Nothing smaller than 11 px.
- **Motion**: 150 ms ease-out for hover/appear, a 1.2 s pulse on the
  attention dot. Nothing decorative; respect `prefers-reduced-motion`.
- **Copy**: sentence case, verbs on buttons ("Resume", "Focus"), "Needs you"
  not "needs_input", no exclamation marks. Empty states say what to do next
  ("No Claude sessions running. ⌘N to start one in a folder.").
- **Keyboard**: ⌘N new session, ⌘, settings, ⌘Q quit, Esc close popover,
  arrow keys move through rows, Enter activates.
