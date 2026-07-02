/**
 * Clamp a popup menu's top-left corner so the menu stays fully inside the
 * viewport. When the menu would overflow the right/bottom edge it flips to
 * open leftward/upward from the cursor; if it is still too large (bigger than
 * the viewport) it clamps to the near edge as a fallback. A small `margin`
 * keeps the menu off the very edge of the window.
 */
export function clampMenuPosition(
  x: number,
  y: number,
  width: number,
  height: number,
  opts: { viewportWidth: number; viewportHeight: number; margin?: number },
): { left: number; top: number } {
  const margin = opts.margin ?? 8;
  const vw = opts.viewportWidth;
  const vh = opts.viewportHeight;

  let left = x;
  if (left + width > vw - margin) left = x - width; // flip leftward
  left = Math.max(margin, Math.min(left, vw - width - margin));

  let top = y;
  if (top + height > vh - margin) top = y - height; // flip upward
  top = Math.max(margin, Math.min(top, vh - height - margin));

  return { left, top };
}
