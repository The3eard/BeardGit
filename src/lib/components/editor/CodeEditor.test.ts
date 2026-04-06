import { describe, it, expect } from 'vitest';
import { getLanguageExtensionName } from './language-support';
import { createCodemirrorTheme } from './codemirror-theme';

describe('getLanguageExtensionName', () => {
  it('returns correct language for known extensions', () => {
    expect(getLanguageExtensionName('src/main.ts')).toBe('typescript');
    expect(getLanguageExtensionName('lib.rs')).toBe('rust');
    expect(getLanguageExtensionName('app.py')).toBe('python');
    expect(getLanguageExtensionName('style.css')).toBe('css');
    expect(getLanguageExtensionName('index.html')).toBe('html');
    expect(getLanguageExtensionName('data.json')).toBe('json');
    expect(getLanguageExtensionName('config.yaml')).toBe('yaml');
    expect(getLanguageExtensionName('config.yml')).toBe('yaml');
    expect(getLanguageExtensionName('README.md')).toBe('markdown');
    expect(getLanguageExtensionName('Main.java')).toBe('java');
    expect(getLanguageExtensionName('main.go')).toBe('go');
    expect(getLanguageExtensionName('main.cpp')).toBe('cpp');
    expect(getLanguageExtensionName('main.c')).toBe('cpp');
    expect(getLanguageExtensionName('main.h')).toBe('cpp');
    expect(getLanguageExtensionName('query.sql')).toBe('sql');
    expect(getLanguageExtensionName('layout.xml')).toBe('xml');
    expect(getLanguageExtensionName('script.sh')).toBe('shell');
    expect(getLanguageExtensionName('script.bash')).toBe('shell');
    expect(getLanguageExtensionName('.svelte')).toBe('html');
  });

  it('returns null for unknown extensions', () => {
    expect(getLanguageExtensionName('file.xyz')).toBeNull();
    expect(getLanguageExtensionName('Makefile')).toBeNull();
    expect(getLanguageExtensionName('')).toBeNull();
  });

  it('is case-insensitive for file extensions', () => {
    expect(getLanguageExtensionName('FILE.TS')).toBe('typescript');
    expect(getLanguageExtensionName('Main.RS')).toBe('rust');
    expect(getLanguageExtensionName('App.PY')).toBe('python');
  });

  it('handles deeply nested paths correctly', () => {
    expect(getLanguageExtensionName('a/b/c/d/e.rs')).toBe('rust');
    expect(getLanguageExtensionName('src/lib/components/editor/main.ts')).toBe('typescript');
  });

  it('uses the last segment after dots for extension detection', () => {
    // "some.config.js" → extension is "js" → typescript
    expect(getLanguageExtensionName('some.config.js')).toBe('typescript');
  });

  it('handles double extensions by using the final extension', () => {
    // "file.test.ts" → extension is "ts" → typescript
    expect(getLanguageExtensionName('file.test.ts')).toBe('typescript');
    // "component.spec.js" → extension is "js" → typescript
    expect(getLanguageExtensionName('component.spec.js')).toBe('typescript');
  });
});

describe('createCodemirrorTheme', () => {
  it('returns an Extension from editor theme data', () => {
    const editorData = {
      background: '#0d1117',
      foreground: '#e6edf3',
      cursor: '#58a6ff',
      selection: '#1f6feb44',
      line_highlight: '#161b2266',
      gutter_bg: '#0d1117',
      gutter_fg: '#8b949e',
      added_bg: 'rgba(63,185,80,0.15)',
      removed_bg: 'rgba(248,81,73,0.15)',
      added_text: '#3fb950',
      removed_text: '#f85149',
      syntax_keyword: '#ff7b72',
      syntax_string: '#a5d6ff',
      syntax_comment: '#8b949e',
      syntax_function: '#d2a8ff',
      syntax_type: '#79c0ff',
      syntax_number: '#79c0ff',
      syntax_operator: '#ff7b72',
      syntax_property: '#7ee787',
    };
    const ext = createCodemirrorTheme(editorData, true);
    expect(ext).toBeDefined();
  });

  it('creates a fallback theme when editor data is null', () => {
    const ext = createCodemirrorTheme(null, true);
    expect(ext).toBeDefined();
  });

  it('creates a light mode theme (isDark=false)', () => {
    const ext = createCodemirrorTheme(null, false);
    expect(ext).toBeDefined();
  });

  it('creates a theme with all syntax tokens provided', () => {
    const editorData = {
      background: '#ffffff',
      foreground: '#1f2328',
      cursor: '#0969da',
      selection: '#0969da33',
      line_highlight: '#f6f8fa',
      gutter_bg: '#f6f8fa',
      gutter_fg: '#6e7781',
      added_bg: 'rgba(63,185,80,0.15)',
      removed_bg: 'rgba(248,81,73,0.15)',
      added_text: '#116329',
      removed_text: '#82071e',
      syntax_keyword: '#cf222e',
      syntax_string: '#0a3069',
      syntax_comment: '#6e7781',
      syntax_function: '#8250df',
      syntax_type: '#0550ae',
      syntax_number: '#0550ae',
      syntax_operator: '#cf222e',
      syntax_property: '#116329',
    };
    const ext = createCodemirrorTheme(editorData, false);
    expect(ext).toBeDefined();
    expect(Array.isArray(ext)).toBe(true);
  });

  it('creates a theme with partial syntax tokens (some null-ish)', () => {
    // Only provide required base colors; syntax fields omitted (undefined)
    const partial = {
      background: '#0d1117',
      foreground: '#e6edf3',
      cursor: '#58a6ff',
      selection: '#1f6feb44',
      line_highlight: 'transparent',
      gutter_bg: '#0d1117',
      gutter_fg: '#8b949e',
      added_bg: null,
      removed_bg: null,
      added_text: null,
      removed_text: null,
      syntax_keyword: null,
      syntax_string: null,
      syntax_comment: null,
      syntax_function: null,
      syntax_type: null,
      syntax_number: null,
      syntax_operator: null,
      syntax_property: null,
    };
    // Cast to any to simulate a partial/incomplete theme payload arriving from IPC
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ext = createCodemirrorTheme(partial as any, true);
    expect(ext).toBeDefined();
  });
});
