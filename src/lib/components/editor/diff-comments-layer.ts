/**
 * CodeMirror extension layer rendering inline review threads on a diff.
 *
 * Responsibilities:
 *   1. Gutter bubble with comment count on every line that has 1+ comments.
 *   2. Click on a bubble → thread panel opens anchored under the line,
 *      listing existing comments + a reply composer.
 *   3. Clicking on the gutter → composer opens fresh for a new comment
 *      on that line.
 *
 * Designed for the right-side (new file) of a `MergeView` — the extension
 * ignores old-side lines by matching on `ForgeComment.line` which in both
 * providers refers to the new-side number.
 */

import { type Extension, StateEffect, StateField } from "@codemirror/state";
import {
  EditorView,
  gutter, GutterMarker,
  ViewPlugin, type ViewUpdate,
} from "@codemirror/view";
import type { ForgeComment } from "$lib/types";

export interface DiffCommentsLayerProps {
  /** Pre-filtered to current file; line numbers are 1-based new-side. */
  comments: ForgeComment[];
  /** Called when the composer submits a brand-new comment on `line`. */
  onPost: (line: number, body: string) => Promise<void>;
  /** Called when the reply button submits under an existing thread. */
  onReply: (threadId: string, body: string) => Promise<void>;
  /** Optional — GitLab only. Fires on the inline resolve toggle. */
  onToggleResolve?: (discussionId: string, resolved: boolean) => Promise<void>;
}

/** StateEffect for "open the composer at this line." */
const openComposer = StateEffect.define<{ line: number }>();
/** StateEffect for "close any composer." */
const closeComposer = StateEffect.define<null>();

const composerState = StateField.define<{ line: number } | null>({
  create: () => null,
  update(value, tr) {
    for (const e of tr.effects) {
      if (e.is(openComposer)) return e.value;
      if (e.is(closeComposer)) return null;
    }
    return value;
  },
});

/** Invisible spacer marker — reserves gutter width without rendering text. */
class SpacerMarker extends GutterMarker {
  toDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-comment-spacer";
    el.style.display = "inline-block";
    el.style.width = "16px";
    return el;
  }
}

class BubbleMarker extends GutterMarker {
  constructor(public count: number) {
    super();
  }
  eq(other: GutterMarker): boolean {
    return other instanceof BubbleMarker && other.count === this.count;
  }
  toDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-comment-bubble";
    el.textContent = String(this.count);
    return el;
  }
}

/**
 * Exposed for tests: open the composer programmatically so tests don't
 * have to fake pointer events.
 */
export function __openComposerForTest(view: EditorView, line: number): void {
  view.dispatch({ effects: openComposer.of({ line }) });
}

export function diffCommentsLayer(props: DiffCommentsLayerProps): Extension {
  const byLine = new Map<number, ForgeComment[]>();
  for (const c of props.comments) {
    if (c.line == null) continue;
    const arr = byLine.get(c.line) ?? [];
    arr.push(c);
    byLine.set(c.line, arr);
  }

  const spacer = new SpacerMarker();

  const bubbleGutter = gutter({
    class: "cm-comment-gutter",
    lineMarker(_view, line) {
      const lineNum = _view.state.doc.lineAt(line.from).number;
      const list = byLine.get(lineNum);
      if (!list || list.length === 0) return null;
      return new BubbleMarker(list.length);
    },
    initialSpacer: () => spacer,
    domEventHandlers: {
      click(view, lineBlock) {
        const lineNum = view.state.doc.lineAt(lineBlock.from).number;
        view.dispatch({ effects: openComposer.of({ line: lineNum }) });
        return true;
      },
    },
  });

  /** ViewPlugin that manages a panel div appended to the editor wrapper. */
  const panelPlugin = ViewPlugin.define((view) => {
    let panelDom: HTMLElement | null = null;

    function removePanel() {
      if (panelDom && panelDom.parentElement) {
        panelDom.parentElement.removeChild(panelDom);
      }
      panelDom = null;
    }

    function renderPanel(state: { line: number }) {
      removePanel();
      const thread = byLine.get(state.line) ?? [];
      const dom = buildComposerPanel(view, state.line, thread, props, removePanel);
      dom.style.position = "absolute";
      dom.style.bottom = "0";
      dom.style.left = "0";
      dom.style.right = "0";
      dom.style.zIndex = "10";
      view.dom.style.position = "relative";
      view.dom.appendChild(dom);
      panelDom = dom;
    }

    // Initial render if state already set (unlikely but safe)
    const initial = view.state.field(composerState, false);
    if (initial) renderPanel(initial);

    return {
      update(update: ViewUpdate) {
        for (const tr of [update.transactions].flat()) {
          for (const e of tr.effects) {
            if (e.is(openComposer)) {
              renderPanel(e.value);
              return;
            }
            if (e.is(closeComposer)) {
              removePanel();
              return;
            }
          }
        }
      },
      destroy() {
        removePanel();
      },
    };
  });

  return [composerState, bubbleGutter, panelPlugin, commentGutterTheme];
}

function buildComposerPanel(
  view: EditorView,
  line: number,
  thread: ForgeComment[],
  props: DiffCommentsLayerProps,
  onClose: () => void,
): HTMLElement {
  const dom = document.createElement("div");
  dom.className = "cm-comment-panel";

  if (thread.length > 0) {
    const list = document.createElement("ul");
    list.className = "cm-comment-thread";
    for (const c of thread) {
      const li = document.createElement("li");
      li.className = "cm-comment-item";
      const author = document.createElement("strong");
      author.textContent = c.author;
      const body = document.createElement("div");
      body.textContent = c.body;
      li.appendChild(author);
      li.appendChild(body);
      if (c.resolvable && c.discussion_id && props.onToggleResolve) {
        const btn = document.createElement("button");
        btn.className = "cm-comment-resolve";
        btn.textContent = c.resolved ? "Unresolve" : "Resolve";
        btn.onclick = () => void props.onToggleResolve!(c.discussion_id!, c.resolved === true);
        li.appendChild(btn);
      }
      list.appendChild(li);
    }
    dom.appendChild(list);
  }

  const textarea = document.createElement("textarea");
  textarea.className = "cm-comment-composer";
  textarea.rows = 3;
  textarea.placeholder = thread.length > 0 ? "Reply…" : "Leave a comment…";
  dom.appendChild(textarea);

  const actions = document.createElement("div");
  actions.className = "cm-comment-actions";
  const submit = document.createElement("button");
  submit.className = "cm-comment-submit";
  submit.textContent = thread.length > 0 ? "Reply" : "Comment";
  submit.onclick = async () => {
    const body = textarea.value.trim();
    if (!body) return;
    submit.disabled = true;
    try {
      if (thread.length > 0 && thread[0].discussion_id) {
        await props.onReply(thread[0].discussion_id, body);
      } else {
        await props.onPost(line, body);
      }
      view.dispatch({ effects: closeComposer.of(null) });
      onClose();
    } finally {
      submit.disabled = false;
    }
  };
  const cancel = document.createElement("button");
  cancel.className = "cm-comment-cancel";
  cancel.textContent = "Cancel";
  cancel.onclick = () => {
    view.dispatch({ effects: closeComposer.of(null) });
    onClose();
  };
  actions.appendChild(submit);
  actions.appendChild(cancel);
  dom.appendChild(actions);

  return dom;
}

const commentGutterTheme = EditorView.baseTheme({
  ".cm-comment-bubble": {
    display: "inline-block",
    padding: "0 5px",
    borderRadius: "8px",
    background: "var(--accent-blue)",
    color: "white",
    fontSize: "10px",
    cursor: "pointer",
  },
  ".cm-comment-panel": {
    padding: "8px 12px",
    borderTop: "1px solid var(--border)",
    background: "var(--bg-secondary)",
  },
  ".cm-comment-thread": { listStyle: "none", margin: 0, padding: 0 },
  ".cm-comment-item": { padding: "4px 0" },
  ".cm-comment-composer": { width: "100%", resize: "vertical" },
  ".cm-comment-actions": { display: "flex", gap: "8px", marginTop: "6px" },
  ".cm-comment-submit, .cm-comment-cancel, .cm-comment-resolve": {
    padding: "4px 10px",
    borderRadius: "4px",
    border: "1px solid var(--border)",
    background: "var(--bg-primary)",
    cursor: "pointer",
  },
});

// Suppress unused import warnings for types used only in type positions.
type _Unused = ViewUpdate;
void (undefined as unknown as _Unused);
