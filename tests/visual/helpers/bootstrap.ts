/**
 * Bootstrap mocks — one-call setup for tests that just want a working
 * shell to take screenshots against.
 *
 * The SvelteKit `+page.svelte`'s `onMount` fires a fixed sequence of
 * commands (theme, sidebar layout, editor prefs, projects, locale,
 * AI providers, …). Without responses the app crashes early and the
 * sidebar never renders, so visual tests can't even click into a view.
 *
 * `installBootstrapMocks(page, opts)` registers responses for every
 * command the bootstrap touches — by default with empty repos / no
 * active project. Pass `activeProject` + per-view fixtures via `extra`
 * to populate a specific view.
 */

import type { Page } from "@playwright/test";

import {
  makeConflictStatus,
  makeEditorPreferences,
  makeProjectSnapshot,
  makeRepoInfo,
  makeSidebarNavLayout,
  makeThemeData,
  makeThemeMetaList,
} from "../../../src/test/fixtures";
import type {
  ProjectInfo,
  ProviderKind,
  ProviderStatusResponse,
  RecentRepo,
} from "../../../src/lib/types";

import { installMockIPC, type IpcResponses } from "./mock-ipc";
import type { ThemeMode } from "./themes";

export interface BootstrapOpts {
  /**
   * Theme mode returned by `resolve_startup_theme`. Default `"dark"`.
   * The matching `mode-{dark|light}` baselines must be regenerated if
   * the canned colours change.
   */
  mode?: ThemeMode;
  /**
   * If set, simulates this project being the single open tab. Drives
   * `restore_projects`, `get_active_project_index`, `get_open_projects`.
   */
  activeProject?: ProjectInfo;
  /** Recent-repos list for the welcome screen. Default `[]`. */
  recentRepos?: RecentRepo[];
  /**
   * Forge provider to advertise as connected. The sidebar gates
   * `pipelines`, `issues`, `merge-requests`, `releases`, and
   * `repo-config` behind a connected provider — pass a kind here
   * to make those nav items render. Default `"github"`.
   */
  forge?: ProviderKind | "none";
  /**
   * Extra responses to merge on top of the defaults. Use this to
   * register per-view fixtures (graph, branches, MRs, …) without
   * having to remember which command each view calls.
   */
  extra?: IpcResponses;
}

function makeProviderStatus(forge: ProviderKind | "none"): ProviderStatusResponse {
  if (forge === "none") return { providers: [], active_index: null };
  const isGitHub = forge === "github";
  return {
    providers: [
      {
        kind: forge,
        instance_url: isGitHub ? "https://github.com" : "https://gitlab.com",
        user: {
          id: 1,
          username: "adolfofuentes",
          display_name: "Adolfo Fuentes",
          email: null,
          avatar_url: null,
          profile_url: isGitHub
            ? "https://github.com/adolfofuentes"
            : "https://gitlab.com/adolfofuentes",
        },
        project_name: "sample",
      },
    ],
    active_index: 0,
  };
}

/**
 * Default bootstrap response set. Every command the app's onMount
 * touches has at least an empty/no-op response so the bundle finishes
 * its init pass without errors.
 */
function bootstrapResponses(opts: BootstrapOpts): IpcResponses {
  const theme = makeThemeData(opts.mode ?? "dark");
  const projects = opts.activeProject ? [opts.activeProject] : [];
  const activeIndex = opts.activeProject ? 0 : null;
  const repoInfo = opts.activeProject
    ? makeRepoInfo({
        path: opts.activeProject.path,
        head_branch: opts.activeProject.head_branch,
      })
    : makeRepoInfo();

  return {
    // Theme
    resolve_startup_theme: theme,
    get_theme: theme,
    list_themes: makeThemeMetaList(),
    get_theme_auto: false,
    set_theme: undefined,
    set_theme_auto: undefined,

    // UI scale
    get_ui_scale: 100,
    set_ui_scale: undefined,

    // Sidebar
    get_sidebar_collapsed: false,
    set_sidebar_collapsed: undefined,
    get_sidebar_nav_layout: makeSidebarNavLayout(),
    set_sidebar_nav_layout: undefined,

    // Editor prefs
    get_editor_preferences: makeEditorPreferences(),
    set_editor_preferences: undefined,

    // Locale
    get_locale: "en-US",
    set_locale_config: undefined,

    // Projects / tabs — `switch_project` is the workhorse the active-tab
    // activation calls; it must return a fully-formed RepoInfo or the
    // bootstrap rejects in `activateProjectTab`.
    restore_projects: projects,
    get_open_projects: projects,
    get_active_project_index: activeIndex,
    get_recent_repos: opts.recentRepos ?? [],
    switch_project: repoInfo,
    open_project: opts.activeProject ?? null,
    open_repo: repoInfo,
    get_project_snapshot: null,
    detect_project: undefined,

    // AI — empty / disabled defaults so onMount paths short-circuit
    detect_ai_providers: [],
    get_preferred_ai_provider: null,
    list_ai_sessions: [],
    list_ai_background_runs: [],
    get_ai_background_settings: {
      auto_resume: false,
      max_concurrent: 1,
    },

    // Forge / CI — `get_provider_status` controls which sidebar items
    // render (pipelines / issues / mr-pr / releases / repo-config).
    get_provider_status: makeProviderStatus(opts.forge ?? "github"),
    get_cli_auth_status: [],
    try_auto_connect: undefined,

    // ── Activation-path commands ─────────────────────────────────────
    // These all run inside `activateProjectTab` (projects.ts:213). If
    // any of them rejects or returns the wrong shape, the surrounding
    // try/finally completes synchronously — but a few callsites read
    // `.length` on the return value, so undefined throws before
    // `isLoading.set(false)` reads consistently. Returning empty
    // collections / sane defaults keeps the bootstrap deterministic.
    get_status_summary: { ahead: 0, behind: 0, staged: 0, unstaged: 0, untracked: 0, conflicted: 0, stash_count: 0 },
    get_branches: [],
    get_file_statuses: [],
    get_diff_workdir: [],
    get_diff_index: [],
    get_user_identities: [],
    get_conflict_status: makeConflictStatus(),
    get_tasks: [],
    compute_project_snapshot: opts.activeProject
      ? makeProjectSnapshot({ path: opts.activeProject.path })
      : makeProjectSnapshot(),
    save_project_snapshot: undefined,
    register_watcher: undefined,

    // Diff settings
    get_diff_show_whitespace: false,
    get_diff_line_wrapping: false,
    set_diff_show_whitespace: undefined,
    set_diff_line_wrapping: undefined,

    // AI surface — alternate command names found at runtime
    ai_refresh_detection: undefined,
    ai_get_preferred_provider: null,
    ai_list_background_runs: [],
    ai_get_providers: [],

    // Window plugin (set_title from titlebar updates)
    "plugin:window|set_title": undefined,

    // Tauri event listeners — the SDK pipes `listen()` calls through
    // here and unwraps the resolved value as the event id. Returning
    // undefined is fine: the unlisten closure simply captures undefined
    // as the id and the bootstrap doesn't depend on it.
    "plugin:event|listen": undefined,

    ...opts.extra,
  };
}

/**
 * Install all bootstrap mocks in one call. Combines `installMockIPC`
 * with a baseline response set so onMount completes cleanly.
 */
export async function installBootstrapMocks(
  page: Page,
  opts: BootstrapOpts = {},
): Promise<void> {
  await installMockIPC(page, bootstrapResponses(opts));
}

/**
 * Wait for `activateProjectTab` to finish. While `isLoading` is true,
 * the main panel renders the "Opening repository..." spinner instead of
 * any view — taking a screenshot then would just baseline the spinner.
 *
 * `.welcome-screen .loading-text` is the specific element bound to the
 * `$isLoading` branch in `+page.svelte` (line ~1065); waiting for it to
 * detach is the cleanest signal that the project has finished
 * activating.
 */
export async function waitForAppReady(page: Page): Promise<void> {
  await page
    .locator(".welcome-screen .loading-text")
    .waitFor({ state: "detached", timeout: 10_000 });
}
