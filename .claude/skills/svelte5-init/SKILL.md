---
name: svelte5-init
description: Scaffolds a new Svelte 5 or SvelteKit application with Tailwind CSS v4, TypeScript, and deployment adapter configuration. Use when the user wants to create, initialize, or start a new Svelte 5 project, SvelteKit app, or asks about project setup with Tailwind. Supports deployment targets like Cloudflare, Vercel, Netlify, Node, and static sites.
---

# Svelte 5 + SvelteKit Project Initialization

Scaffold a production-ready Svelte 5 / SvelteKit project with Tailwind CSS v4, TypeScript strict mode, and modern conventions.

## User parameters

Before starting, ask the user (if not already specified):

1. **Project name**: directory name for the project
2. **Deployment target**: where the app will run. Options:
   - `auto` (default) - auto-detects platform at build time
   - `cloudflare` - Cloudflare Workers / Pages
   - `vercel` - Vercel serverless / edge
   - `netlify` - Netlify functions / edge
   - `node` - Self-hosted Node.js server
   - `static` - Static site generation (SSG)
   - `spa` - Single-page application (client-only)

If the user does not specify a deployment target, use `auto`.

## Prerequisites

- Node.js 18.13+ (22+ recommended)
- pnpm (preferred) or npm

## Workflow

### Step 1: Scaffold the project

Use the official `sv` CLI to create the project:

```bash
npx sv create <project-name>
```

When prompted, select:
- **Template**: SvelteKit minimal (or skeleton if available)
- **Type checking**: TypeScript
- **Add-ons**: Select Tailwind CSS, prettier, eslint

If the user wants a library instead of an app, use the "library" template.

If `sv create` is not available or fails, fall back to:

```bash
npx create-svelte@latest <project-name>
```

### Step 2: Install and configure the deployment adapter

Based on the user's chosen deployment target, install the appropriate adapter.

#### auto (default)

No action needed. `@sveltejs/adapter-auto` is installed by default and auto-detects the platform (Cloudflare, Vercel, Netlify, Azure, AWS, Google Cloud).

#### cloudflare

```bash
pnpm add -D @sveltejs/adapter-cloudflare
```

`svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter()
  }
};

export default config;
```

Create `wrangler.jsonc`:

```jsonc
{
  "name": "<project-name>",
  "main": ".svelte-kit/cloudflare/_worker.js",
  "compatibility_flags": ["nodejs_als"],
  "compatibility_date": "2025-01-01",
  "assets": {
    "binding": "ASSETS",
    "directory": ".svelte-kit/cloudflare"
  }
}
```

Update `src/app.d.ts` to type platform bindings:

```ts
declare global {
  namespace App {
    interface Error {
      message: string;
      code?: string;
    }
    interface Locals {}
    interface PageData {}
    interface PageState {}
    interface Platform {
      env: {
        // Add your KV, D1, R2 bindings here, e.g.:
        // MY_KV: KVNamespace;
      };
      ctx: ExecutionContext;
      caches: CacheStorage;
      cf: IncomingRequestCfProperties;
    }
  }
}

export {};
```

Add to `.gitignore`:

```
.wrangler
```

**Important**: Cloudflare Workers cannot use Node.js `fs`. Use `read()` from `$app/server` to read static files, or prerender routes that need file access.

#### vercel

```bash
pnpm add -D @sveltejs/adapter-vercel
```

`svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-vercel';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter()
  }
};

export default config;
```

Per-route configuration is available via `config` export:

```ts
// src/routes/api/heavy/+server.ts
export const config = {
  runtime: 'nodejs22.x', // or 'edge'
  regions: ['iad1'],
  maxDuration: 15
};
```

For ISR (Incremental Static Regeneration):

```ts
// src/routes/blog/[slug]/+page.server.ts
export const config = {
  isr: {
    expiration: 60 // seconds
  }
};
```

#### netlify

```bash
pnpm add -D @sveltejs/adapter-netlify
```

`svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-netlify';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      edge: false,
      split: false
    })
  }
};

export default config;
```

Create `netlify.toml`:

```toml
[build]
  command = "npm run build"
  publish = "build"
```

Set `edge: true` for Netlify Edge Functions (Deno-based, lower latency). Note: `edge` and `split` cannot both be `true`.

#### node

```bash
pnpm add -D @sveltejs/adapter-node
```

`svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      out: 'build',
      precompress: true
    })
  }
};

export default config;
```

Run in production:

```bash
ORIGIN=https://my.site node build
```

For reverse proxy setups, use header-based origin detection:

```bash
PROTOCOL_HEADER=x-forwarded-proto HOST_HEADER=x-forwarded-host node build
```

Create a `Dockerfile` if the user wants containerized deployment:

```dockerfile
FROM node:22-slim AS build
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:22-slim
WORKDIR /app
COPY --from=build /app/build ./build
COPY --from=build /app/package.json .
ENV NODE_ENV=production
ENV PORT=3000
EXPOSE 3000
CMD ["node", "build"]
```

#### static

```bash
pnpm add -D @sveltejs/adapter-static
```

`svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      precompress: true,
      strict: true
    })
  }
};

export default config;
```

Enable prerendering in root layout:

```ts
// src/routes/+layout.ts
export const prerender = true;
```

**Important**: All pages must be prerenderable. No form actions, no `url.searchParams` in load functions, no dynamic user-specific content.

#### spa

Uses `@sveltejs/adapter-static` with a fallback page:

```bash
pnpm add -D @sveltejs/adapter-static
```

`svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: '200.html'
    })
  }
};

export default config;
```

Disable SSR in root layout:

```ts
// src/routes/+layout.ts
export const ssr = false;
```

**Warning**: SPA mode hurts SEO and performance (no server rendering, multiple round trips before content shows). Prefer hybrid rendering (prerender static pages + SSR dynamic pages) when possible.

### Step 3: Install Tailwind CSS v4

If Tailwind was not added during scaffolding, install it manually:

```bash
pnpm add -D tailwindcss @tailwindcss/vite
```

Configure the Vite plugin in `vite.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()]
});
```

Create or update `src/app.css`:

```css
@import 'tailwindcss';
```

Import in `src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
  let { children } = $props();
</script>

{@render children()}
```

### Step 4: Configure TypeScript strict mode

Update `tsconfig.json` to ensure strict mode:

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": false
  }
}
```

### Step 5: Set up the app type declarations

Create `src/app.d.ts` with proper type declarations (skip if already created in Step 2 for Cloudflare):

```ts
declare global {
  namespace App {
    interface Error {
      message: string;
      code?: string;
    }
    interface Locals {
      // Add per-request state here (e.g., user session)
    }
    interface PageData {
      // Add shared page data here
    }
    interface PageState {
      // Add shallow routing state here
    }
    interface Platform {}
  }
}

export {};
```

Create `src/app.html` if not present:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link rel="icon" href="%sveltekit.assets%/favicon.png" />
    %sveltekit.head%
  </head>
  <body data-sveltekit-preload-data="hover">
    <div style="display: contents">%sveltekit.body%</div>
  </body>
</html>
```

### Step 6: Create the root layout

`src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';

  let { children } = $props();
</script>

<svelte:head>
  <meta name="description" content="App description" />
</svelte:head>

{@render children()}
```

### Step 7: Create the root error page

`src/routes/+error.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>Error {page.status}</title>
</svelte:head>

<h1>{page.status}</h1>
<p>{page.error?.message}</p>
```

Create `src/error.html` as a static fallback:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>%sveltekit.status%</title>
  </head>
  <body>
    <h1>%sveltekit.status%</h1>
    <p>%sveltekit.error.message%</p>
  </body>
</html>
```

### Step 8: Create a sample home page

`src/routes/+page.svelte`:

```svelte
<script lang="ts">
  let count = $state(0);
</script>

<svelte:head>
  <title>Home</title>
</svelte:head>

<h1 class="text-3xl font-bold">Welcome</h1>
<button class="rounded bg-blue-500 px-4 py-2 text-white" onclick={() => count++}>
  Clicks: {count}
</button>
```

### Step 9: Set up testing

Install Vitest and Playwright:

```bash
pnpm add -D vitest @testing-library/svelte @testing-library/jest-dom jsdom playwright @playwright/test
npx playwright install
```

Create `vitest.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    include: ['src/**/*.{test,spec}.{js,ts}'],
    environment: 'jsdom',
    setupFiles: ['./vitest-setup.ts']
  }
});
```

Create `vitest-setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
```

Create `playwright.config.ts`:

```ts
import type { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
  webServer: {
    command: 'pnpm build && pnpm preview',
    port: 4173
  },
  testDir: 'tests',
  testMatch: /(.+\.)?(test|spec)\.[jt]s/
};

export default config;
```

Add scripts to `package.json`:

```json
{
  "scripts": {
    "test": "vitest",
    "test:unit": "vitest run",
    "test:e2e": "playwright test"
  }
}
```

### Step 10: Install dependencies and verify

```bash
pnpm install
pnpm dev
```

Verify: open the browser, confirm the page renders with Tailwind styles and the counter works.

## Project structure summary

```
src/
  app.css               # Tailwind import
  app.d.ts              # Type declarations
  app.html              # Root HTML template
  error.html            # Static error fallback
  lib/
    domains/            # Business domain modules
      auth/             # Auth components, state, services
      ...
    shared/             # Generic UI components (Button, Modal, etc.)
    server/             # Server-only utilities
    utils/              # Pure utility functions
  routes/
    +layout.svelte      # Root layout
    +error.svelte       # Root error boundary
    +page.svelte        # Home page
tests/                  # E2E tests (Playwright)
svelte.config.js        # SvelteKit config with adapter
vite.config.ts          # Vite config with Tailwind plugin
vitest.config.ts        # Vitest config
playwright.config.ts    # Playwright config
wrangler.jsonc          # Only for Cloudflare target
netlify.toml            # Only for Netlify target
Dockerfile              # Only for Node target (if containerized)
```

## Conventions to follow

**Svelte 5 syntax (mandatory)**
- Use `$props()` for component inputs, never `export let`
- Use `$state`, `$derived`, `$effect` runes (not `$:` reactive declarations)
- Use `{#snippet}` + `{@render}` instead of `<slot>`
- Use `onclick={handler}` instead of `on:click={handler}`
- Use `{@attach fn}` instead of `use:action`
- Use callback props instead of `createEventDispatcher`
- Use reactive classes in `.svelte.ts` for shared state (not stores)

**Architecture**
- Organize code by domain (`$lib/domains/`), not by technical layer
- Keep `src/routes/` thin: data passing and composition only
- Business logic in `.svelte.ts` files, not in components
- Use `$lib` imports instead of deep relative paths
- Keep server-only code in `$lib/server/`
- Use `$env/static/private` for secrets, `$env/static/public` for client config

**Quality**
- Use TypeScript with explicit types for props and exported functions
- Use Tailwind v4 utility classes for styling
- Semantic HTML elements (`<button>`, `<nav>`, `<main>`, not `<div onclick>`)
- Every page has a unique `<title>` (accessibility + SEO)
- Test server actions first (they are pure functions), then components, then E2E
