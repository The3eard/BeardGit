/**
 * CodeMirror decorations for the 3-way merge editor.
 *
 * Side panels: line backgrounds (added / conflict / active conflict) plus a
 * block widget above every conflict chunk with accept / discard buttons for
 * that side. Center panel: each conflict placeholder line is replaced by a
 * widget carrying the same actions for both sides, with the state of each
 * side read from the placeholder text (see `merge-placeholder.ts`).
 *
 * The buttons act on `mousedown`, as `@codemirror/merge` does for its own
 * controls: inside a contenteditable host the browser may move focus and
 * selection between mousedown and mouseup and swallow the click.
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
import * as m from '$lib/paraglide/messages';
import {
  parsePlaceholder,
  type ConflictMarker,
  type ConflictSide,
  type SideDecision,
} from './merge-placeholder';

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

/** Actions the conflict widgets can trigger. */
export interface ConflictWidgetCallbacks {
  decide: (index: number, side: ConflictSide, decision: 'accepted' | 'discarded') => void;
  /** The user pointed at a conflict; used to highlight it across panels. */
  activate: (index: number) => void;
}

/** One conflict chunk as shown on a side panel. */
export interface SideConflict {
  index: number;
  /** 0-based first line of the chunk on this side. */
  fromLine: number;
  /** Line count on this side; 0 when this side deleted the block. */
  lineCount: number;
  decision: SideDecision;
  active: boolean;
}

// ---------------------------------------------------------------------------
// State effects
// ---------------------------------------------------------------------------

/** Effect to set all highlight ranges at once. */
export const setMergeHighlights = StateEffect.define<HighlightRange[]>();

/** Effect to pass conflict widget callbacks into the extension. */
export const setConflictCallbacks = StateEffect.define<ConflictWidgetCallbacks>();

/** Effect to (re)publish the conflict chunks of a side panel. */
export const setSideConflicts = StateEffect.define<{ side: ConflictSide; conflicts: SideConflict[] }>();

/** Effect to mark which conflict is active in the center panel. */
export const setActiveConflict = StateEffect.define<number | null>();

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
// Shared button factory
// ---------------------------------------------------------------------------

function button(
  className: string,
  text: string,
  title: string,
  onAct: () => void,
): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = className;
  btn.textContent = text;
  btn.title = title;
  btn.setAttribute('aria-label', title);
  btn.addEventListener('mousedown', (e) => {
    e.preventDefault();
    e.stopPropagation();
    onAct();
  });
  // Keyboard activation still arrives as a click.
  btn.addEventListener('click', (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.detail === 0) onAct();
  });
  return btn;
}

function sideLabel(side: ConflictSide): string {
  return side === 'theirs' ? m.merge_panel_theirs() : m.merge_panel_ours();
}

function acceptTitle(side: ConflictSide): string {
  return side === 'theirs' ? m.merge_accept_theirs() : m.merge_accept_ours();
}

function discardTitle(side: ConflictSide): string {
  return side === 'theirs' ? m.merge_discard_theirs() : m.merge_discard_ours();
}

function decisionBadge(decision: SideDecision): HTMLElement {
  const badge = document.createElement('span');
  badge.className = `cm-cw-state cm-cw-state-${decision}`;
  badge.textContent = decision === 'accepted' ? m.merge_side_accepted() : m.merge_side_discarded();
  return badge;
}

// ---------------------------------------------------------------------------
// Center conflict widget
// ---------------------------------------------------------------------------

/**
 * Widget that replaces a conflict placeholder line in the result panel.
 *
 * Layout: [❯ accept theirs] [✕]   ◆ Conflict N   [✕] [accept ours ❮]
 * A decided side shows its state instead of its buttons.
 */
class ConflictLineWidget extends WidgetType {
  constructor(
    private readonly marker: ConflictMarker,
    private readonly active: boolean,
    private readonly callbacks: ConflictWidgetCallbacks,
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const { index } = this.marker;
    const wrap = document.createElement('div');
    wrap.className = 'cm-conflict-widget' + (this.active ? ' cm-conflict-widget-active' : '');
    wrap.addEventListener('mousedown', () => this.callbacks.activate(index));

    const sideGroup = (side: ConflictSide): HTMLElement => {
      const group = document.createElement('span');
      group.className = `cm-cw-side cm-cw-side-${side}`;
      const decision = this.marker[side];
      if (decision !== 'pending') {
        group.appendChild(decisionBadge(decision));
        return group;
      }
      const accept = button(
        `cm-cw-accept cm-cw-accept-${side}`,
        side === 'theirs' ? '❯' : '❮',
        acceptTitle(side),
        () => this.callbacks.decide(index, side, 'accepted'),
      );
      const discard = button(
        'cm-cw-ignore',
        '✕',
        discardTitle(side),
        () => this.callbacks.decide(index, side, 'discarded'),
      );
      if (side === 'theirs') {
        group.append(accept, discard);
      } else {
        group.append(discard, accept);
      }
      return group;
    };

    const label = document.createElement('span');
    label.className = 'cm-cw-label';
    label.textContent = `◆ ${m.merge_conflict_label({ n: String(index + 1) })}`;

    wrap.append(sideGroup('theirs'), label, sideGroup('ours'));
    return wrap;
  }

  eq(other: WidgetType): boolean {
    if (!(other instanceof ConflictLineWidget)) return false;
    return (
      other.marker.index === this.marker.index &&
      other.marker.theirs === this.marker.theirs &&
      other.marker.ours === this.marker.ours &&
      other.active === this.active
    );
  }

  ignoreEvent(): boolean {
    return true;
  }
}

interface ConflictWidgetState {
  decos: DecorationSet;
  callbacks: ConflictWidgetCallbacks | null;
  active: number | null;
}

/**
 * StateField that scans the document for conflict placeholder lines and
 * replaces them with `ConflictLineWidget` decorations.
 */
const conflictWidgetField = StateField.define<ConflictWidgetState>({
  create() {
    return { decos: Decoration.none, callbacks: null, active: null };
  },

  update(value, tr) {
    let { callbacks, active } = value;

    for (const effect of tr.effects) {
      if (effect.is(setConflictCallbacks)) callbacks = effect.value;
      if (effect.is(setActiveConflict)) active = effect.value;
    }

    // Rebuild decorations on every transaction (doc may have changed)
    const builder = new RangeSetBuilder<Decoration>();

    if (callbacks) {
      const doc = tr.state.doc;
      for (let i = 1; i <= doc.lines; i++) {
        const line = doc.line(i);
        const marker = parsePlaceholder(line.text);
        if (!marker) continue;
        builder.add(
          line.from,
          line.to,
          Decoration.replace({
            widget: new ConflictLineWidget(marker, marker.index === active, callbacks),
          }),
        );
      }
    }

    return { decos: builder.finish(), callbacks, active };
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
// Side panel conflict widget
// ---------------------------------------------------------------------------

/**
 * Block widget shown above a conflict chunk on a side panel.
 *
 * Layout: ◆ Conflict N · <side>   [Accept] [Discard]
 * Once this side is decided the buttons give way to a state badge.
 */
class SideConflictWidget extends WidgetType {
  constructor(
    private readonly side: ConflictSide,
    private readonly conflict: SideConflict,
    private readonly callbacks: ConflictWidgetCallbacks,
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const { index, decision, lineCount, active } = this.conflict;
    const wrap = document.createElement('div');
    wrap.className =
      `cm-side-conflict cm-side-conflict-${this.side}` +
      (active ? ' cm-side-conflict-active' : '') +
      (decision !== 'pending' ? ' cm-side-conflict-decided' : '');
    wrap.addEventListener('mousedown', () => this.callbacks.activate(index));

    const label = document.createElement('span');
    label.className = 'cm-cw-label';
    label.textContent = `◆ ${m.merge_conflict_label({ n: String(index + 1) })}`;
    wrap.appendChild(label);

    if (lineCount === 0) {
      const note = document.createElement('span');
      note.className = 'cm-cw-note';
      note.textContent = m.merge_side_deleted();
      wrap.appendChild(note);
    }

    const spacer = document.createElement('span');
    spacer.className = 'cm-cw-spacer';
    wrap.appendChild(spacer);

    if (decision !== 'pending') {
      wrap.appendChild(decisionBadge(decision));
      return wrap;
    }

    wrap.appendChild(
      button(
        `cm-cw-accept cm-cw-accept-${this.side}`,
        `${this.side === 'theirs' ? '❯ ' : '❮ '}${m.merge_accept()}`,
        acceptTitle(this.side),
        () => this.callbacks.decide(index, this.side, 'accepted'),
      ),
    );
    wrap.appendChild(
      button(
        'cm-cw-ignore',
        `✕ ${m.merge_discard()}`,
        discardTitle(this.side),
        () => this.callbacks.decide(index, this.side, 'discarded'),
      ),
    );
    return wrap;
  }

  eq(other: WidgetType): boolean {
    if (!(other instanceof SideConflictWidget)) return false;
    const a = this.conflict;
    const b = other.conflict;
    return (
      other.side === this.side &&
      a.index === b.index &&
      a.fromLine === b.fromLine &&
      a.lineCount === b.lineCount &&
      a.decision === b.decision &&
      a.active === b.active
    );
  }

  ignoreEvent(): boolean {
    return true;
  }

  get estimatedHeight(): number {
    return 24;
  }
}

interface SideWidgetState {
  decos: DecorationSet;
  callbacks: ConflictWidgetCallbacks | null;
  side: ConflictSide;
  conflicts: SideConflict[];
}

function buildSideDecos(state: SideWidgetState, doc: import('@codemirror/state').Text): DecorationSet {
  if (!state.callbacks) return Decoration.none;
  const builder = new RangeSetBuilder<Decoration>();
  const sorted = [...state.conflicts].sort((a, b) => a.fromLine - b.fromLine);
  let lastPos = -1;
  for (const conflict of sorted) {
    // A side that deleted the block anchors its widget to the line that
    // follows the deletion, or to the end of the document.
    const lineNum = Math.min(conflict.fromLine + 1, doc.lines);
    const pos = conflict.fromLine >= doc.lines ? doc.length : doc.line(lineNum).from;
    if (pos < lastPos) continue;
    lastPos = pos;
    builder.add(
      pos,
      pos,
      Decoration.widget({
        widget: new SideConflictWidget(state.side, conflict, state.callbacks),
        block: true,
        side: -1,
      }),
    );
  }
  return builder.finish();
}

const sideConflictField = StateField.define<SideWidgetState>({
  create() {
    return { decos: Decoration.none, callbacks: null, side: 'theirs', conflicts: [] };
  },

  update(value, tr) {
    let next = value;
    let dirty = false;
    for (const effect of tr.effects) {
      if (effect.is(setConflictCallbacks)) {
        next = { ...next, callbacks: effect.value };
        dirty = true;
      } else if (effect.is(setSideConflicts)) {
        next = { ...next, side: effect.value.side, conflicts: effect.value.conflicts };
        dirty = true;
      }
    }
    if (!dirty) {
      return tr.docChanged ? { ...value, decos: value.decos.map(tr.changes) } : value;
    }
    return { ...next, decos: buildSideDecos(next, tr.state.doc) };
  },

  provide(f) {
    return EditorView.decorations.from(f, (v) => v.decos);
  },
});

/**
 * Returns the side-panel conflict widget extension. Dispatch
 * `setConflictCallbacks` once and `setSideConflicts` whenever the
 * conflicts' state changes.
 */
export function sideConflictWidgetExtension(): Extension {
  return sideConflictField;
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
 * - Accent: conflict location in center panel
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
      gap: '4px',
      padding: '2px 6px',
      minHeight: '24px',
      backgroundColor: 'color-mix(in srgb, var(--accent-primary) 12%, transparent)',
      borderLeft: '2px solid var(--accent-primary)',
      cursor: 'default',
    },
    '.cm-conflict-widget-active': {
      backgroundColor: 'color-mix(in srgb, var(--accent-primary) 24%, transparent)',
      boxShadow: 'inset 0 0 0 1px color-mix(in srgb, var(--accent-primary) 40%, transparent)',
    },
    '.cm-cw-side': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '2px',
      minWidth: '58px',
    },
    '.cm-cw-side-ours': {
      justifyContent: 'flex-end',
    },
    '.cm-side-conflict': {
      display: 'flex',
      alignItems: 'center',
      gap: '6px',
      padding: '2px 6px 2px 8px',
      minHeight: '24px',
      backgroundColor: 'color-mix(in srgb, var(--accent-purple) 18%, transparent)',
      borderLeft: '2px solid var(--accent-purple)',
      borderBottom: '1px solid color-mix(in srgb, var(--accent-purple) 35%, transparent)',
      fontFamily: 'var(--font-mono)',
      fontSize: '11px',
      cursor: 'default',
    },
    '.cm-side-conflict-active': {
      backgroundColor: 'color-mix(in srgb, var(--accent-purple) 32%, transparent)',
    },
    '.cm-side-conflict-decided': {
      backgroundColor: 'color-mix(in srgb, var(--accent-green) 12%, transparent)',
      borderLeftColor: 'var(--accent-green)',
      borderBottomColor: 'color-mix(in srgb, var(--accent-green) 30%, transparent)',
    },
    '.cm-cw-spacer': {
      flex: '1',
    },
    '.cm-cw-label': {
      flex: '1',
      textAlign: 'center',
      color: 'var(--accent-primary)',
      fontSize: '11px',
      fontFamily: 'var(--font-mono)',
      whiteSpace: 'nowrap',
    },
    '.cm-side-conflict .cm-cw-label': {
      flex: '0 0 auto',
      color: 'var(--text-primary)',
      fontWeight: '600',
    },
    '.cm-cw-note': {
      color: 'var(--text-secondary)',
      fontStyle: 'italic',
      whiteSpace: 'nowrap',
    },
    '.cm-cw-accept': {
      background: 'color-mix(in srgb, var(--accent-green) 25%, transparent)',
      border: 'none',
      color: 'var(--accent-green)',
      borderRadius: '3px',
      padding: '2px 8px',
      fontSize: '11px',
      fontFamily: 'var(--font-mono)',
      cursor: 'pointer',
      fontWeight: '700',
      whiteSpace: 'nowrap',
      lineHeight: '16px',
    },
    '.cm-cw-accept:hover': {
      backgroundColor: 'color-mix(in srgb, var(--accent-green) 40%, transparent)',
      color: 'var(--text-primary)',
    },
    '.cm-cw-ignore': {
      background: 'none',
      border: '1px solid transparent',
      borderRadius: '3px',
      color: 'var(--accent-red)',
      padding: '2px 6px',
      fontSize: '11px',
      fontFamily: 'var(--font-mono)',
      cursor: 'pointer',
      opacity: '0.7',
      fontWeight: '700',
      whiteSpace: 'nowrap',
      lineHeight: '16px',
    },
    '.cm-cw-ignore:hover': {
      opacity: '1',
      borderColor: 'color-mix(in srgb, var(--accent-red) 40%, transparent)',
      backgroundColor: 'color-mix(in srgb, var(--accent-red) 12%, transparent)',
    },
    '.cm-cw-accept:focus-visible, .cm-cw-ignore:focus-visible': {
      outline: '2px solid var(--accent-primary)',
      outlineOffset: '1px',
    },
    '.cm-cw-state': {
      fontSize: '10px',
      fontFamily: 'var(--font-mono)',
      fontWeight: '700',
      textTransform: 'uppercase',
      letterSpacing: '0.4px',
      padding: '1px 6px',
      borderRadius: '3px',
      whiteSpace: 'nowrap',
    },
    '.cm-cw-state-accepted': {
      color: 'var(--accent-green)',
      backgroundColor: 'color-mix(in srgb, var(--accent-green) 18%, transparent)',
    },
    '.cm-cw-state-discarded': {
      color: 'var(--text-secondary)',
      backgroundColor: 'color-mix(in srgb, var(--text-secondary) 15%, transparent)',
      textDecoration: 'line-through',
    },
  });
}
