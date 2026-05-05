import type { Extension } from '@codemirror/state';

/**
 * Map of lowercase file extension → language name. Used by
 * `getLanguageExtensionName` for the common case (extension-driven
 * detection); special basenames (`Dockerfile`, `Makefile`, …) are
 * handled separately in `BASENAME_MAP` because they have no extension.
 */
const EXTENSION_MAP: Record<string, string> = {
  ts: 'typescript', tsx: 'typescript', js: 'typescript', jsx: 'typescript', mjs: 'typescript',
  rs: 'rust',
  py: 'python', pyw: 'python',
  css: 'css', scss: 'css', less: 'css',
  html: 'html', htm: 'html', svelte: 'html', vue: 'html',
  json: 'json',
  yaml: 'yaml', yml: 'yaml',
  md: 'markdown', mdx: 'markdown',
  java: 'java',
  go: 'go',
  cpp: 'cpp', c: 'cpp', h: 'cpp', hpp: 'cpp', cc: 'cpp', cxx: 'cpp',
  sql: 'sql',
  xml: 'xml', svg: 'xml',
  toml: 'toml',
  sh: 'shell', bash: 'shell', zsh: 'shell',
  dockerfile: 'dockerfile',
  mk: 'makefile',
  ini: 'properties',
  properties: 'properties',
  lua: 'lua',
  pl: 'perl',
  r: 'r',
  nginx: 'nginx',
};

/**
 * Map of basename (case-sensitive, no extension) → language name. Used
 * by `getLanguageExtensionName` before the extension lookup so files
 * named `Dockerfile` / `Makefile` get the right grammar even though
 * they have no extension.
 */
const BASENAME_MAP: Record<string, string> = {
  Dockerfile: 'dockerfile',
  Makefile: 'makefile',
  GNUmakefile: 'makefile',
};

/** Module-level cache for loaded CodeMirror language extensions. */
const languageCache = new Map<string, Extension>();

/**
 * Returns a language name string for the given file path, or null if
 * unknown. Checks the basename first (so `Dockerfile` resolves even
 * without an extension), then falls back to the lowercase extension
 * lookup.
 */
export function getLanguageExtensionName(path: string): string | null {
  // Strip directory components — basename is what we match against the
  // BASENAME_MAP and is also what the extension lookup needs to handle
  // dotfile-style names like `.eslintrc`.
  const basename = path.split(/[\\/]/).pop() ?? path;
  const direct = BASENAME_MAP[basename];
  if (direct) return direct;
  const ext = basename.split('.').pop()?.toLowerCase();
  if (!ext || ext === basename.toLowerCase()) return null;
  return EXTENSION_MAP[ext] ?? null;
}

/**
 * Lazy-loads the language extension without caching. Native CodeMirror
 * `lang-*` packs are preferred where they exist; languages backed by
 * `@codemirror/legacy-modes` use `StreamLanguage.define(...)` instead.
 */
async function loadLanguageExtensionUncached(langName: string): Promise<Extension | null> {
  switch (langName) {
    case 'typescript': {
      const { javascript } = await import('@codemirror/lang-javascript');
      return javascript({ typescript: true, jsx: true });
    }
    case 'rust': {
      const { rust } = await import('@codemirror/lang-rust');
      return rust();
    }
    case 'python': {
      const { python } = await import('@codemirror/lang-python');
      return python();
    }
    case 'css': {
      const { css } = await import('@codemirror/lang-css');
      return css();
    }
    case 'html': {
      const { html } = await import('@codemirror/lang-html');
      return html();
    }
    case 'json': {
      const { json } = await import('@codemirror/lang-json');
      return json();
    }
    case 'yaml': {
      const { yaml } = await import('@codemirror/lang-yaml');
      return yaml();
    }
    case 'markdown': {
      const { markdown } = await import('@codemirror/lang-markdown');
      return markdown();
    }
    case 'java': {
      const { java } = await import('@codemirror/lang-java');
      return java();
    }
    case 'go': {
      const { go } = await import('@codemirror/lang-go');
      return go();
    }
    case 'cpp': {
      const { cpp } = await import('@codemirror/lang-cpp');
      return cpp();
    }
    case 'sql': {
      const { sql } = await import('@codemirror/lang-sql');
      return sql();
    }
    case 'xml': {
      const { xml } = await import('@codemirror/lang-xml');
      return xml();
    }
    // ── Legacy modes (no dedicated `lang-*` pack) ────────────────────
    case 'toml': {
      const { toml } = await import('@codemirror/legacy-modes/mode/toml');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(toml);
    }
    case 'shell': {
      const { shell } = await import('@codemirror/legacy-modes/mode/shell');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(shell);
    }
    case 'dockerfile': {
      const { dockerFile } = await import('@codemirror/legacy-modes/mode/dockerfile');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(dockerFile);
    }
    case 'makefile': {
      // Legacy modes ship a CMake mode (`cmake`) but no Make grammar; the
      // shell highlighter is the closest approximation for recipe lines.
      const { shell } = await import('@codemirror/legacy-modes/mode/shell');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(shell);
    }
    case 'properties': {
      const { properties } = await import('@codemirror/legacy-modes/mode/properties');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(properties);
    }
    case 'lua': {
      const { lua } = await import('@codemirror/legacy-modes/mode/lua');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(lua);
    }
    case 'perl': {
      const { perl } = await import('@codemirror/legacy-modes/mode/perl');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(perl);
    }
    case 'r': {
      const { r } = await import('@codemirror/legacy-modes/mode/r');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(r);
    }
    case 'nginx': {
      const { nginx } = await import('@codemirror/legacy-modes/mode/nginx');
      const { StreamLanguage } = await import('@codemirror/language');
      return StreamLanguage.define(nginx);
    }
    default:
      return null;
  }
}

/** Lazy-loads the CodeMirror language extension for a given language name, with caching. */
export async function loadLanguageExtension(langName: string): Promise<Extension | null> {
  const cached = languageCache.get(langName);
  if (cached) {
    return cached;
  }

  const ext = await loadLanguageExtensionUncached(langName);
  if (ext) {
    languageCache.set(langName, ext);
  }
  return ext;
}

/** Clear the language extension cache. Exposed for testing. */
export function clearLanguageCache(): void {
  languageCache.clear();
}
