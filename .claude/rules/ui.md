---
paths:
  - "Sources/Heron/UI/**"
  - "Sources/Heron/App/**"
---
# UI design language

Heron should feel like a piece of macOS, not a web app. Minimal, quiet, sharp.

- **System first.** Materials (`.regularMaterial`, `.thinMaterial`), SF
  Symbols, `Color.primary/secondary`, system fonts. The only custom colours are
  the four status colours defined in `Theme` (green working, orange needs-you,
  grey idle, blue-ish "done" flash). Both appearances must look right without
  any `colorScheme` branches — pick semantic colours.
- **Menu bar window**: 320 pt wide, sections "Running" and "Recent", each row =
  status dot · project name · subtitle (title or branch) · relative time · a
  trailing action on hover (Focus / Resume). Footer: "New Session…",
  "Show/Hide Panel", "Settings…", "Quit". Keyboard: ⌘N new session, ⌘, settings,
  ⌘Q quit.
- **Menu bar label**: a template glyph (SF `bird` or a custom heron mark) plus the
  running count. Count is hidden when 0. Glyph gets an orange dot overlay when
  any session needs attention.
- **Floating panel**: compact capsule at the screen edge (collapsed: glyph + count
  + attention dot; expanded: one line per running session with dot + project +
  status word). Draggable anywhere, snaps back to the edge. Never steals focus
  (`.nonactivatingPanel`). Click a row → focus that terminal. Right-click → menu.
- **Settings**: `Form` with `.formStyle(.grouped)` in a `TabView` (General,
  Panel, Notifications, Hooks, About). Every toggle has a one-line explanation
  in `.caption` secondary text where the effect is not obvious.
- **Spacing** on an 8 pt grid; corner radius 10 (windows/cards) / 8 (rows).
  Type: `.headline` for names, `.subheadline` secondary for meta, `.caption2`
  for times. No text smaller than `.caption2`.
- **Motion**: only `withAnimation(.snappy)` for list changes and the attention
  dot pulse. Nothing decorative.
- **UX writing**: sentence case, verbs on buttons ("Resume", "Focus"), no
  exclamation marks, "Needs you" instead of "needs_input".
- Empty states say what to do next ("No Claude sessions running. ⌘N to start
  one in a folder.").
