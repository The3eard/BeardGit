/**
 * CodeMirror 6 theme bridge.
 *
 * Converts a `ThemeEditorData` payload (from the Rust TOML theme system) into
 * a CodeMirror `Extension` that styles the editor chrome, diff highlights,
 * and syntax tokens.  Falls back to CSS variable values when `editor` is `null`.
 */

import { EditorView } from '@codemirror/view';
import { type Extension } from '@codemirror/state';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import type { ThemeEditorData } from '$lib/types';

/**
 * Build a CodeMirror `Extension[]` from a theme's editor token data.
 *
 * Returns an array containing the base chrome theme AND syntax highlighting.
 */
export function createCodemirrorTheme(
  editor: ThemeEditorData | null,
  isDark: boolean,
): Extension {
  return [
    buildChromeTheme(editor, isDark),
    buildSyntaxHighlighting(editor, isDark),
  ];
}

// ---------------------------------------------------------------------------
// Chrome theme (editor background, gutters, diff colors, etc.)
// ---------------------------------------------------------------------------

function buildChromeTheme(editor: ThemeEditorData | null, isDark: boolean): Extension {
  const bg = editor?.background ?? getCssVar('--bg-primary', '#0d1117');
  const bgSecondary = getCssVar('--bg-secondary', '#161b22');
  const fg = editor?.foreground ?? getCssVar('--text-primary', '#e6edf3');
  const cursor = editor?.cursor ?? getCssVar('--accent-blue', '#58a6ff');
  const selection = editor?.selection ?? getCssVar('--selection', '#1f6feb44');
  const lineHighlight = editor?.line_highlight ?? 'transparent';
  const gutterBg = editor?.gutter_bg ?? bg;
  const gutterFg = editor?.gutter_fg ?? getCssVar('--text-secondary', '#8b949e');
  const border = getCssVar('--border', '#30363d');
  const addedBg = editor?.added_bg ?? (isDark ? '#1b3829' : '#d4f8db');
  const removedBg = editor?.removed_bg ?? (isDark ? '#3c1e22' : '#fdd8d8');
  const addedText = editor?.added_text ?? '#3fb950';
  const removedText = editor?.removed_text ?? '#f85149';

  return EditorView.theme(
    {
      // -- Base editor chrome --
      '&': {
        backgroundColor: bg,
        color: fg,
        fontFamily: "'Fira Code', var(--font-mono), monospace",
        fontSize: '12px',
        lineHeight: '1.6',
      },
      '.cm-content': {
        caretColor: cursor,
        padding: '4px 0',
      },
      '.cm-cursor, .cm-dropCursor': {
        borderLeftColor: cursor,
        borderLeftWidth: '2px',
      },
      '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
        backgroundColor: selection,
      },
      '.cm-activeLine': {
        backgroundColor: lineHighlight,
      },

      // -- Gutters (line numbers) --
      '.cm-gutters': {
        backgroundColor: gutterBg,
        color: gutterFg,
        borderRight: `1px solid ${border}`,
        fontSize: '11px',
        minWidth: '40px',
      },
      '.cm-lineNumbers .cm-gutterElement': {
        padding: '0 8px 0 4px',
        minWidth: '32px',
        textAlign: 'right',
      },
      '.cm-activeLineGutter': {
        backgroundColor: lineHighlight,
        color: fg,
      },

      // -- Fold gutters --
      '.cm-foldGutter': {
        width: '12px',
      },

      // -- Scrollbar --
      '.cm-scroller': {
        scrollbarWidth: 'thin',
        scrollbarColor: `${border} transparent`,
      },

      // -- Merge/diff view --
      '.cm-mergeView': {
        height: '100%',
      },
      '.cm-mergeViewEditors': {
        gap: '1px',
        backgroundColor: border,
      },
      '.cm-mergeViewEditor': {
        backgroundColor: bg,
      },

      // -- Diff changed lines (merge-a = old/deleted side, merge-b = new/added side) --
      '&.cm-merge-a .cm-changedLine, .cm-deletedChunk': {
        backgroundColor: `${removedBg} !important`,
      },
      '&.cm-merge-b .cm-changedLine, .cm-inlineChangedLine': {
        backgroundColor: `${addedBg} !important`,
      },

      // -- Disable inline text underlines (keep lines clean) --
      '&.cm-merge-a .cm-changedText, .cm-deletedChunk .cm-deletedText': {
        background: 'none !important',
        textDecoration: 'none !important',
      },
      '&.cm-merge-b .cm-changedText': {
        background: 'none !important',
        textDecoration: 'none !important',
      },

      // -- Diff gutter markers --
      '.cm-changeGutter': {
        width: '3px',
        minWidth: '3px',
        paddingLeft: '0',
      },
      '.cm-changeGutter .cm-gutterElement': {
        padding: '0',
      },

      // -- Collapse unchanged regions --
      '.cm-collapsedLines': {
        backgroundColor: bgSecondary,
        color: gutterFg,
        borderTop: `1px solid ${border}`,
        borderBottom: `1px solid ${border}`,
        padding: '2px 12px',
        fontSize: '11px',
        fontStyle: 'italic',
        cursor: 'pointer',
      },
      '.cm-collapsedLines:hover': {
        backgroundColor: border,
      },

      // -- Tooltips --
      '.cm-tooltip': {
        backgroundColor: bgSecondary,
        color: fg,
        border: `1px solid ${border}`,
        borderRadius: '6px',
        fontSize: '12px',
      },

      // -- Matching brackets --
      '&.cm-focused .cm-matchingBracket': {
        backgroundColor: selection,
        outline: `1px solid ${gutterFg}`,
      },
    },
    { dark: isDark },
  );
}

// ---------------------------------------------------------------------------
// Syntax highlighting
// ---------------------------------------------------------------------------

function buildSyntaxHighlighting(editor: ThemeEditorData | null, isDark: boolean): Extension {
  // Derive syntax colors from editor data or sensible defaults per mode
  const keyword = editor?.syntax_keyword ?? (isDark ? '#ff7b72' : '#cf222e');
  const string = editor?.syntax_string ?? (isDark ? '#a5d6ff' : '#0a3069');
  const comment = editor?.syntax_comment ?? (isDark ? '#8b949e' : '#6e7781');
  const fn = editor?.syntax_function ?? (isDark ? '#d2a8ff' : '#8250df');
  const type = editor?.syntax_type ?? (isDark ? '#79c0ff' : '#0550ae');
  const number = editor?.syntax_number ?? (isDark ? '#79c0ff' : '#0550ae');
  const operator = editor?.syntax_operator ?? (isDark ? '#ff7b72' : '#cf222e');
  const property = editor?.syntax_property ?? (isDark ? '#7ee787' : '#116329');
  const fg = editor?.foreground ?? (isDark ? '#e6edf3' : '#1f2328');

  const highlightStyle = HighlightStyle.define([
    // Keywords: if, else, fn, let, const, return, import, export, etc.
    { tag: tags.keyword, color: keyword },
    { tag: tags.controlKeyword, color: keyword },
    { tag: tags.moduleKeyword, color: keyword },
    { tag: tags.operatorKeyword, color: keyword },
    { tag: tags.definitionKeyword, color: keyword },

    // Operators: +, -, =, =>, !, &&, ||, etc.
    { tag: tags.operator, color: operator },
    { tag: tags.compareOperator, color: operator },
    { tag: tags.logicOperator, color: operator },
    { tag: tags.arithmeticOperator, color: operator },
    { tag: tags.updateOperator, color: operator },
    { tag: tags.derefOperator, color: operator },

    // Strings and related
    { tag: tags.string, color: string },
    { tag: tags.special(tags.string), color: string },
    { tag: tags.regexp, color: string, fontStyle: 'italic' },
    { tag: tags.escape, color: keyword },

    // Comments
    { tag: tags.comment, color: comment, fontStyle: 'italic' },
    { tag: tags.lineComment, color: comment, fontStyle: 'italic' },
    { tag: tags.blockComment, color: comment, fontStyle: 'italic' },
    { tag: tags.docComment, color: comment, fontStyle: 'italic' },

    // Functions
    { tag: tags.function(tags.variableName), color: fn },
    { tag: tags.function(tags.definition(tags.variableName)), color: fn },

    // Types and classes
    { tag: tags.typeName, color: type },
    { tag: tags.className, color: type },
    { tag: tags.namespace, color: type },
    { tag: tags.macroName, color: fn },

    // Numbers and booleans
    { tag: tags.number, color: number },
    { tag: tags.integer, color: number },
    { tag: tags.float, color: number },
    { tag: tags.bool, color: number },
    { tag: tags.null, color: number },

    // Properties and attributes
    { tag: tags.propertyName, color: property },
    { tag: tags.attributeName, color: property },
    { tag: tags.definition(tags.propertyName), color: property },

    // Variables
    { tag: tags.variableName, color: fg },
    { tag: tags.definition(tags.variableName), color: fg },
    { tag: tags.local(tags.variableName), color: fg },
    { tag: tags.special(tags.variableName), color: keyword },
    { tag: tags.self, color: keyword },

    // Punctuation and brackets
    { tag: tags.punctuation, color: fg, opacity: '0.8' },
    { tag: tags.paren, color: fg, opacity: '0.8' },
    { tag: tags.brace, color: fg, opacity: '0.8' },
    { tag: tags.squareBracket, color: fg, opacity: '0.8' },
    { tag: tags.angleBracket, color: fg, opacity: '0.8' },
    { tag: tags.separator, color: fg, opacity: '0.7' },

    // Tags (HTML/XML)
    { tag: tags.tagName, color: keyword },
    { tag: tags.attributeValue, color: string },

    // Meta / annotations / decorators
    { tag: tags.meta, color: comment },
    { tag: tags.annotation, color: fn },

    // Labels and headings (Markdown)
    { tag: tags.heading, color: keyword, fontWeight: 'bold' },
    { tag: tags.link, color: type, textDecoration: 'underline' },
    { tag: tags.url, color: type, textDecoration: 'underline' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, fontWeight: 'bold' },
  ]);

  return syntaxHighlighting(highlightStyle);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getCssVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}
