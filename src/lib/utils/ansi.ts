/**
 * ANSI escape code parser utilities.
 *
 * Converts ANSI-escaped terminal output to HTML, handling SGR color/style codes,
 * CSI sequences, OSC sequences, and GitLab CI section markers.
 *
 * Supported SGR codes:
 * - Reset (0), bold (1), dim (2), italic (3), underline (4)
 * - Standard foreground (30–37), bright foreground (90–97)
 * - Standard background (40–47), bright background (100–107)
 * - 256-color foreground (38;5;N), 256-color background (48;5;N)
 * - True-color foreground (38;2;R;G;B), true-color background (48;2;R;G;B)
 */

/** Options for the ANSI-to-HTML converter. */
export interface AnsiParserOptions {
  /** Override foreground color map (keys are SGR codes 30–37, 90–97). */
  colorTheme?: Record<number, string>;
  /** Override background color map (keys are SGR codes 40–47, 100–107). */
  bgColorTheme?: Record<number, string>;
  /** When true, strip all ANSI codes and return plain text instead of HTML. */
  stripOnly?: boolean;
}

/** Default ANSI foreground color map. */
const DEFAULT_COLORS: Record<number, string> = {
  30: "#333", 31: "#f85149", 32: "#3fb950", 33: "#d29922",
  34: "#58a6ff", 35: "#bb80ff", 36: "#39c5cf", 37: "#cccccc",
  90: "#666", 91: "#ff7b72", 92: "#7ee787", 93: "#ffa657",
  94: "#79c0ff", 95: "#d2a8ff", 96: "#56d4dd", 97: "#ffffff",
};

/** Default ANSI background color map (SGR codes 40–47, 100–107). */
const DEFAULT_BG_COLORS: Record<number, string> = {
  40: "#333", 41: "#f85149", 42: "#3fb950", 43: "#d29922",
  44: "#58a6ff", 45: "#bb80ff", 46: "#39c5cf", 47: "#cccccc",
  100: "#666", 101: "#ff7b72", 102: "#7ee787", 103: "#ffa657",
  104: "#79c0ff", 105: "#d2a8ff", 106: "#56d4dd", 107: "#ffffff",
};

/**
 * Convert a 256-color palette index (0–255) to a hex color string.
 *
 * - 0–7: standard colors
 * - 8–15: bright colors
 * - 16–231: 6x6x6 color cube
 * - 232–255: grayscale ramp
 */
function ansi256ToHex(n: number): string {
  // Standard colors (0–7)
  const standard = [
    "#333", "#f85149", "#3fb950", "#d29922",
    "#58a6ff", "#bb80ff", "#39c5cf", "#cccccc",
  ];
  // Bright colors (8–15)
  const bright = [
    "#666", "#ff7b72", "#7ee787", "#ffa657",
    "#79c0ff", "#d2a8ff", "#56d4dd", "#ffffff",
  ];

  if (n < 8) return standard[n];
  if (n < 16) return bright[n - 8];

  if (n < 232) {
    // 6x6x6 color cube: index 16–231
    const idx = n - 16;
    const b = idx % 6;
    const g = Math.floor(idx / 6) % 6;
    const r = Math.floor(idx / 36);
    const toVal = (c: number) => (c === 0 ? 0 : 55 + c * 40);
    return `#${toVal(r).toString(16).padStart(2, "0")}${toVal(g).toString(16).padStart(2, "0")}${toVal(b).toString(16).padStart(2, "0")}`;
  }

  // Grayscale ramp: index 232–255 → 8, 18, 28, …, 238
  const gray = 8 + (n - 232) * 10;
  const hex = gray.toString(16).padStart(2, "0");
  return `#${hex}${hex}${hex}`;
}

/**
 * Strip all ANSI escape sequences from text and return plain text.
 * Also removes GitLab CI section markers and carriage returns.
 */
function stripAnsi(text: string): string {
  // Remove GitLab CI section marker lines
  const lines = text.split('\n').filter(line => !line.match(/^section_(start|end):/));
  let cleaned = lines.join('\n').replace(/\r/g, '');

  let result = '';
  let i = 0;

  while (i < cleaned.length) {
    if (cleaned[i] === '\x1b' && cleaned[i + 1] === '[') {
      // CSI sequence — skip to end command character
      let j = i + 2;
      while (j < cleaned.length && (cleaned[j] === ';' || (cleaned[j] >= '0' && cleaned[j] <= '9'))) {
        j++;
      }
      i = j + 1;
    } else if (cleaned[i] === '\x1b' && cleaned[i + 1] === ']') {
      // OSC sequence — skip until BEL or ST
      let j = i + 2;
      while (j < cleaned.length && cleaned[j] !== '\x07' && !(cleaned[j] === '\x1b' && cleaned[j + 1] === '\\')) {
        j++;
      }
      i = j + (cleaned[j] === '\x07' ? 1 : 2);
    } else if (cleaned[i] === '\x1b') {
      // Any other lone escape — skip escape and following character
      i += 2;
    } else {
      result += cleaned[i];
      i++;
    }
  }

  return result;
}

/**
 * Convert ANSI-escaped terminal output to HTML.
 *
 * Handles:
 * - GitLab CI section markers (lines starting with `section_start:` / `section_end:`)
 * - Carriage return stripping
 * - SGR codes: foreground (30–37, 90–97), background (40–47), bold (1), reset (0)
 * - CSI sequences: parses `\x1b[<params>m`, ignores other CSI commands
 * - OSC sequences: skips `\x1b]...` until BEL or ST
 * - Other escapes: skipped
 * - HTML entity escaping for `<`, `>`, and `&`
 *
 * @param text - Raw text with ANSI escape codes
 * @param options - Optional configuration (color theme overrides, stripOnly mode)
 * @returns HTML string with `<span>` elements for colored/styled regions,
 *          or plain text if `stripOnly` is true
 */
export function ansiToHtml(text: string, options?: AnsiParserOptions): string {
  if (options?.stripOnly) {
    return stripAnsi(text);
  }

  const colors = options?.colorTheme ?? DEFAULT_COLORS;
  const bgColors = options?.bgColorTheme ?? DEFAULT_BG_COLORS;

  // Remove GitLab CI section marker lines
  const lines = text.split('\n').filter(line => !line.match(/^section_(start|end):/));
  let cleaned = lines.join('\n').replace(/\r/g, '');

  let result = '';
  let currentColor: string | null = null;
  let currentBg: string | null = null;
  let bold = false;
  let dim = false;
  let italic = false;
  let underline = false;
  let i = 0;

  const hasStyle = () => currentColor || currentBg || bold || dim || italic || underline;

  const openSpan = () => {
    const styles: string[] = [];
    if (currentColor) styles.push(`color:${currentColor}`);
    if (currentBg) styles.push(`background:${currentBg}`);
    if (bold) styles.push('font-weight:bold');
    if (dim) styles.push('opacity:0.6');
    if (italic) styles.push('font-style:italic');
    if (underline) styles.push('text-decoration:underline');
    result += `<span style="${styles.join(';')}">`;
  };

  const closeSpan = () => {
    if (hasStyle()) {
      result += '</span>';
    }
  };

  while (i < cleaned.length) {
    if (cleaned[i] === '\x1b' && cleaned[i + 1] === '[') {
      // Parse CSI sequence
      let j = i + 2;
      while (j < cleaned.length && (cleaned[j] === ';' || (cleaned[j] >= '0' && cleaned[j] <= '9'))) {
        j++;
      }
      const params = cleaned.slice(i + 2, j);
      const cmd = cleaned[j] || '';
      i = j + 1;

      if (cmd === 'm') {
        // SGR — color/style codes
        const codes = params ? params.split(';').map(Number) : [0];
        closeSpan();
        let ci = 0;
        while (ci < codes.length) {
          const code = codes[ci];
          if (code === 0) {
            currentColor = null;
            currentBg = null;
            bold = false;
            dim = false;
            italic = false;
            underline = false;
          } else if (code === 1) {
            bold = true;
          } else if (code === 2) {
            dim = true;
          } else if (code === 3) {
            italic = true;
          } else if (code === 4) {
            underline = true;
          } else if (code === 22) {
            bold = false;
            dim = false;
          } else if (code === 23) {
            italic = false;
          } else if (code === 24) {
            underline = false;
          } else if (code === 38 && codes[ci + 1] === 5 && ci + 2 < codes.length) {
            // 256-color foreground: 38;5;N
            currentColor = ansi256ToHex(codes[ci + 2]);
            ci += 2;
          } else if (code === 48 && codes[ci + 1] === 5 && ci + 2 < codes.length) {
            // 256-color background: 48;5;N
            currentBg = ansi256ToHex(codes[ci + 2]);
            ci += 2;
          } else if (code === 38 && codes[ci + 1] === 2 && ci + 4 < codes.length) {
            // True-color foreground: 38;2;R;G;B
            const r = codes[ci + 2], g = codes[ci + 3], b = codes[ci + 4];
            currentColor = `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
            ci += 4;
          } else if (code === 48 && codes[ci + 1] === 2 && ci + 4 < codes.length) {
            // True-color background: 48;2;R;G;B
            const r = codes[ci + 2], g = codes[ci + 3], b = codes[ci + 4];
            currentBg = `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
            ci += 4;
          } else if (code === 39) {
            currentColor = null;
          } else if (code === 49) {
            currentBg = null;
          } else if (colors[code] !== undefined) {
            currentColor = colors[code];
          } else if (bgColors[code] !== undefined) {
            currentBg = bgColors[code];
          }
          ci++;
        }
        if (hasStyle()) openSpan();
      }
      // All other CSI sequences (K, J, H, A, B, s, u, etc.) are stripped
    } else if (cleaned[i] === '\x1b' && cleaned[i + 1] === ']') {
      // OSC sequence — skip until BEL (\x07) or ST (\x1b\\)
      let j = i + 2;
      while (j < cleaned.length && cleaned[j] !== '\x07' && !(cleaned[j] === '\x1b' && cleaned[j + 1] === '\\')) {
        j++;
      }
      i = j + (cleaned[j] === '\x07' ? 1 : 2);
    } else if (cleaned[i] === '\x1b') {
      // Any other lone escape — skip the escape and the following character
      i += 2;
    } else {
      // Regular character — HTML-escape it
      const ch = cleaned[i];
      if (ch === '<') result += '&lt;';
      else if (ch === '>') result += '&gt;';
      else if (ch === '&') result += '&amp;';
      else result += ch;
      i++;
    }
  }

  closeSpan();
  return result;
}
