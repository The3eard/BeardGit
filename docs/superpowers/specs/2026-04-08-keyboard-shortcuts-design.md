# Keyboard Shortcuts

## Overview

Add hardcoded keyboard shortcuts with a discoverable cheat sheet overlay. Platform-aware modifiers (Cmd on macOS, Ctrl on Windows/Linux). No user customization for now — configurable bindings can be layered on later.

## Architecture

### Shortcut Registry (`src/lib/stores/shortcuts.ts`)

A central store that:

- Defines all shortcuts as a typed array of `{ id, keys, label, category, action }`
- Registers a single `keydown` listener on `window` (in `+layout.svelte` on mount)
- Dispatches matching actions, respecting focus context (skips when input/textarea focused for non-modified shortcuts)
- Exposes the shortcut list for the cheat sheet overlay

```typescript
interface Shortcut {
  id: string;
  keys: ShortcutKeys;
  label: string;           // i18n message key
  category: string;        // for cheat sheet grouping
  action: () => void;
}

interface ShortcutKeys {
  mod?: boolean;           // Cmd (macOS) or Ctrl (Win/Linux)
  shift?: boolean;
  alt?: boolean;
  key: string;             // KeyboardEvent.key value
}
```

### Platform Detection

Use `navigator.platform` or `navigator.userAgentData` to detect macOS vs Windows/Linux. Store as a reactive value for display purposes (showing "⌘" vs "Ctrl" in the cheat sheet).

### Focus Context

Shortcuts are split into two categories:

- **Global shortcuts** (with modifier keys: `Cmd+1`, `Cmd+Shift+F`) — Always fire, even when inputs are focused
- **Contextual shortcuts** (bare keys: `J`, `K`, `/`, `?`) — Only fire when no input/textarea/contenteditable is focused

The keydown handler checks `document.activeElement` before dispatching contextual shortcuts.

### Listener Registration

Single `keydown` listener registered in `+layout.svelte` on mount. The handler:

1. Normalizes the event (extract mod/shift/alt/key)
2. Finds matching shortcut from registry
3. Checks focus context for bare-key shortcuts
4. Calls `event.preventDefault()` and executes the action
5. Returns early if inside a dialog (Escape is handled by dialogs themselves)

## Shortcut Definitions

### Tier 1 — Navigation

| Shortcut | macOS | Windows/Linux | Action |
|---|---|---|---|
| Switch to Graph | `⌘1` | `Ctrl+1` | Navigate to Graph view |
| Switch to Changes | `⌘2` | `Ctrl+2` | Navigate to Changes view |
| Switch to Branches | `⌘3` | `Ctrl+3` | Navigate to Branches view |
| Switch to Tags | `⌘4` | `Ctrl+4` | Navigate to Tags view |
| Switch to Stashes | `⌘5` | `Ctrl+5` | Navigate to Stashes view |
| Switch to Worktrees | `⌘6` | `Ctrl+6` | Navigate to Worktrees view |
| Open Settings | `⌘,` | `Ctrl+,` | Navigate to Settings view |
| Next tab | `⌘Tab` | `Ctrl+Tab` | Switch to next project tab |
| Previous tab | `⌘⇧Tab` | `Ctrl+Shift+Tab` | Switch to previous project tab |
| Close tab | `⌘W` | `Ctrl+W` | Close current project tab |
| Dismiss | `Escape` | `Escape` | Close popup/dialog/panel (handled by components, not registry) |

### Tier 2 — Git Operations

| Shortcut | macOS | Windows/Linux | Action |
|---|---|---|---|
| Fetch | `⌘⇧F` | `Ctrl+Shift+F` | Fetch from origin |
| Pull | `⌘⇧L` | `Ctrl+Shift+L` | Pull from origin |
| Push | `⌘⇧P` | `Ctrl+Shift+P` | Push to origin |
| Commit | `⌘Enter` | `Ctrl+Enter` | Commit staged changes (already exists) |
| Stage all | `⌘⇧S` | `Ctrl+Shift+S` | Stage all unstaged files |
| Unstage all | `⌘⇧U` | `Ctrl+Shift+U` | Unstage all staged files |

### Tier 3 — Graph & Search

| Shortcut | macOS | Windows/Linux | Action |
|---|---|---|---|
| Focus search | `⌘F` or `/` | `Ctrl+F` or `/` | Focus search bar in current view |
| Next commit | `J` | `J` | Select next commit in graph (contextual) |
| Previous commit | `K` | `K` | Select previous commit in graph (contextual) |
| First commit | `Home` | `Home` | Jump to first commit (contextual) |
| Last commit | `End` | `End` | Jump to last commit (contextual) |

### Tier 4 — Utility

| Shortcut | macOS | Windows/Linux | Action |
|---|---|---|---|
| Show shortcuts | `?` | `?` | Toggle cheat sheet overlay (contextual) |
| Toggle tasks | `⌘⇧T` | `Ctrl+Shift+T` | Toggle task history popup |

## Cheat Sheet Overlay

A modal overlay triggered by `?` that shows all available shortcuts grouped by category.

### Layout

```
╔══════════════════════════════════════════════╗
║  Keyboard Shortcuts                      ✕   ║
╠══════════════════════════════════════════════╣
║                                              ║
║  Navigation              Git Operations      ║
║  ⌘1  Graph               ⌘⇧F  Fetch         ║
║  ⌘2  Changes             ⌘⇧L  Pull          ║
║  ⌘3  Branches            ⌘⇧P  Push          ║
║  ⌘4  Tags                ⌘⇧S  Stage all     ║
║  ⌘5  Stashes             ⌘⇧U  Unstage all   ║
║  ⌘6  Worktrees           ⌘↵   Commit        ║
║  ⌘,  Settings                                ║
║  ⌘Tab  Next tab          Graph               ║
║  ⌘⇧Tab Prev tab         /    Search          ║
║  ⌘W  Close tab           J    Next commit     ║
║                          K    Prev commit     ║
║  Utility                 Home First commit    ║
║  ?    This overlay        End  Last commit    ║
║  ⌘⇧T  Tasks                                  ║
║                                              ║
╚══════════════════════════════════════════════╝
```

- Two-column grid layout, categories as section headers
- Keys displayed in `<kbd>` styled elements with platform-appropriate symbols
- Dismiss with `Escape`, `?`, or clicking outside
- Fixed position, centered, z-index above everything

### Component

`src/lib/components/common/ShortcutOverlay.svelte`

- Reads shortcut list from the registry store
- Groups by `category` field
- Formats key display using platform detection (⌘ vs Ctrl, ⇧ vs Shift)
- Rendered in `+page.svelte`, toggled by a `showShortcuts` state

## Frontend Files

| File | Purpose |
|---|---|
| `src/lib/stores/shortcuts.ts` | Shortcut registry, definitions, matcher, platform detection |
| `src/lib/components/common/ShortcutOverlay.svelte` | Cheat sheet overlay component |
| `src/routes/+layout.svelte` | Register global keydown listener on mount |
| `src/routes/+page.svelte` | Render ShortcutOverlay, wire navigation actions |

## i18n

All shortcut labels and category names use Paraglide message keys:

- `shortcuts_title` — "Keyboard Shortcuts"
- `shortcuts_cat_navigation` — "Navigation"
- `shortcuts_cat_git` — "Git Operations"
- `shortcuts_cat_graph` — "Graph"
- `shortcuts_cat_utility` — "Utility"
- Individual action labels reuse existing i18n keys where available (e.g. `toolbar_fetch`, `toolbar_pull`)

## Scope Boundaries

**In scope:**
- Central shortcut registry with typed definitions
- Global keydown listener with focus-aware dispatching
- Platform-aware modifier display (⌘ vs Ctrl)
- Cheat sheet overlay (`?`)
- ~20 shortcuts across 4 categories

**Out of scope:**
- User-customizable keybindings (future — add config layer on top of registry)
- Shortcut recording/capture UI (future — for customization)
- Shortcut conflicts detection (unnecessary with hardcoded set)
- Context-specific shortcut sets per view (all shortcuts global for now)
