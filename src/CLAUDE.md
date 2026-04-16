# Svelte Frontend

## Stack

Svelte 5 + SvelteKit (static adapter, SPA mode) + TypeScript + Paraglide i18n

## Architecture

**Single-page app** — no file-based routing. All views are switched via `activeView` state in `+page.svelte` using `{#if}` blocks. Only 3 route files exist: `+layout.ts` (SSR disabled), `+layout.svelte` (app shell init), `+page.svelte` (the entire UI).

**Directory structure:**
```
src/lib/
├── api/tauri.ts          # ALL IPC wrappers (single source of truth)
├── components/           # UI components organized by domain
├── stores/               # Svelte writable/derived stores per domain
├── types/index.ts        # IPC types — MUST match Rust structs exactly
├── styles/               # Shared CSS fragments (list.css, detail.css)
├── search/               # Search abstraction (types, parser, providers)
├── utils/                # ansi.ts, debounce.ts, status.ts, time.ts
└── paraglide/            # Codegen output — DO NOT EDIT
```

## Component Conventions

### Props — use `$props()` with interface

```ts
interface Props {
  project: ProjectInfo;
  isActive: boolean;
  onClose: (index: number) => void;  // callback props, NOT Svelte events
}
let { project, isActive, onClose }: Props = $props();
```

### Runes — Svelte 5 only

| Rune | Usage |
|---|---|
| `$props()` | Component inputs with destructuring |
| `$state()` | Local reactive state |
| `$derived()` | Computed values |
| `$effect()` | Side effects watching reactive values |
| `$bindable()` | Two-way binding props |

**Never use** legacy Svelte 4 syntax (`export let`, `$:`, `on:click` directive). All event handling uses callback props or inline handlers.

### Snippets for slots

```ts
let { left, right }: { left: Snippet; right: Snippet } = $props();
```

## Store Conventions

All stores in `src/lib/stores/` follow this pattern:

1. **Export raw `writable<T>` atoms** as named exports
2. **Export `derived` stores** for computed values
3. **Export async action functions** that call API then update stores
4. **Last-wins async guard** — compare current store value before applying to handle rapid clicks
5. **No circular imports** between stores — inter-store calls through top-level imports

### Key patterns

**Polling:** `provider.ts` uses interval-based polling with auto-stop:
- CI run list: 15s interval
- CI run detail: 10s, auto-stops on terminal status
- Job log: 3s

**Event listeners:** `tasks.ts` listens to Tauri events (`task-started`, `task-output`, etc.) and batches updates via `requestAnimationFrame`. Auto-refreshes graph/branches after Fetch/Pull completes.

**Watcher refresh:** `repo.ts` registers a debounced `repo-changed` Tauri event listener (300ms) that auto-refreshes statuses, diffs, and conflict state.

**Tab switching:** `projects.ts` on switch: clears stale CI data, re-registers watcher, updates native title bar with starship-style status.

## Types — IPC Contract

`src/lib/types/index.ts` defines all interfaces matching Rust structs. **Rules:**

- **snake_case fields** to match Rust serde output exactly
- All IPC command return types must have a corresponding TypeScript interface
- `CiStatus` and `TaskStatus` are string literal unions matching Rust enum serialization
- `TaskStatus` is a discriminated union on `state` field

When modifying a Rust IPC struct, **always update the TypeScript type** in the same PR.

## IPC Wrappers

`src/lib/api/tauri.ts` is the **single source of truth** for all Tauri `invoke()` calls. Rules:

- Every function is `async`
- Uses `invoke<ReturnType>()` from `@tauri-apps/api/core`
- Params are camelCase (Tauri auto-converts to Rust snake_case)
- Organized by domain: Repository, Staging, Branches, Stash, Tags, Conflict, Remote, Provider, CI, Locale, Tasks, Projects, Theme
- **Never call `invoke()` directly from components or stores** — always go through `tauri.ts`

## Styling

**No Tailwind, no CSS modules.** Styling approach:

1. **CSS custom properties** on `:root` for all theme values — dynamically written by `theme.ts` at runtime
2. **Scoped `<style>` blocks** per component — each component owns its styles
3. **Shared CSS** in `src/lib/styles/` — imported by list/detail/dialog components (`list.css`, `detail.css`, `dialog.css`)
4. **Global CSS** in `src/app.css` — resets, fonts, scrollbars, utility classes

### Fonts
- **Code/text:** `Fira Code` (monospace with ligatures)
- **Icons:** `NerdFontSymbols` — Nerd Font glyphs as Unicode codepoints (e.g., `"\uE728"`) with `.nf` class or `font-family: var(--font-icons)`

### Theme variables
All colors come from CSS variables: `--bg-primary`, `--bg-secondary`, `--text-primary`, `--accent-blue`, `--border`, etc. **Never hardcode colors** — always use theme variables.

Font size baseline: `13px`. Layout: `display: flex` with `overflow: hidden`.

## i18n

**Tool:** `@inlang/paraglide-js` via Vite plugin. Two locales: `en-US` (base), `es-ES`.

**Usage:**
```ts
import * as m from "$lib/paraglide/messages";
m.sidebar_graph()                                    // no params
m.graph_commits_range({ start, end, total })         // with params
```

**Key naming:** `<domain>_<sub>` — e.g., `sidebar_branches`, `pipeline_status_running`

**All user-visible strings must use i18n** — never hardcode English text in components.

**Locale switching:** persists to backend via `setLocaleConfig()`, then `window.location.reload()`.

## Performance

- **Large lists:** Must use virtual scrolling or viewport slicing — never render 1000+ DOM nodes. The commit graph already uses canvas; apply the same principle to any unbounded list.
- **Debounce user input:** Always use `src/lib/utils/debounce.ts` for search, filter, and text input that triggers IPC calls or heavy computation.
- **Batch rapid updates:** Use `requestAnimationFrame` when handling rapid Tauri events (see `tasks.ts` pattern). Never update a store on every event tick.
- **Avoid unnecessary reactivity:** Don't create `$derived()` chains that recompute on every tick. Keep reactive graphs shallow.
- **Prefer events over polling:** Use Tauri event bridge (`listen()`) where possible. When polling is required, always auto-stop on terminal state.

## Reuse Checklist

Before creating any new code, check these locations:

| What | Check first |
|---|---|
| UI component | `src/lib/components/` — buttons, modals, dropdowns, icons, list items |
| Utility function | `src/lib/utils/` — `debounce.ts`, `ansi.ts`, `status.ts`, `time.ts` |
| CSS styles | `src/lib/styles/` — `list.css`, `detail.css` + shared classes in `app.css` |
| TypeScript type | `src/lib/types/index.ts` — check if the type already exists or can be extended |
| IPC wrapper | `src/lib/api/tauri.ts` — never duplicate an invoke call |
| i18n key | `messages/en-US.json` — reuse existing keys where the meaning matches |

## Adding a New View/Feature

1. **Check existing components** in `src/lib/components/` — reuse or extend before creating new ones
2. Add component(s) under `src/lib/components/<domain>/`
3. Add store in `src/lib/stores/<domain>.ts` if state management needed
4. Add IPC wrapper(s) in `src/lib/api/tauri.ts`
5. Add TypeScript types in `src/lib/types/index.ts`
6. Add i18n keys in both `en-US.json` and `es-ES.json`
7. Wire into `+page.svelte` via `activeView` if it's a new sidebar section
8. Use existing shared styles from `src/lib/styles/` where applicable
