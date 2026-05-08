import { getLocale } from "$lib/paraglide/runtime";
import * as m from "$lib/paraglide/messages";

// Per-locale formatter caches. `Intl.*` constructors are non-trivial to
// create — building one per call shows up under flame graphs in long lists
// (graph rows, MR list). Cache lives for the lifetime of the module; locale
// changes trigger a full `window.location.reload()` so the cache is purged
// implicitly with the module reset.
const rtfCache = new Map<string, Intl.RelativeTimeFormat>();
const dateCache = new Map<string, Intl.DateTimeFormat>();
const dateTimeCache = new Map<string, Intl.DateTimeFormat>();

function rtf(): Intl.RelativeTimeFormat {
  const locale = getLocale();
  let f = rtfCache.get(locale);
  if (!f) {
    f = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
    rtfCache.set(locale, f);
  }
  return f;
}

function dateFormatter(): Intl.DateTimeFormat {
  const locale = getLocale();
  let f = dateCache.get(locale);
  if (!f) {
    f = new Intl.DateTimeFormat(locale, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
    dateCache.set(locale, f);
  }
  return f;
}

function dateTimeFormatter(): Intl.DateTimeFormat {
  const locale = getLocale();
  let f = dateTimeCache.get(locale);
  if (!f) {
    f = new Intl.DateTimeFormat(locale, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
    dateTimeCache.set(locale, f);
  }
  return f;
}

/**
 * Format a Unix timestamp (seconds) as a relative time string in the
 * user's active locale ("5 minutes ago", "hace 5 minutos", "yesterday",
 * …). Sub-minute differences collapse to a localised "just now" string.
 */
export function formatRelativeTimeUnix(timestamp: number): string {
  const now = Date.now() / 1000;
  const diff = now - timestamp;
  if (diff < 60) return m.time_just_now();
  if (diff < 3600) return rtf().format(-Math.floor(diff / 60), "minute");
  if (diff < 86400) return rtf().format(-Math.floor(diff / 3600), "hour");
  if (diff < 604800) return rtf().format(-Math.floor(diff / 86400), "day");
  if (diff < 2592000) return rtf().format(-Math.floor(diff / 604800), "week");
  if (diff < 31536000) return rtf().format(-Math.floor(diff / 2592000), "month");
  return rtf().format(-Math.floor(diff / 31536000), "year");
}

/**
 * Format a Unix epoch timestamp (milliseconds) as a relative time string.
 *
 * Delegates to {@link formatRelativeTimeUnix} after converting ms to seconds.
 */
export function formatRelativeTimeMs(timestampMs: number): string {
  return formatRelativeTimeUnix(timestampMs / 1000);
}

/**
 * Format an ISO date string as a relative time string.
 */
export function formatRelativeTime(dateStr: string | null | undefined): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const timestamp = date.getTime() / 1000;
  return formatRelativeTimeUnix(timestamp);
}

/**
 * Format a Unix timestamp (seconds) as a short absolute date in the
 * user's active locale. Example: "Apr 7, 2026" (en-US) /
 * "7 abr 2026" (es-ES).
 */
export function formatDate(timestamp: number): string {
  return dateFormatter().format(new Date(timestamp * 1000));
}

/**
 * Format a Unix timestamp (seconds) as a date with time in the user's
 * active locale. Example: "Apr 7, 2026, 10:30 AM" /
 * "7 abr 2026, 10:30".
 */
export function formatDateTime(timestamp: number): string {
  return dateTimeFormatter().format(new Date(timestamp * 1000));
}
