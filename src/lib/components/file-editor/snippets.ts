/**
 * Per-language snippet packs for the in-app file editor.
 *
 * Returns a list of `Completion` entries built via CodeMirror's
 * `snippetCompletion(template, …)` factory. Snippets are tab-stop
 * templates (`${1:placeholder}`) — pressing Tab cycles through fields
 * and Esc exits the snippet session.
 *
 * Coverage is intentionally short — the goal is bread-and-butter
 * scaffolds (function shells, control-flow blocks, common type
 * declarations), not an exhaustive list. Users get the rest via
 * keyword completion and `completeAnyWord`.
 *
 * Wired from `EditorPane.svelte` behind the `prefs.snippets` toggle via
 * `EditorState.languageData.of(...)`. The pane derives the language
 * name from the active filename through `getLanguageExtensionName`,
 * then asks this module for the matching pack.
 */

import { snippetCompletion } from "@codemirror/autocomplete";
import type { Completion } from "@codemirror/autocomplete";

/** Rust scaffolds — function shells, control-flow, common type decls. */
const RUST: readonly Completion[] = [
  snippetCompletion("fn ${1:name}(${2}) {\n\t${3}\n}", { label: "fn", detail: "function" }),
  snippetCompletion("pub fn ${1:name}(${2}) -> ${3:()} {\n\t${4}\n}", {
    label: "pub fn",
    detail: "public function",
  }),
  snippetCompletion("impl ${1:Type} {\n\t${2}\n}", { label: "impl", detail: "impl block" }),
  snippetCompletion(
    "match ${1:expr} {\n\t${2:pattern} => ${3:value},\n\t_ => ${4:default},\n}",
    { label: "match", detail: "match expression" },
  ),
  snippetCompletion("let ${1:name} = ${2:value};", { label: "let", detail: "binding" }),
  snippetCompletion("if let ${1:Some(${2:x})} = ${3:expr} {\n\t${4}\n}", {
    label: "if let",
    detail: "if let",
  }),
  snippetCompletion("loop {\n\t${1}\n}", { label: "loop", detail: "loop" }),
  snippetCompletion("for ${1:item} in ${2:iter} {\n\t${3}\n}", {
    label: "for",
    detail: "for-in",
  }),
  snippetCompletion("while ${1:cond} {\n\t${2}\n}", { label: "while", detail: "while" }),
  snippetCompletion("mod ${1:name} {\n\t${2}\n}", { label: "mod", detail: "module" }),
  snippetCompletion("struct ${1:Name} {\n\t${2:field}: ${3:Type},\n}", {
    label: "struct",
    detail: "struct",
  }),
  snippetCompletion("enum ${1:Name} {\n\t${2:Variant},\n}", {
    label: "enum",
    detail: "enum",
  }),
  snippetCompletion("trait ${1:Name} {\n\tfn ${2:method}(&self);\n}", {
    label: "trait",
    detail: "trait",
  }),
  snippetCompletion("#[derive(${1:Debug, Clone})]", { label: "derive", detail: "derive attr" }),
  snippetCompletion("Result<${1:T}, ${2:E}>", { label: "Result", detail: "Result type" }),
  snippetCompletion("Option<${1:T}>", { label: "Option", detail: "Option type" }),
  snippetCompletion('println!("${1}");', { label: "println", detail: "println! macro" }),
  snippetCompletion("dbg!(${1});", { label: "dbg", detail: "dbg! macro" }),
];

/** TypeScript / JavaScript scaffolds. */
const TYPESCRIPT: readonly Completion[] = [
  snippetCompletion("function ${1:name}(${2}): ${3:void} {\n\t${4}\n}", {
    label: "fn",
    detail: "function",
  }),
  snippetCompletion("const ${1:name} = (${2}) => ${3};", {
    label: "arrow",
    detail: "arrow function",
  }),
  snippetCompletion("class ${1:Name} {\n\tconstructor(${2}) {\n\t\t${3}\n\t}\n}", {
    label: "class",
    detail: "class",
  }),
  snippetCompletion("interface ${1:Name} {\n\t${2:field}: ${3:Type};\n}", {
    label: "interface",
    detail: "interface",
  }),
  snippetCompletion("for (const ${1:item} of ${2:iter}) {\n\t${3}\n}", {
    label: "for",
    detail: "for-of",
  }),
  snippetCompletion("if (${1:cond}) {\n\t${2}\n}", { label: "if", detail: "if statement" }),
  snippetCompletion('import { ${1} } from "${2:module}";', {
    label: "import",
    detail: "import statement",
  }),
  snippetCompletion("export ${1:const ${2:name} = ${3:value};}", {
    label: "export",
    detail: "export statement",
  }),
  snippetCompletion("try {\n\t${1}\n} catch (${2:err}) {\n\t${3}\n}", {
    label: "try",
    detail: "try / catch",
  }),
];

/** Python scaffolds. */
const PYTHON: readonly Completion[] = [
  snippetCompletion("def ${1:name}(${2}):\n\t${3:pass}", { label: "def", detail: "function" }),
  snippetCompletion("class ${1:Name}:\n\tdef __init__(self, ${2}):\n\t\t${3:pass}", {
    label: "class",
    detail: "class",
  }),
  snippetCompletion("for ${1:item} in ${2:iter}:\n\t${3:pass}", {
    label: "for",
    detail: "for loop",
  }),
  snippetCompletion("if ${1:cond}:\n\t${2:pass}", { label: "if", detail: "if statement" }),
  snippetCompletion("try:\n\t${1:pass}\nexcept ${2:Exception} as ${3:e}:\n\t${4:pass}", {
    label: "try",
    detail: "try / except",
  }),
  snippetCompletion('with ${1:open(${2:"path"})} as ${3:f}:\n\t${4:pass}', {
    label: "with",
    detail: "with statement",
  }),
];

/** Go scaffolds. */
const GO: readonly Completion[] = [
  snippetCompletion("func ${1:name}(${2}) ${3:error} {\n\t${4}\n}", {
    label: "func",
    detail: "function",
  }),
  snippetCompletion("if ${1:cond} {\n\t${2}\n}", { label: "if", detail: "if statement" }),
  snippetCompletion("for ${1:i := 0; i < ${2:n}; i++} {\n\t${3}\n}", {
    label: "for",
    detail: "for loop",
  }),
  snippetCompletion("type ${1:Name} struct {\n\t${2:Field} ${3:Type}\n}", {
    label: "struct",
    detail: "struct type",
  }),
  snippetCompletion("type ${1:Name} interface {\n\t${2:Method}(${3}) ${4}\n}", {
    label: "interface",
    detail: "interface type",
  }),
  snippetCompletion("package ${1:name}", { label: "package", detail: "package decl" }),
  snippetCompletion("defer ${1:cleanup()}", { label: "defer", detail: "defer call" }),
];

/**
 * Return the snippet pack for `langName` (the value produced by
 * `getLanguageExtensionName`). Returns an empty array for unknown or
 * unmapped languages so the caller can compose unconditionally.
 */
export function snippetsForLanguage(langName: string): readonly Completion[] {
  switch (langName) {
    case "rust":
      return RUST;
    case "typescript":
      return TYPESCRIPT;
    case "python":
      return PYTHON;
    case "go":
      return GO;
    default:
      return [];
  }
}
