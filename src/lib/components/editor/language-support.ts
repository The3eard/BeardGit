import type { Extension } from '@codemirror/state';

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
};

/** Returns a language name string for the given file path, or null if unknown. */
export function getLanguageExtensionName(path: string): string | null {
  const ext = path.split('.').pop()?.toLowerCase();
  if (!ext) return null;
  return EXTENSION_MAP[ext] ?? null;
}

/** Lazy-loads the CodeMirror language extension for a given language name. */
export async function loadLanguageExtension(langName: string): Promise<Extension | null> {
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
    default:
      return null;
  }
}
