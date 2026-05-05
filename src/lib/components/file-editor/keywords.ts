/**
 * Per-language keyword lists for the in-app file editor.
 *
 * Combined with `keywordCompletion()` (below) to produce a CodeMirror
 * `CompletionSource` that suggests the active language's reserved
 * words. The popup tags each entry with `type: "keyword"` so the icon
 * comes out right.
 *
 * Curated, not exhaustive — the goal is the words a user actually
 * types. `completeAnyWord` covers the long tail by scanning the
 * buffer.
 */

import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";

const RUST = [
  "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
  "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
  "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
  "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
  "where", "while",
] as const;

const TYPESCRIPT = [
  "abstract", "any", "as", "async", "await", "boolean", "break", "case",
  "catch", "class", "const", "continue", "debugger", "declare", "default",
  "delete", "do", "else", "enum", "export", "extends", "false", "finally",
  "for", "from", "function", "get", "if", "implements", "import", "in",
  "instanceof", "interface", "is", "keyof", "let", "namespace", "never",
  "new", "null", "number", "of", "package", "private", "protected", "public",
  "readonly", "return", "set", "static", "string", "super", "switch", "this",
  "throw", "true", "try", "type", "typeof", "undefined", "unknown", "var",
  "void", "while", "with", "yield",
] as const;

const PYTHON = [
  "False", "None", "True", "and", "as", "assert", "async", "await", "break",
  "class", "continue", "def", "del", "elif", "else", "except", "finally",
  "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
  "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
] as const;

const GO = [
  "break", "case", "chan", "const", "continue", "default", "defer", "else",
  "fallthrough", "for", "func", "go", "goto", "if", "import", "interface",
  "map", "package", "range", "return", "select", "struct", "switch", "type",
  "var",
] as const;

const C = [
  "auto", "break", "case", "char", "const", "continue", "default", "do",
  "double", "else", "enum", "extern", "float", "for", "goto", "if", "inline",
  "int", "long", "register", "restrict", "return", "short", "signed",
  "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned",
  "void", "volatile", "while",
] as const;

const CPP = [
  ...C,
  "alignas", "alignof", "and", "asm", "bool", "catch", "class", "constexpr",
  "const_cast", "decltype", "delete", "dynamic_cast", "explicit", "export",
  "false", "friend", "mutable", "namespace", "new", "noexcept", "not",
  "nullptr", "operator", "or", "private", "protected", "public",
  "reinterpret_cast", "static_assert", "static_cast", "template", "this",
  "thread_local", "throw", "true", "try", "typeid", "typename", "using",
  "virtual", "wchar_t", "xor",
] as const;

const JAVA = [
  "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char",
  "class", "const", "continue", "default", "do", "double", "else", "enum",
  "extends", "false", "final", "finally", "float", "for", "goto", "if",
  "implements", "import", "instanceof", "int", "interface", "long", "native",
  "new", "null", "package", "private", "protected", "public", "return",
  "short", "static", "strictfp", "super", "switch", "synchronized", "this",
  "throw", "throws", "transient", "true", "try", "var", "void", "volatile",
  "while", "yield",
] as const;

const CSS = [
  "@charset", "@import", "@namespace", "@media", "@supports", "@font-face",
  "@keyframes", "@page", "@layer", "@container", "@property",
  "auto", "inherit", "initial", "none", "unset", "revert",
  "absolute", "relative", "fixed", "sticky", "static",
  "block", "inline", "inline-block", "flex", "grid", "contents", "table",
  "row", "column", "row-reverse", "column-reverse",
  "center", "start", "end", "stretch", "space-between", "space-around",
  "space-evenly", "wrap", "nowrap",
  "bold", "italic", "underline", "uppercase", "lowercase", "capitalize",
  "transparent", "currentColor",
] as const;

/**
 * Return the curated keyword list for `langName` (the value produced by
 * `getLanguageExtensionName`). Returns an empty array for unknown
 * languages so callers can compose unconditionally.
 */
export function keywordsForLanguage(langName: string): readonly string[] {
  switch (langName) {
    case "rust":
      return RUST;
    case "typescript":
      return TYPESCRIPT;
    case "python":
      return PYTHON;
    case "go":
      return GO;
    case "cpp":
      return CPP;
    case "java":
      return JAVA;
    case "css":
      return CSS;
    default:
      return [];
  }
}

/**
 * Build a CodeMirror `CompletionSource` that suggests the given
 * keywords. Matches the word currently being typed (`/\w+/` before the
 * caret) and returns an entry for each word, tagged
 * `type: "keyword"` so the popup icon is correct.
 *
 * Returns `null` when no word is being typed and the user hasn't
 * explicitly invoked completion — same convention as
 * `completeAnyWord`.
 */
export function keywordCompletion(
  words: readonly string[],
): (context: CompletionContext) => CompletionResult | null {
  const options: Completion[] = words.map((word) => ({ label: word, type: "keyword" }));
  return (context: CompletionContext) => {
    const word = context.matchBefore(/\w+/);
    if (!word || (word.from === word.to && !context.explicit)) {
      return null;
    }
    return {
      from: word.from,
      options,
      validFor: /^\w*$/,
    };
  };
}
