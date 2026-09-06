---
name: svelte5-review
description: Reviews Svelte 5 code for correct rune usage, reactivity patterns, component architecture, cohesion, decoupling, accessibility, performance, testing, and TypeScript best practices. Use when the user asks to review, audit, lint, or check Svelte 5 components, runes, state management, or code quality.
---

# Svelte 5 Code Review

Expert review of Svelte 5 code focusing on correct rune usage, reactivity patterns, component cohesion, decoupling, type safety, accessibility, and performance.

## Review workflow

1. Read all files the user provides (or scan `src/` for `.svelte`, `.svelte.ts`, `.svelte.js`, `.ts` files)
2. Evaluate each file against the checklist categories below
3. Report findings grouped by severity: **Error** (broken/incorrect), **Warning** (suboptimal), **Suggestion** (improvement)
4. For each finding, show the problematic code and the corrected version

## Checklist

### 1. Rune correctness

**$state**
- Only used for values that must trigger UI updates
- Objects/arrays that are only reassigned (never mutated) use `$state.raw` instead
- No destructuring of `$state` objects expecting reactivity (breaks the proxy link)
- Class fields use `$state` correctly with arrow functions or proper method binding
- `$state.snapshot()` used when passing proxied state to external libraries (structuredClone, postMessage, third-party libs)
- `$state.eager()` used only for immediate UI feedback (navigation indicators, active states)

**$derived**
- Used instead of `$effect` for any computed value
- Expression is pure (no side effects inside `$derived`)
- `$derived.by(() => {...})` used for multi-statement logic
- Props-dependent values always wrapped in `$derived`, never computed once at init:

```svelte
<!-- WRONG -->
<script lang="ts">
  let { type } = $props();
  let color = type === 'error' ? 'red' : 'blue'; // computed once, never updates
</script>

<!-- CORRECT -->
<script lang="ts">
  let { type } = $props();
  let color = $derived(type === 'error' ? 'red' : 'blue');
</script>
```

**$effect**
- Treated as an escape hatch, not normal control flow
- Never used to synchronize state (use `$derived` instead):

```svelte
<!-- WRONG: effect setting state -->
<script lang="ts">
  let count = $state(0);
  let doubled = $state(0);
  $effect(() => { doubled = count * 2; });
</script>

<!-- CORRECT: derived value -->
<script lang="ts">
  let count = $state(0);
  let doubled = $derived(count * 2);
</script>
```

- Returns cleanup functions when creating subscriptions, intervals, or event listeners
- `$effect.pre` used only when DOM measurement before update is needed
- No `if (browser)` wrapping (effects already skip SSR)

**$props**
- Always destructured with type annotations
- Fallback values provided for optional props
- `$props.id()` used for accessibility IDs instead of manual ID generation
- Rest props (`...rest`) used to forward unknown attributes
- Props not mutated unless marked with `$bindable`

**$bindable**
- Used sparingly and only when two-way data flow is truly needed
- Parent uses `bind:` directive when consuming bindable props
- Not used as a shortcut to avoid proper event callbacks

**$inspect**
- Only present in development code, never in production paths
- Used instead of `console.log` for reactive debugging
- `$inspect.trace()` used as first statement in functions to trace re-execution

**$host**
- Only used inside components compiled as custom elements (`<svelte:options customElement="...">`)

### 2. Template syntax

**Snippets and rendering**
- `{#snippet}` + `{@render}` used instead of `<slot>` (legacy)
- Snippet props typed with `Snippet<[ParamTypes]>` from `'svelte'`
- Optional snippets rendered with `{@render snippet?.()}`
- Reusable markup extracted into snippets to eliminate duplication

**Attachments**
- `{@attach fn}` used instead of `use:action` for DOM integration
- Attachment factories used for reactive parameters
- Cleanup functions returned from attachments

**Event handlers**
- `onclick={handler}` syntax used instead of `on:click={handler}` (legacy)
- Inline handlers for simple operations: `onclick={() => count++}`
- Named functions for complex handlers
- Callback props used instead of `createEventDispatcher` (legacy)

**Each blocks**
- Keyed with unique identifiers: `{#each items as item (item.id)}`
- Never keyed by index when items can reorder
- Destructuring used in each blocks when accessing multiple properties

**Dynamic components**
- Direct `<Component>` usage instead of `<svelte:component this={Component}>` (legacy)

### 3. State management and architecture

**Reactivity hierarchy** (prefer in this order)
1. Template expressions (most automatic)
2. `$derived` (computed values)
3. `$effect` (escape hatch for side effects only)

**Unidirectional data flow**
- Data flows parent-to-child via props
- Child-to-parent communication via callback props (not mutation)
- No bypassing the hierarchy with global mutable state

**Component-level state**
- State declared at the top of `<script>` block
- Related state grouped into cohesive objects
- Derived values placed after the state they depend on
- Static values do not use `$state` (no unnecessary reactivity)

**Shared state (reactive classes preferred)**
- Shared reactive state lives in `.svelte.ts` or `.svelte.js` files (not plain `.ts`)
- Reactive classes preferred over plain functions for complex state:

```ts
// counter.svelte.ts — preferred pattern
export class Counter {
  count = $state(0);
  doubled = $derived(this.count * 2);

  increment() { this.count++; }
  decrement() { this.count--; }
}
```

Simple module-level state for trivial cases:

```ts
// toggle.svelte.ts
let active = $state(false);
export function toggle() { active = !active; }
export function isActive() { return active; }
```

- No duplicated state across components (centralize in shared modules)
- No global module-level state in SSR apps (use context API instead)

**Context API**
- `createContext<T>()` used for type-safe context (Svelte 5.40+)
- Context used to avoid prop drilling through intermediate components
- Reactive state passed via context uses `$state` objects (mutate, don't reassign)
- Context values not reassigned after creation (breaks the reference link)

**Stores**
- Avoided in new code (use runes instead)
- If present, flagged for migration to `$state` / `$derived` / context

### 4. Cohesion and decoupling

**Single responsibility**
- Each component has one clear purpose
- Components under 200 lines (extract sub-components if larger)
- Logic separated from presentation where complex
- Presentational components: receive data via props, render UI
- Container components: fetch data, manage state, pass down to presentational

**Domain-driven organization**
- Code organized by business domain, not technical layer:

```
src/lib/
  domains/
    auth/           # Auth components, state, utils
    products/       # Product components, state, utils
    checkout/       # Checkout flow
  shared/           # Generic UI components (Button, Modal, etc.)
  server/           # Server-only utilities
```

- Each domain is self-contained with its own components, state, and utilities
- Minimal cross-domain dependencies
- `src/routes/` kept thin (data passing + composition only)

**Separation of concerns**
- Business logic in `.svelte.ts` files, not inline in components
- API calls abstracted into service modules in `$lib/`
- Types defined in dedicated `.ts` files or co-located with their module
- Server-only code strictly in `$lib/server/` or `+page.server.ts`

**Reusability**
- Shared components in `$lib/shared/` or `$lib/components/`
- Shared state in domain-specific `.svelte.ts` files
- Shared utilities in `$lib/utils/`
- No circular dependencies between modules

**Props interface**
- Components accept the minimum props needed (interface segregation)
- Complex data transformed before passing to children
- Callback props preferred over two-way binding for actions

**Naming conventions**
- Consistent naming across the codebase (pick one style, stick to it)
- Components: PascalCase (`UserCard.svelte`)
- State files: camelCase or kebab-case with `.svelte.ts` extension
- Functions/variables: camelCase
- Types/interfaces: PascalCase
- Constants: UPPER_SNAKE_CASE or camelCase (be consistent)

### 5. TypeScript and type safety

- All component props typed (inline or interface)
- Generic components use proper generics with `generics` attribute
- Event handler types use Svelte's built-in types
- Return types on exported functions
- `satisfies` used for type narrowing where appropriate
- No `any` without explicit justification
- `$lib` alias used instead of deep relative paths

### 6. Accessibility

- Semantic HTML elements used (`<button>`, `<nav>`, `<main>`, `<article>`, not `<div onclick>`)
- Interactive elements are keyboard-navigable (`<button>` over `<div onclick>`)
- Images have meaningful `alt` text (or `alt=""` for decorative images)
- Form inputs have associated `<label>` elements (or `aria-label`)
- `$props.id()` used for linking labels to inputs (SSR-safe unique IDs)
- Touch targets at least 24x24 px (WCAG 2.2 AA)
- Visible focus indicators with sufficient contrast (>=3:1)
- No drag-only interactions without keyboard alternatives
- Color is not the sole means of conveying information
- `aria-live` regions for dynamic content updates
- Headings form a logical hierarchy (`<h1>` to `<h6>`)

### 7. Error handling

- User-facing errors are actionable (tell the user what to do, not just what went wrong)
- Different error types get appropriate treatment:
  - Network errors: offer retry
  - Validation errors: highlight specific fields
  - Auth errors: redirect to login
  - Unknown errors: show generic message with support contact
- `{#if error}` blocks provide fallback UI, not blank screens
- `<svelte:boundary>` used for component-level error boundaries where appropriate
- Async operations handle both success and failure paths
- Loading states shown during async operations (no blank/frozen UI)

### 8. Performance

- `$state.raw` for large, immutable data structures (API responses, config)
- `$state.snapshot()` for serialization boundaries
- Keyed `{#each}` blocks for lists that reorder
- Lazy imports for heavy components: `const Component = await import('./Heavy.svelte')`
- No unnecessary reactivity (static values should not use `$state`)
- Images use modern formats (WebP/AVIF), explicit width/height to prevent CLS
- Awareness of Core Web Vitals targets: LCP <= 2.5s, INP <= 200ms, CLS <= 0.1

### 9. Testing

- Components have unit tests (Vitest recommended)
- Reactive classes / shared state modules tested independently
- Svelte 5 components testable as functions (no heavy DOM simulators needed)
- Vitest browser mode for interactive component tests (over jsdom)
- Playwright for E2E and critical user flows
- Test files co-located with source or in `tests/` directory

## Report format

```
## Svelte 5 Code Review

### Errors (must fix)
- [FILE:LINE] Description
  Before: `code`
  After: `corrected code`

### Warnings (should fix)
- [FILE:LINE] Description
  Before: `code`
  After: `corrected code`

### Suggestions (nice to have)
- [FILE:LINE] Description
  Recommendation: explanation

### Summary
- X errors, Y warnings, Z suggestions
- Overall assessment: [score/description]
```
