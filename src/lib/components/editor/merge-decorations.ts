/**
 * CodeMirror decorations for the 3-way merge editor.
 *
 * Side panels: green (added), red (removed), purple (conflict) backgrounds.
 * Center panel: blue background on conflict placeholder lines, with
 * inline widget buttons for accept/ignore actions.
 */

import {
  EditorView,
  Decoration,
  type DecorationSet,
  WidgetType,
} from '@codemirror/view';
import {
  StateField,
  StateEffect,
  type Extension,
  RangeSetBuilder,
} from '@codemirror/state';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Highlight kind for merge decorations. */
export type HighlightKind = 'added' | 'removed' | 'conflict' | 'conflict-active' | 'conflict-center' | 'conflict-center-active';

/** A range of lines to highlight. */
export interface HighlightRange {
  /** 0-based line index in this panel's document. */
  fromLine: number;
  /** Number of lines to highlight. */
  lineCount: number;
  /** Color category. */
  kind: HighlightKind;
  /** Index of this conflict (for conflict chunks). -1 for non-conflict. */
  conflictIndex: number;
}

/** Callbacks for conflict widget accept/ignore buttons. */
export interface ConflictWidgetCallbacks {
  acceptTheirs: (index: number) => void;
  acceptOurs: (index: number) => void;
  ignoreTheirs: (index: number) => void;
  ignoreOurs: (index: number) => void;
}

// ---------------------------------------------------------------------------
// State effects
// ---------------------------------------------------------------------------

/** Effect to set all highlight ranges at once. */
export const setMergeHighlights = StateEffect.define<HighlightRange[]>();

/** Effect to pass conflict widget callbacks into the extension. */
export const setConflictCallbacks = StateEffect.define<ConflictWidgetCallbacks>();

// ---------------------------------------------------------------------------
// Line highlight decorations
// ---------------------------------------------------------------------------

const addedLineDeco = Decoration.line({ class: 'cm-merge-added' });
const removedLineDeco = Decoration.line({ class: 'cm-merge-removed' });
const conflictLineDeco = Decoration.line({ class: 'cm-merge-conflict' });
const conflictActiveDeco = Decoration.line({ class: 'cm-merge-conflict-active' });
const conflictCenterDeco = Decoration.line({ class: 'cm-merge-conflict-center' });
const conflictCenterActiveDeco = Decoration.line({ class: 'cm-merge-conflict-center-active' });

/** Map kind to the correct decoration. */
function decoForKind(kind: HighlightKind): typeof addedLineDeco {
  switch (kind) {
    case 'added': return addedLineDeco;
    case 'removed': return removedLineDeco;
    case 'conflict': return conflictLineDeco;
    case 'conflict-active': return conflictActiveDeco;
    case 'conflict-center': return conflictCenterDeco;
    case 'conflict-center-active': return conflictCenterActiveDeco;
  }
}

/**
 * StateField that listens for `setMergeHighlights` effects and builds
 * line background decorations.
 */
const mergeHighlightField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },

  update(decorations, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setMergeHighlights)) {
        const highlights: HighlightRange[] = effect.value;
        const builder = new RangeSetBuilder<Decoration>();

        // Sort by fromLine to satisfy RangeSetBuilder ordering requirement
        const sorted = [...highlights].sort((a, b) => a.fromLine - b.fromLine);

        for (const range of sorted) {
          const deco = decoForKind(range.kind);
          for (let i = 0; i < range.lineCount; i++) {
            // CodeMirror lines are 1-based
            const lineNum = range.fromLine + i + 1;
            if (lineNum > tr.state.doc.lines) break;
            const line = tr.state.doc.line(lineNum);
            builder.add(line.from, line.from, deco);
          }
        }

        return builder.finish();
      }
    }

    // Map decorations through any document changes
    return decorations.map(tr.changes);
  },

  provide(field) {
    return EditorView.decorations.from(field);
  },
});

/**
 * Returns the line highlight state field extension.
 * Add to an EditorView's extensions, then dispatch `setMergeHighlights` to update.
 */
export function mergeHighlightExtension(): Extension {
  return mergeHighlightField;
}

// ---------------------------------------------------------------------------
// Conflict line widget
// ---------------------------------------------------------------------------

/** Prefix used for conflict placeholder lines in the center editor. */
const CONFLICT_PREFIX = '\u25C6 CONFLICT ';

/**
 * Widget that replaces a conflict placeholder line with accept/ignore buttons.
 *
 * Layout: [ > ] [ x ]   CONFLICT N   [ x ] [ < ]
 */
class ConflictLineWidget extends WidgetType {
  constructor(
    private readonly conflictIndex: number,
    private readonly callbacks: ConflictWidgetCallbacks,
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const wrap = document.createElement('div');
    wrap.className = 'cm-conflict-widget';

    const btnAcceptTheirs = document.createElement('button');
    btnAcceptTheirs.className = 'cm-cw-accept-theirs';
    btnAcceptTheirs.title = 'Accept Theirs';
    btnAcceptTheirs.textContent = '\u276F';
    btnAcceptTheirs.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.callbacks.acceptTheirs(this.conflictIndex);
    });

    const btnIgnoreTheirs = document.createElement('button');
    btnIgnoreTheirs.className = 'cm-cw-ignore';
    btnIgnoreTheirs.title = 'Ignore Theirs';
    btnIgnoreTheirs.textContent = '\u2715';
    btnIgnoreTheirs.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.callbacks.ignoreTheirs(this.conflictIndex);
    });

    const label = document.createElement('span');
    label.className = 'cm-cw-label';
    label.textContent = `\u25C6 CONFLICT ${this.conflictIndex}`;

    const btnIgnoreOurs = document.createElement('button');
    btnIgnoreOurs.className = 'cm-cw-ignore';
    btnIgnoreOurs.title = 'Ignore Ours';
    btnIgnoreOurs.textContent = '\u2715';
    btnIgnoreOurs.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.callbacks.ignoreOurs(this.conflictIndex);
    });

    const btnAcceptOurs = document.createElement('button');
    btnAcceptOurs.className = 'cm-cw-accept-ours';
    btnAcceptOurs.title = 'Accept Ours';
    btnAcceptOurs.textContent = '\u276E';
    btnAcceptOurs.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.callbacks.acceptOurs(this.conflictIndex);
    });

    wrap.appendChild(btnAcceptTheirs);
    wrap.appendChild(btnIgnoreTheirs);
    wrap.appendChild(label);
    wrap.appendChild(btnIgnoreOurs);
    wrap.appendChild(btnAcceptOurs);

    return wrap;
  }

  eq(other: WidgetType): boolean {
    if (!(other instanceof ConflictLineWidget)) return false;
    return other.conflictIndex === this.conflictIndex;
  }
}

// ---------------------------------------------------------------------------
// Conflict line widget extension
// ---------------------------------------------------------------------------

interface ConflictWidgetState {
  decos: DecorationSet;
  callbacks: ConflictWidgetCallbacks | null;
}

/**
 * StateField that scans the document for conflict placeholder lines and
 * replaces them with `ConflictLineWidget` decorations.
 */
const conflictWidgetField = StateField.define<ConflictWidgetState>({
  create() {
    return { decos: Decoration.none, callbacks: null };
  },

  update(value, tr) {
    let callbacks = value.callbacks;

    // Check for callback updates
    for (const effect of tr.effects) {
      if (effect.is(setConflictCallbacks)) {
        callbacks = effect.value;
      }
    }

    // Rebuild decorations on every transaction (doc may have changed)
    const builder = new RangeSetBuilder<Decoration>();

    if (callbacks) {
      const doc = tr.state.doc;
      for (let i = 1; i <= doc.lines; i++) {
        const line = doc.line(i);
        if (line.text.startsWith(CONFLICT_PREFIX)) {
          const indexStr = line.text.slice(CONFLICT_PREFIX.length).trim();
          const conflictIndex = parseInt(indexStr, 10);
          if (!isNaN(conflictIndex)) {
            builder.add(
              line.from,
              line.to,
              Decoration.replace({
                widget: new ConflictLineWidget(conflictIndex, callbacks),
              }),
            );
          }
        }
      }
    }

    return { decos: builder.finish(), callbacks };
  },

  provide(f) {
    return EditorView.decorations.from(f, (v) => v.decos);
  },
});

/**
 * Returns the conflict line widget extension.
 * Dispatch `setConflictCallbacks` to enable the widget buttons.
 */
export function conflictLineWidgetExtension(): Extension {
  return conflictWidgetField;
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

/**
 * Returns a CodeMirror theme with styles for all merge decoration classes.
 * Colors use CSS variables from the app theme.
 *
 * - Green: additions (non-conflict changes)
 * - Red: removals (deleted lines)
 * - Purple: conflict chunks on side panels
 * - Blue: conflict location in center panel
 */
export function mergeDecorationTheme(): Extension {
  return EditorView.theme({
    '.cm-merge-added': {
      backgroundColor: 'color-mix(in srgb, var(--accent-green) 15%, transparent)',
    },
    '.cm-merge-removed': {
      backgroundColor: 'color-mix(in srgb, var(--accent-red) 15%, transparent)',
    },
    '.cm-merge-conflict': {
      backgroundColor: 'color-mix(in srgb, var(--accent-purple) 15%, transparent)',
    },
    '.cm-merge-conflict-active': {
      backgroundColor: 'color-mix(in srgb, var(--accent-purple) 30%, transparent)',
      borderLeft: '2px solid var(--accent-purple)',
    },
    '.cm-merge-conflict-center': {
      backgroundColor: 'color-mix(in srgb, var(--accent-primary) 15%, transparent)',
    },
    '.cm-merge-conflict-center-active': {
      backgroundColor: 'color-mix(in srgb, var(--accent-primary) 30%, transparent)',
      borderLeft: '2px solid var(--accent-primary)',
    },
    '.cm-conflict-widget': {
      display: 'flex',
      alignItems: 'center',
      padding: '0 4px',
      minHeight: '20px',
      backgroundColor: 'color-mix(in srgb, var(--accent-primary) 12%, transparent)',
      borderLeft: '2px solid var(--accent-primary)',
    },
    '.cm-cw-label': {
      flex: '1',
      textAlign: 'center',
      color: 'var(--accent-primary)',
      fontSize: '11px',
      fontFamily: 'var(--font-mono)',
    },
    '.cm-cw-accept-theirs, .cm-cw-accept-ours': {
      background: 'color-mix(in srgb, var(--accent-green) 25%, transparent)',
      border: 'none',
      color: 'var(--accent-green)',
      borderRadius: '3px',
      padding: '2px 8px',
      fontSize: '11px',
      cursor: 'pointer',
      fontWeight: '700',
    },
    '.cm-cw-accept-theirs:hover, .cm-cw-accept-ours:hover': {
      backgroundColor: 'color-mix(in srgb, var(--accent-green) 40%, transparent)',
      color: 'var(--text-primary)',
    },
    '.cm-cw-ignore': {
      background: 'none',
      border: 'none',
      color: 'var(--accent-red)',
      padding: '2px 8px',
      fontSize: '14px',
      cursor: 'pointer',
      opacity: '0.5',
      fontWeight: '700',
      lineHeight: '1',
    },
    '.cm-cw-ignore:hover': {
      color: 'var(--accent-red)',
      opacity: '1',
    },
  });
}
