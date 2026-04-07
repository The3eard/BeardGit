/**
 * Format a Unix timestamp (seconds) as a relative time string.
 */
export function formatRelativeTimeUnix(timestamp: number): string {
  const now = Date.now() / 1000;
  const diff = now - timestamp;
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
  if (diff < 2592000) return `${Math.floor(diff / 604800)}w ago`;
  if (diff < 31536000) return `${Math.floor(diff / 2592000)}mo ago`;
  return `${Math.floor(diff / 31536000)}y ago`;
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
 * Format a Unix timestamp (seconds) as a short absolute date.
 * Example: "Apr 7, 2026"
 */
export function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

/**
 * Format a Unix timestamp (seconds) as a date with time.
 * Example: "Apr 7, 2026, 10:30 AM"
 */
export function formatDateTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
