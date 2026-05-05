/**
 * Map a file basename to a Nerd Font Symbols Only glyph for icon-prefix
 * rendering in the file tree. Returns the generic "file" glyph for any
 * extension we haven't mapped explicitly so the caller never has to guard
 * against undefined.
 *
 * The codepoints below come from the Nerd Font Symbols Only set bundled at
 * `static/fonts/SymbolsNerdFontMono-Regular.ttf`; verified via the upstream
 * cheatsheet (https://www.nerdfonts.com/cheat-sheet) and grouped by
 * provenance (Devicons, Seti UI, FontAwesome, Material) for review.
 */

/** Map of basename → glyph (matches before extension lookup). */
const BASENAME_MAP: Record<string, string> = {
  ".gitignore": "\ue702", // dev-git
  ".gitattributes": "\ue702",
  ".gitmodules": "\ue702",
  ".dockerignore": "\uf308", // dev-docker
  "Dockerfile": "\uf308",
  "Cargo.toml": "\ue7a8", // dev-rust
  "Cargo.lock": "\ue7a8",
  "package.json": "\ue718", // dev-nodejs_small
  "package-lock.json": "\ue718",
  "pnpm-lock.yaml": "\ue718",
  "yarn.lock": "\ue718",
  "tsconfig.json": "\ue628", // typescript
  "svelte.config.js": "\ue697", // svelte
  "vite.config.js": "\ue74e",
  "vite.config.ts": "\ue628",
  "README": "\uf02d", // book
  "README.md": "\uf02d",
  "LICENSE": "\uf48a",
  "LICENSE.md": "\uf48a",
  "Makefile": "\ue779",
};

/** Map of lowercase extension → glyph. */
const EXT_MAP: Record<string, string> = {
  // TypeScript / JavaScript
  ts: "\ue628",
  tsx: "\ue7ba",
  js: "\ue74e",
  jsx: "\ue7ba",
  mjs: "\ue74e",
  cjs: "\ue74e",
  // Frontend frameworks / templates
  svelte: "\ue697",
  vue: "\ufd42",
  // Web
  html: "\ue60e",
  htm: "\ue60e",
  css: "\ue614",
  scss: "\ue603",
  sass: "\ue603",
  less: "\ue614",
  // Data
  json: "\ue60b",
  yaml: "\ue6a8",
  yml: "\ue6a8",
  toml: "\ue6b2",
  xml: "\ue619",
  // Markdown / docs
  md: "\ue73e",
  mdx: "\ue73e",
  txt: "\uf15c",
  rst: "\uf15c",
  // Languages
  rs: "\ue7a8",
  py: "\ue606",
  pyw: "\ue606",
  java: "\ue738",
  kt: "\ue634",
  go: "\ue626",
  rb: "\ue21e",
  php: "\ue608",
  cs: "\uf81a",
  swift: "\ue755",
  c: "\ue61e",
  h: "\ue61e",
  cpp: "\ue61d",
  cxx: "\ue61d",
  cc: "\ue61d",
  hpp: "\ue61d",
  hxx: "\ue61d",
  // Shell
  sh: "\ue795",
  bash: "\ue795",
  zsh: "\ue795",
  fish: "\ue795",
  ps1: "\uebc7",
  // Config / lock
  env: "\uf462",
  ini: "\uf013",
  conf: "\uf013",
  config: "\uf013",
  lock: "\uf023",
  // Database
  sql: "\ue706",
  db: "\uf1c0",
  // Images
  svg: "\uf1c5",
  png: "\uf1c5",
  jpg: "\uf1c5",
  jpeg: "\uf1c5",
  gif: "\uf1c5",
  webp: "\uf1c5",
  ico: "\uf1c5",
  // Audio / video
  mp3: "\uf1c7",
  mp4: "\uf1c8",
  mov: "\uf1c8",
  webm: "\uf1c8",
  // Archives
  zip: "\uf1c6",
  tar: "\uf1c6",
  gz: "\uf1c6",
  bz2: "\uf1c6",
  rar: "\uf1c6",
  "7z": "\uf1c6",
  // PDF
  pdf: "\uf1c1",
  // Misc
  log: "\uf18d",
};

/** Generic file glyph used as the fallback. */
export const GENERIC_FILE_GLYPH = "\uf15b";

/**
 * Return the Nerd Font glyph that best represents a file. Falls back to
 * the generic file glyph for unknown extensions so callers can render
 * the result unconditionally.
 */
export function fileGlyphFor(name: string): string {
  if (BASENAME_MAP[name]) return BASENAME_MAP[name];
  const dot = name.lastIndexOf(".");
  if (dot < 0 || dot === name.length - 1) return GENERIC_FILE_GLYPH;
  const ext = name.slice(dot + 1).toLowerCase();
  return EXT_MAP[ext] ?? GENERIC_FILE_GLYPH;
}
