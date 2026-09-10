#!/usr/bin/env node
/**
 * Builds the public site under `docs/` from the sources in `docs/_pages`,
 * `docs/_partials` and `docs/_i18n`.
 *
 * The site is four pages in two languages, so the shared chrome — head,
 * nav, footer, asset URLs — exists once here rather than eight times in
 * eight hand-edited files. Two things follow that are worth knowing before
 * editing anything under `docs/`:
 *
 *   - `docs/**` HTML is OUTPUT. Edit `docs/_pages/*.html` and re-run.
 *     `npm run check:site` (in the gate) fails when they disagree.
 *   - Every asset URL carries `?v=ASSET_VERSION`. GitHub Pages serves
 *     assets with `Cache-Control: max-age=600` and the screenshots keep
 *     their filenames across redesigns, so a reused name without a bumped
 *     token shows visitors the previous release's UI. Bump ASSET_VERSION
 *     whenever an asset's *content* changes under the same filename.
 *
 * Translation model: prose lives twice, structure lives once. A page marks
 * per-language prose with `<!--@en-->…<!--/@en-->` blocks, and short
 * strings (attributes, labels, captions) come from `docs/_i18n/<lang>.json`
 * via `{{t:key}}`. Nothing that carries layout is duplicated, so the two
 * languages cannot drift structurally.
 *
 *     node scripts/build-site.mjs           # write docs/
 *     node scripts/build-site.mjs --check   # verify docs/ matches sources
 */

import { readFile, writeFile, mkdir, readdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DOCS = join(ROOT, "docs");
const SITE_ORIGIN = "https://the3eard.github.io/BeardGit";

/** Bump when any asset changes content under an unchanged filename. */
const ASSET_VERSION = "20260910a";

/** `lastmod` for the sitemap. A literal, not `new Date()`, so `--check`
 *  compares equal on any day other than the one the site was built. */
const SITE_UPDATED = "2026-09-10";

/**
 * The release this build of the site ships with.
 *
 * The site goes live at the same time as the release that closes the
 * changelog's `[Unreleased]` block, so publishing that block under the
 * word "unreleased" would be wrong the moment anyone could read it. It is
 * published under this version instead. Bump it when you cut a release —
 * the build fails if this version is already a released heading in
 * CHANGELOG.md while an `[Unreleased]` block is still open.
 */
const UPCOMING_VERSION = "26.9.3";

const LANGS = ["en", "es"];
const DEFAULT_LANG = "en";

/** Page order is nav order. `out` is relative to docs/ for the default language. */
const PAGES = [
  { slug: "home", source: "index.html", out: "index.html", nav: "home" },
  { slug: "features", source: "features.html", out: "features/index.html", nav: "features" },
  { slug: "guide", source: "guide.html", out: "guide/index.html", nav: "guide" },
  { slug: "changelog", source: "changelog.html", out: "changelog/index.html", nav: "changelog" },
];

const args = process.argv.slice(2);
const CHECK_ONLY = args.includes("--check");

/* ------------------------------------------------------------------ *
 * Template helpers
 * ------------------------------------------------------------------ */

/**
 * Replaces `{{KEY}}` with vars[KEY]. Unknown keys are left alone so a typo
 * shows up in the output instead of silently emptying a section.
 *
 * Runs until it stops changing, because the partials it injects carry
 * placeholders of their own (the nav holds the language switch href).
 */
function fill(template, vars) {
  const once = (html) =>
    html.replace(/\{\{([A-Z_]+)\}\}/g, (match, key) =>
      Object.prototype.hasOwnProperty.call(vars, key) ? vars[key] : match,
    );
  let html = template;
  for (let pass = 0; pass < 4; pass += 1) {
    const next = once(html);
    if (next === html) return html;
    html = next;
  }
  return html;
}

/** Resolves `{{t:key}}` against the language dictionary. */
function translate(html, dict, lang) {
  return html.replace(/\{\{t:([a-z0-9_.-]+)\}\}/g, (_match, key) => {
    if (!Object.prototype.hasOwnProperty.call(dict, key)) {
      throw new Error(`missing ${lang} translation for "${key}" (docs/_i18n/${lang}.json)`);
    }
    return dict[key];
  });
}

/** Keeps the blocks for `lang`, drops every other language's. */
function pickLanguageBlocks(html, lang) {
  for (const other of LANGS) {
    if (other === lang) continue;
    const drop = new RegExp(`<!--@${other}-->[\\s\\S]*?<!--/@${other}-->\\s*`, "g");
    html = html.replace(drop, "");
  }
  const keep = new RegExp(`<!--@${lang}-->([\\s\\S]*?)<!--/@${lang}-->`, "g");
  return html.replace(keep, "$1");
}

/**
 * Expands `{{fig:slug}}` into a themed, three-format figure.
 *
 * Each screenshot exists as light/dark × avif/webp/png, and app.js swaps
 * the pair when the page's theme changes — twelve URLs and two data
 * attributes per figure. Written by hand twenty times over, one of them
 * ends up pointing at the wrong mode's file, which only shows up as a
 * screenshot that stops following the toggle. Here it is one token.
 *
 * Modifiers: `|<class>` adds a class to the <figure>, `|eager` opts the
 * image out of lazy loading (use for the one above the fold).
 */
function expandFigures(html, available) {
  return html.replace(/\{\{fig:([a-z0-9-]+)((?:\|[a-z0-9-]+)*)\}\}/g, (_match, slug, mods) => {
    const modifiers = mods ? mods.slice(1).split("|") : [];
    const eager = modifiers.includes("eager");
    const classes = ["shot-figure", ...modifiers.filter((m) => m !== "eager")].join(" ");

    for (const mode of ["light", "dark"]) {
      for (const ext of ["avif", "webp", "png"]) {
        const name = `${slug}-${mode}.${ext}`;
        if (!available.has(name)) {
          throw new Error(`{{fig:${slug}}} needs docs/assets/screenshots/${name}, which is missing`);
        }
      }
    }

    const url = (mode, ext) => `{{BASE}}assets/screenshots/${slug}-${mode}.${ext}?v={{V}}`;
    const source = (ext) =>
      [
        `          <source class="shot-src" type="image/${ext}"`,
        `                  data-srcset-light="${url("light", ext)}"`,
        `                  data-srcset-dark="${url("dark", ext)}"`,
        `                  srcset="${url("dark", ext)}" />`,
      ].join("\n");

    return [
      `      <figure class="${classes}">`,
      `        <div class="screenshot-slot" data-slot="${slug}">`,
      `          <picture>`,
      source("avif"),
      source("webp"),
      `            <img class="shot-img"`,
      `                 data-src-light="${url("light", "png")}"`,
      `                 data-src-dark="${url("dark", "png")}"`,
      `                 src="${url("dark", "png")}"`,
      `                 width="2880" height="1800"`,
      eager ? `                 fetchpriority="high" decoding="async"` : `                 loading="lazy" decoding="async"`,
      `                 alt="{{t:shot.${slug}.alt}}" />`,
      `          </picture>`,
      `        </div>`,
      `        <figcaption class="shot-caption mono">{{t:shot.${slug}.caption}}</figcaption>`,
      `      </figure>`,
    ].join("\n");
  });
}

/** Reads the `<!--meta {...} -->` header off a page source. */
async function readPageSource(name) {
  const raw = await readFile(join(DOCS, "_pages", name), "utf8");
  const match = raw.match(/^<!--meta\s*([\s\S]*?)-->\s*/);
  if (!match) throw new Error(`${name} has no <!--meta { … } --> header`);
  let meta;
  try {
    meta = JSON.parse(match[1]);
  } catch (error) {
    throw new Error(`${name} has an invalid meta header: ${error.message}`);
  }
  return { meta, body: raw.slice(match[0].length) };
}

/** Relative prefix from a built page back up to docs/. */
function basePrefix(outPath) {
  const depth = outPath.split("/").length - 1;
  return depth === 0 ? "" : "../".repeat(depth);
}

function outPathFor(page, lang) {
  return lang === DEFAULT_LANG ? page.out : `${lang}/${page.out}`;
}

/** Public URL of a page, with the trailing `index.html` dropped. */
function urlFor(page, lang) {
  const path = outPathFor(page, lang).replace(/index\.html$/, "");
  return `${SITE_ORIGIN}/${path}`;
}

/* ------------------------------------------------------------------ *
 * Changelog: CHANGELOG.md → the body of the changelog page
 * ------------------------------------------------------------------ */

function escapeHtml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** Markdown inline subset the changelog actually uses. */
function inlineMarkdown(text) {
  let html = escapeHtml(text);
  // Code spans first: their contents must not be read as emphasis.
  const codes = [];
  html = html.replace(/`([^`]+)`/g, (_match, code) => {
    codes.push(code);
    return `\u0000${codes.length - 1}\u0000`;
  });
  html = html
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[\s(])_([^_]+)_(?=[\s.,;:)]|$)/g, "$1<em>$2</em>")
    .replace(/(^|[\s(])\*([^*]+)\*(?=[\s.,;:)]|$)/g, "$1<em>$2</em>");
  return html.replace(/\u0000(\d+)\u0000/g, (_match, i) => `<code class="mono">${codes[Number(i)]}</code>`);
}

/**
 * Parses the release sections of CHANGELOG.md.
 *
 * Recognised shapes, and nothing else — a heading that does not match is
 * reported rather than dropped, so a changelog written in a new shape
 * fails the build instead of publishing a page with holes in it.
 */
function parseChangelog(markdown) {
  const releases = [];
  let release = null;
  let group = null;

  const lines = markdown.split("\n");
  for (const line of lines) {
    const heading2 = line.match(/^##\s+(.*)$/);
    if (heading2) {
      const text = heading2[1].trim();
      const versioned = text.match(/^\[([^\]]+)\]\s*(.*)$/);
      if (!versioned) throw new Error(`unparsed changelog heading: "${text}"`);
      // Four shapes are in the file, from three different years of habit:
      // "[v] — title — date", "[v] — title", "[v] - date — title" and
      // "[v] - date". Pull the ISO date out wherever it sits and treat
      // whatever is left as the title.
      let rest = versioned[2];
      const dateMatch = rest.match(/\d{4}-\d{2}-\d{2}/);
      if (dateMatch) rest = rest.replace(dateMatch[0], "");
      release = {
        version: versioned[1],
        title: rest.replace(/^[\s—–-]+|[\s—–-]+$/g, "").trim(),
        date: dateMatch ? dateMatch[0] : "",
        groups: [],
      };
      releases.push(release);
      group = null;
      continue;
    }

    const heading3 = line.match(/^###\s+(.*)$/);
    if (heading3) {
      if (!release) throw new Error(`changelog group "${heading3[1]}" outside a release`);
      group = { name: heading3[1].trim(), items: [] };
      release.groups.push(group);
      continue;
    }

    const item = line.match(/^-\s+(.*)$/);
    if (item) {
      if (!group) continue;
      group.items.push(item[1]);
      continue;
    }

    // A wrapped continuation line belongs to the item above it.
    if (line.trim() && group && group.items.length > 0 && !line.startsWith("#")) {
      group.items[group.items.length - 1] += ` ${line.trim()}`;
    }
  }

  const kept = releases.filter((r) => r.groups.length > 0);

  // Re-label the open block as the release that will carry it.
  const open = kept.find((release) => /unreleased/i.test(release.version));
  if (open) {
    const clash = kept.find((release) => release.version === UPCOMING_VERSION);
    if (clash) {
      throw new Error(
        `CHANGELOG.md still has an [Unreleased] block while ${UPCOMING_VERSION} is already released — bump UPCOMING_VERSION in scripts/build-site.mjs`,
      );
    }
    open.version = UPCOMING_VERSION;
    open.upcoming = true;
  }

  return kept;
}

const GROUP_TONE = { Added: "added", Fixed: "fixed", Changed: "changed", Removed: "removed" };

function renderChangelog(releases, dict) {
  // The two most recent releases open by default; older ones collapse, so
  // the page opens on what changed rather than on two years of history.
  return releases
    .map((release, index) => {
      const open = index < 2 ? " open" : "";  // newest two expanded
      const groups = release.groups
        .map((group) => {
          const tone = GROUP_TONE[group.name] || "other";
          const items = group.items
            .map((item) => `            <li>${inlineMarkdown(item)}</li>`)
            .join("\n");
          const label = dict[`changelog.group.${tone}`] || group.name;
          return [
            `        <div class="release-group">`,
            `          <h3 class="release-group-title ${tone}">${escapeHtml(label)}</h3>`,
            `          <ul class="release-list">`,
            items,
            `          </ul>`,
            `        </div>`,
          ].join("\n");
        })
        .join("\n");

      const version = release.version;
      // The newest block has no date in the changelog until it is tagged;
      // saying "this release" beats printing a date that isn't decided.
      const date = release.date
        ? `<span class="release-date mono">${escapeHtml(release.date)}</span>`
        : `<span class="release-date mono">${escapeHtml(dict["changelog.this_release"])}</span>`;
      // The title belongs in the summary: it is what tells you whether a
      // collapsed release is the one you're looking for.
      const title = release.title
        ? `\n          <p class="release-title">${inlineMarkdown(release.title)}</p>`
        : "";

      return [
        `      <details class="release"${open}${release.upcoming ? ' data-current="true"' : ""}>`,
        `        <summary>`,
        `          <div class="release-head">`,
        `            <span class="release-version mono">${escapeHtml(version)}</span>`,
        `            ${date}`,
        `          </div>${title}`,
        `        </summary>`,
        groups,
        `      </details>`,
      ].join("\n");
    })
    .join("\n");
}

/* ------------------------------------------------------------------ *
 * Build
 * ------------------------------------------------------------------ */

async function loadPartials() {
  const dir = join(DOCS, "_partials");
  const names = (await readdir(dir)).filter((name) => name.endsWith(".html"));
  const partials = {};
  for (const name of names) {
    partials[name.replace(/\.html$/, "")] = await readFile(join(dir, name), "utf8");
  }
  return partials;
}

async function loadDictionaries() {
  const dicts = {};
  for (const lang of LANGS) {
    dicts[lang] = JSON.parse(await readFile(join(DOCS, "_i18n", `${lang}.json`), "utf8"));
  }
  return dicts;
}

function navHtml(partial, page, lang, langRoot) {
  // The current entry is marked in the built HTML rather than by script, so
  // it is right in the markup that search engines and no-JS visitors see.
  let html = partial.replace(
    new RegExp(`(<a\\b[^>]*\\bdata-nav="${page.nav}"[^>]*?)>`),
    '$1 aria-current="page" class="active">',
  );
  return html.replace(/\{\{L\}\}/g, langRoot);
}

async function build() {
  const partials = await loadPartials();
  const dicts = await loadDictionaries();
  const changelogSource = await readFile(join(ROOT, "CHANGELOG.md"), "utf8");
  const releases = parseChangelog(changelogSource);
  const screenshots = new Set(await readdir(join(DOCS, "assets", "screenshots")));

  const outputs = new Map();

  for (const page of PAGES) {
    const { meta, body } = await readPageSource(page.source);

    for (const lang of LANGS) {
      const dict = dicts[lang];
      const out = outPathFor(page, lang);
      const base = basePrefix(out);
      const langRoot = lang === DEFAULT_LANG ? base : `${base}${lang}/`;
      const other = LANGS.find((l) => l !== lang);

      let content = expandFigures(pickLanguageBlocks(body, lang), screenshots);
      if (page.slug === "changelog") {
        content = content.replace("{{CHANGELOG}}", renderChangelog(releases, dict));
      }

      const headExtra = meta.head ? pickLanguageBlocks(partials[meta.head] || "", lang) : "";

      let html = fill(partials.layout, {
        LANG: lang,
        TITLE: escapeHtml(meta.title[lang]),
        DESCRIPTION: escapeHtml(meta.description[lang]),
        CANONICAL: urlFor(page, lang),
        ALT_EN: urlFor(page, "en"),
        ALT_ES: urlFor(page, "es"),
        OG_IMAGE: `${SITE_ORIGIN}/assets/${meta.og || "og-image.png"}?v=${ASSET_VERSION}`,
        HEAD_EXTRA: headExtra,
        NAV: navHtml(partials.nav, page, lang, langRoot),
        FOOTER: partials.footer,
        CONTENT: content,
        // Relative so the built site also works opened from disk and from
        // a preview server that isn't mounted at /BeardGit/.
        LANG_SWITCH_HREF: base + outPathFor(page, other).replace(/index\.html$/, ""),
        LANG_SWITCH_LABEL: other.toUpperCase(),
        LANG_SWITCH_TITLE: dict["nav.lang_switch"],
        V: ASSET_VERSION,
        PAGE: page.slug,
      });

      // Language-scoped links and dictionary strings resolve after the
      // layout is assembled so partials can use them too.
      html = html.replace(/\{\{L\}\}/g, langRoot).replace(/\{\{BASE\}\}/g, base);
      html = translate(html, dict, lang);
      html = html.replace(/\{\{V\}\}/g, ASSET_VERSION);

      // `{{raw:x}}` publishes a literal `{{x}}` — the pages document
      // BeardGit's own `{{var}}` request syntax, which would otherwise
      // read as a placeholder here.
      const leftover = html.replace(/\{\{raw:[^}]*\}\}/g, "").match(/\{\{[^}]+\}\}/);
      if (leftover) throw new Error(`${out}: unresolved placeholder ${leftover[0]}`);
      html = html.replace(/\{\{raw:([^}]*)\}\}/g, "{{$1}}");

      outputs.set(out, html);
    }
  }

  outputs.set("sitemap.xml", renderSitemap());
  return outputs;
}

/** Both languages of every page, each pointing at the other as its
 *  alternate — the pairing is what stops the two from competing. */
function renderSitemap() {
  const entries = PAGES.flatMap((page) =>
    LANGS.map((lang) => {
      const alternates = LANGS.map(
        (other) =>
          `    <xhtml:link rel="alternate" hreflang="${other}" href="${urlFor(page, other)}" />`,
      ).join("\n");
      return [
        "  <url>",
        `    <loc>${urlFor(page, lang)}</loc>`,
        `    <lastmod>${SITE_UPDATED}</lastmod>`,
        `    <priority>${page.slug === "home" ? "1.0" : "0.8"}</priority>`,
        alternates,
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${urlFor(page, DEFAULT_LANG)}" />`,
        "  </url>",
      ].join("\n");
    }),
  );

  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"',
    '        xmlns:xhtml="http://www.w3.org/1999/xhtml">',
    ...entries,
    "</urlset>",
    "",
  ].join("\n");
}

async function main() {
  const outputs = await build();

  if (CHECK_ONLY) {
    const stale = [];
    for (const [path, html] of outputs) {
      let current = null;
      try {
        current = await readFile(join(DOCS, path), "utf8");
      } catch {
        stale.push(`${path} (missing)`);
        continue;
      }
      if (current !== html) stale.push(path);
    }
    if (stale.length > 0) {
      console.error("✗ docs/ is out of date with its sources:\n");
      for (const path of stale) console.error(`    docs/${path}`);
      console.error("\n  Run `npm run build:site` and commit the result.\n");
      process.exit(1);
    }
    console.log(`✓ site up to date — ${outputs.size} pages match docs/_pages`);
    return;
  }

  for (const [path, html] of outputs) {
    const full = join(DOCS, path);
    await mkdir(dirname(full), { recursive: true });
    await writeFile(full, html, "utf8");
    console.log(`  wrote docs/${path}`);
  }
  console.log(`\n✓ ${outputs.size} pages · assets pinned at ?v=${ASSET_VERSION}`);
}

main().catch((error) => {
  console.error(`✗ ${error.message}`);
  process.exit(1);
});
