/**
 * CodeMirror 6 theme bridge.
 *
 * Emits an Extension whose every color is a CSS custom property
 * (`var(--…)`) — the same tokens `theme.ts::applyTheme()` keeps in sync
 * with the active theme's `[editor]` palette. CodeMirror themes are
 * CSS-in-JS, so var() references are valid values; this makes every
 * editor/diff surface follow the app theme live, with no rebuilds and
 * no per-instance theme plumbing.
 *
 * The `_editor` parameter is kept for call-site compatibility (it also
 * serves as a rebuild dependency in the wrappers) but is no longer read.
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
  _editor: ThemeEditorData | null,
  isDark: boolean,
): Extension {
  return [buildChromeTheme(isDark), buildSyntaxHighlighting()];
}

// ---------------------------------------------------------------------------
// Chrome theme (editor background, gutters, diff colors, etc.)
// ---------------------------------------------------------------------------

function buildChromeTheme(isDark: boolean): Extension {
  const bg = 'var(--bg-primary)';
  const bgSecondary = 'var(--bg-secondary)';
  const fg = 'var(--text-primary)';
  const cursor = 'var(--editor-cursor)';
  const selection = 'var(--editor-selection)';
  const lineHighlight = 'var(--editor-line-highlight)';
  const gutterBg = 'var(--editor-gutter-bg)';
  const gutterFg = 'var(--editor-gutter-fg)';
  const border = 'var(--border)';
  const addedBg = 'var(--diff-added-bg)';
  const removedBg = 'var(--diff-removed-bg)';

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

function buildSyntaxHighlighting(): Extension {
  // Same tokens the line-level highlighter consumes (lib/styles/syntax.css).
  const keyword = 'var(--syntax-keyword)';
  const string = 'var(--syntax-string)';
  const comment = 'var(--syntax-comment)';
  const fn = 'var(--syntax-function)';
  const type = 'var(--syntax-type)';
  const number = 'var(--syntax-number)';
  const operator = 'var(--syntax-operator)';
  const property = 'var(--syntax-property)';
  const fg = 'var(--text-primary)';

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

