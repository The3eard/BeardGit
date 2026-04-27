/**
 * Race a promise against a fixed timeout.
 *
 * On timeout the returned promise rejects with {@link TimeoutError};
 * the underlying promise is **not cancelled** (Tauri's `invoke` has
 * no cancel semantics at this layer) but its eventual settlement is
 * ignored by any awaiter of the wrapper.
 *
 * Used by forge detail fetches so a slow `gh api .../files
 * --paginate` — e.g. the ~3.4k-file PR that triggered the original
 * infinite-spinner bug — can't strand the UI in a never-settling
 * loading state. Pair this with a toast + retry affordance driven
 * by `ForgeDetailShell`.
 */

/** Error thrown when a {@link withTimeout} race hits the timer. */
export class TimeoutError extends Error {
  constructor(ms: number) {
    super(`timed out after ${ms}ms`);
    this.name = "TimeoutError";
  }
}

/**
 * Race `promise` against a `ms`-millisecond timer.
 *
 * Resolves with the promise's value when it settles first, rejects
 * with the promise's reason when it rejects first, and rejects with
 * a fresh {@link TimeoutError} if the timer wins.
 *
 * The timer is always cleared on settle to avoid leaking handles in
 * the happy path.
 */
export function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  let handle: ReturnType<typeof setTimeout> | undefined;
  const timer = new Promise<never>((_, reject) => {
    handle = setTimeout(() => reject(new TimeoutError(ms)), ms);
  });
  return Promise.race([promise, timer]).finally(() => {
    if (handle !== undefined) clearTimeout(handle);
  });
}
