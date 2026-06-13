<!--
  Skeleton.svelte — shimmer placeholder rows for loading lists.

  Replaces the centered spinner while a list panel fetches its first
  page: the shape of the incoming content is sketched immediately, so
  the panel feels populated instead of blocked. Widths follow a fixed
  per-row pattern (no randomness) so the placeholder is stable across
  renders and screenshots.

  The shimmer is a translated gradient; `prefers-reduced-motion`
  collapses it via the global animation kill-switch in app.css.

  Usage:
    <Skeleton rows={8} />
-->
<script lang="ts">
  interface Props {
    /** Number of placeholder rows (list) / paragraph bars (detail). Default 8. */
    rows?: number;
    /**
     * Shape of the placeholder. `list` (default) sketches two-line
     * rows with a leading dot; `detail` sketches a detail pane —
     * heading + meta line + paragraph bars.
     */
    variant?: "list" | "detail";
  }

  const { rows = 8, variant = "list" }: Props = $props();

  /* Deterministic "organic" width pattern (% of the row) for the
     title / meta bars, cycled by row index. */
  const TITLE_WIDTHS = [62, 48, 71, 55, 66, 43, 58, 50];
  const META_WIDTHS = [34, 26, 38, 22, 30, 36, 24, 28];
  const PARA_WIDTHS = [92, 84, 96, 70, 88, 78, 94, 58];
</script>

{#if variant === "detail"}
  <div
    class="bg-skeleton bg-skeleton--detail"
    role="status"
    aria-live="polite"
    data-testid="skeleton"
  >
    <span class="bg-skeleton__bar bg-skeleton__bar--heading" aria-hidden="true"></span>
    <span
      class="bg-skeleton__bar bg-skeleton__bar--meta bg-skeleton__bar--standalone"
      style:width="38%"
      aria-hidden="true"
    ></span>
    <span class="bg-skeleton__gap" aria-hidden="true"></span>
    {#each Array.from({ length: rows }) as _, i (i)}
      <span
        class="bg-skeleton__bar bg-skeleton__bar--standalone"
        style:width="{PARA_WIDTHS[i % PARA_WIDTHS.length]}%"
        aria-hidden="true"
      ></span>
    {/each}
  </div>
{:else}
  <div class="bg-skeleton" role="status" aria-live="polite" data-testid="skeleton">
    {#each Array.from({ length: rows }) as _, i (i)}
      <div class="bg-skeleton__row" aria-hidden="true">
        <span class="bg-skeleton__dot"></span>
        <span class="bg-skeleton__lines">
          <span
            class="bg-skeleton__bar"
            style:width="{TITLE_WIDTHS[i % TITLE_WIDTHS.length]}%"
          ></span>
          <span
            class="bg-skeleton__bar bg-skeleton__bar--meta"
            style:width="{META_WIDTHS[i % META_WIDTHS.length]}%"
          ></span>
        </span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .bg-skeleton {
    display: flex;
    flex-direction: column;
  }

  .bg-skeleton--detail {
    gap: 10px;
    padding: 20px 16px;
  }

  .bg-skeleton__bar--heading {
    width: 46%;
    height: 14px;
    margin-bottom: 2px;
  }

  .bg-skeleton__bar--standalone {
    display: block;
  }

  .bg-skeleton__gap {
    height: 10px;
  }

  .bg-skeleton__row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .bg-skeleton__dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    flex-shrink: 0;
    background: color-mix(in srgb, var(--text-primary) 8%, transparent);
  }

  .bg-skeleton__lines {
    display: flex;
    flex-direction: column;
    gap: 7px;
    flex: 1;
    min-width: 0;
  }

  .bg-skeleton__bar {
    height: 9px;
    border-radius: 4px;
    background:
      linear-gradient(
        100deg,
        transparent 30%,
        var(--overlay-hover) 50%,
        transparent 70%
      )
      color-mix(in srgb, var(--text-primary) 8%, transparent);
    background-size: 220% 100%;
    animation: bg-skeleton-shimmer 1.4s ease-in-out infinite;
  }

  .bg-skeleton__bar--meta {
    height: 7px;
    opacity: 0.7;
  }

  @keyframes bg-skeleton-shimmer {
    from {
      background-position: 120% 0;
    }
    to {
      background-position: -120% 0;
    }
  }
</style>
