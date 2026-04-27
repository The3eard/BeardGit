/**
 * ref-colors.ts — Deterministic color hashing for Git ref badges.
 *
 * Provides a stable hash → color index mapping used by commit graph rendering
 * and commit detail views to assign consistent colors to branch/tag names.
 */

/**
 * DJB2-style string hash that returns a non-negative integer.
 *
 * The same input always produces the same output, making it safe to use
 * for visual color assignment across re-renders.
 */
export function hashString(s: string): number {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) - hash + s.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

/**
 * Maps a ref name to a color array index in the range [0, colorCount).
 *
 * @param name       The ref label (already stripped of `refs/heads/` etc.)
 * @param colorCount Number of available colors
 */
export function refColorIndex(name: string, colorCount: number): number {
  return hashString(name) % colorCount;
}
