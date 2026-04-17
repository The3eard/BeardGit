/**
 * TypeScript types matching Rust structs exactly.
 *
 * These interfaces are serialized/deserialized across the Tauri IPC boundary.
 * Field names use snake_case to match Rust's serde output. Do not rename
 * fields without updating the corresponding Rust struct.
 */

export interface RepoInfo {
  path: string;
  head_branch: string | null;
  head_oid: string | null;
  branch_count: number;
}

export interface LayoutNode {
  oid: string;
  lane: number;
  row: number;
  refs: string[];
  summary: string;
  author: string;
  email: string;
  timestamp: number;
  is_merge: boolean;
  is_root: boolean;
  segment_group: number;
}

export interface LaneSegment {
  lane: number;
  start_row: number;
  end_row: number;
  color_index: number;
  recycled: boolean;
  sync_state: "Synced" | "LocalOnly" | "RemoteOnly" | "Unknown";
  group_id: number;
}

export interface MergeCurve {
  from_lane: number;
  from_row: number;
  to_lane: number;
  to_row: number;
  color_index: number;
  group_id: number;
}

export interface GraphViewport {
  nodes: LayoutNode[];
  lane_segments: LaneSegment[];
  merge_curves: MergeCurve[];
  total_count: number;
  offset: number;
  visible_lane_count: number;
  total_lane_count: number;
  head_lane: number | null;
}

export interface GraphThemeRefBadge {
  branch: string;
  remote: string;
  tag: string;
  head: string;
}

export interface GraphTheme {
  background: string;
  currentLine: string;
  selection: string;
  foreground: string;
  comment: string;
  red: string;
  orange: string;
  yellow: string;
  green: string;
  cyan: string;
  purple: string;
  pink: string;
  laneColors: string[];
  headLaneTint: string;
  dimOpacity: number;
  selectionHighlight: string;
  nodeRadius: number;
  mergeRadius: number;
  refBadge: GraphThemeRefBadge;
  textPrimary: string;
  textSecondary: string;
  textSha: string;
}

export interface CommitInfo {
  oid: string;
  summary: string;
  body: string;
  author: string;
  email: string;
  timestamp: number;
  parents: string[];
  refs: string[];
}

export interface BranchInfo {
  name: string;
  is_head: boolean;
  is_remote: boolean;
  oid: string;
}

export interface FileStatus {
  path: string;
  status: string;
  is_staged: boolean;
}

export interface FileDiff {
  path: string;
  old_path: string | null;
  status: string;
  hunks: DiffHunkInfo[];
  additions: number;
  deletions: number;
}

export interface DiffHunkInfo {
  header: string;
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLineInfo[];
}

export interface DiffLineInfo {
  origin: string;
  content: string;
  old_lineno: number | null;
  new_lineno: number | null;
}

export interface CommitFileChange {
  path: string;
  status: string;
}

export interface ProviderUser {
  id: number;
  username: string;
  display_name: string;
  email: string | null;
  avatar_url: string | null;
  profile_url: string;
}

export interface ConnectedProvider {
  kind: "gitlab" | "github";
  instance_url: string;
  user: ProviderUser;
  project_name: string | null;
}

export interface ProviderStatusResponse {
  providers: ConnectedProvider[];
  active_index: number | null;
}

export interface CiRun {
  id: number;
  display_id: number;
  status: string;
  ref_name: string;
  sha: string;
  source: string | null;
  name: string | null;
  created_at: string | null;
  updated_at: string | null;
  web_url: string;
}

export interface CiRunDetail {
  run: CiRun;
  duration: number | null;
  finished_at: string | null;
  stages: CiStage[];
}

export interface CiJobStep {
  number: number;
  name: string;
  status: string;
  duration: number | null;
}

export interface CiJob {
  id: number;
  name: string;
  stage: string | null;
  status: string;
  duration: number | null;
  started_at: string | null;
  finished_at: string | null;
  web_url: string;
  allow_failure: boolean | null;
  steps: CiJobStep[] | null;
}

export interface CiStage {
  name: string;
  jobs: CiJob[];
}

// ---------------------------------------------------------------------------
// CI/CD control (Phase 8.4)
// ---------------------------------------------------------------------------

export type WorkflowState = "active" | "disabled";

export interface Workflow {
  id: string;
  name: string;
  path: string;
  state: WorkflowState;
}

export interface TriggerWorkflowInput {
  workflow_id: string;
  git_ref: string;
  inputs: Record<string, string>;
}

export interface TriggerResult {
  run_id: string;
  url: string;
}

export type ProviderKind = "gitlab" | "github";

// ---------------------------------------------------------------------------
// Background tasks
// ---------------------------------------------------------------------------

export type TaskId = number;

export type TaskStatus =
  | { state: "queued" }
  | { state: "running" }
  | { state: "completed" }
  | { state: "failed"; error: string }
  | { state: "cancelled" };

export interface TaskInfo {
  id: TaskId;
  label: string;
  status: TaskStatus;
  cancellable: boolean;
  elapsed_secs: number | null;
  command: string;
  started_at_ms: number | null;
  exit_code: number | null;
}

export interface TaskOutputLine {
  stream: "stdout" | "stderr";
  text: string;
}

export interface TaskOutputEvent {
  task_id: TaskId;
  line: TaskOutputLine;
}

// ---------------------------------------------------------------------------
// Multi-project tabs
// ---------------------------------------------------------------------------

export interface ProjectInfo {
  path: string;
  name: string;
  head_branch: string | null;
  change_count: number;
}

export interface RecentRepo {
  path: string;
  name: string;
}

export interface RemoteInfo {
  name: string;
  url: string | null;
}

/** Starship-style git status counters for the title bar. */
export interface StatusSummary {
  ahead: number;
  behind: number;
  staged: number;
  unstaged: number;
  untracked: number;
  conflicted: number;
  stash_count: number;
}

/** Per-project cached git state for instant UI on switch. */
export interface ProjectSnapshot {
  path: string;
  head_branch: string | null;
  ahead: number;
  behind: number;
  staged: number;
  unstaged: number;
  untracked: number;
  conflicted: number;
  stash_count: number;
  change_count: number;
}

export interface StashEntry {
  index: number;
  message: string;
  branch: string;
  timestamp: number;
  oid: string;
}

export interface TagInfo {
  name: string;
  object_oid: string;
  commit_oid: string;
  annotated: boolean;
  message: string;
  tagger_name: string;
  tagger_email: string;
  date: string;
}

export interface CommitStats {
  files_changed: number;
  insertions: number;
  deletions: number;
}

export type ConflictStateValue =
  | "none"
  | "merging"
  | "rebasing"
  | "cherry_picking"
  | "reverting";

export interface ConflictStatus {
  state: ConflictStateValue;
  conflicted_files: string[];
  can_continue: boolean;
}

/** The three versions of a conflicted file (ours, theirs, base). */
export interface ConflictFileContents {
  ours: string;
  theirs: string;
  base: string;
}

// ---------------------------------------------------------------------------
// Worktrees
// ---------------------------------------------------------------------------

export interface WorktreeInfo {
  /** Absolute path to the worktree directory. */
  path: string;
  /** Branch checked out in this worktree, or null for detached HEAD. */
  branch: string | null;
  /** HEAD commit OID for this worktree. */
  head_oid: string;
  /** True when this is the main (primary) worktree. */
  is_main: boolean;
  /** True when the worktree is locked and cannot be removed without --force. */
  is_locked: boolean;
}

/** WorktreeInfo enriched with AI provider data when the worktree was created by an AI tool. */
export interface EnrichedWorktree extends WorktreeInfo {
  ai_provider: AiProviderKind | null;
  ai_status: "active" | "clean" | "orphaned" | null;
  ai_session_id: string | null;
}

// ---------------------------------------------------------------------------
// Clean
// ---------------------------------------------------------------------------

/** An untracked item that would be removed by git clean. */
export interface CleanItem {
  path: string;
  is_directory: boolean;
  is_ignored: boolean;
}

// Git config
// ---------------------------------------------------------------------------

/** Scope of a git configuration entry. Matches Rust `ConfigScope`. */
export type ConfigScope = "local" | "global" | "system";

/** A single git configuration entry. Matches Rust `ConfigEntry`. */
export interface ConfigEntry {
  key: string;
  value: string;
  scope: ConfigScope;
}

// ── Theme types ──────────────────────────────────────────────────────

export interface ThemeMeta {
  id: string;
  name: string;
  mode: string;
  complementary: string | null;
}

export interface ThemeBaseColors {
  background: string;
  foreground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  bright_black: string;
  bright_red: string;
  bright_green: string;
  bright_yellow: string;
  bright_blue: string;
  bright_magenta: string;
  bright_cyan: string;
  bright_white: string;
}

export interface DerivedColors {
  bg_primary: string;
  bg_secondary: string;
  bg_toolbar: string;
  text_primary: string;
  text_secondary: string;
  accent_blue: string;
  accent_green: string;
  accent_orange: string;
  accent_purple: string;
  accent_red: string;
  border: string;
  selection: string;
}

export interface ThemeGraphData {
  lane_colors: string[];
  background: string;
  foreground: string;
  text_primary: string;
  text_secondary: string;
  text_sha: string;
  selection: string;
  head_lane_tint: string;
  selection_highlight: string;
  dim_opacity: number;
  node_radius: number;
  merge_radius: number;
  ref_branch: string;
  ref_remote: string;
  ref_tag: string;
  ref_head: string;
}

export interface ThemeEditorData {
  background: string;
  foreground: string;
  cursor: string;
  selection: string;
  line_highlight: string;
  gutter_bg: string;
  gutter_fg: string;
  added_bg: string;
  removed_bg: string;
  added_text: string;
  removed_text: string;
  syntax_keyword: string | null;
  syntax_string: string | null;
  syntax_comment: string | null;
  syntax_function: string | null;
  syntax_type: string | null;
  syntax_number: string | null;
  syntax_operator: string | null;
  syntax_property: string | null;
}

export interface ThemeData {
  meta: ThemeMeta;
  colors: ThemeBaseColors;
  derived: DerivedColors;
  graph: ThemeGraphData;
  editor: ThemeEditorData | null;
}

// ---------------------------------------------------------------------------
// Blame & file history
// ---------------------------------------------------------------------------

/** A single line of blame output with commit attribution. */
export interface BlameLine {
  line_num: number;
  content: string;
  oid: string;
  author: string;
  email: string;
  timestamp: number;
  summary: string;
}

/** An entry in a file's commit history with diff stats. */
export interface FileHistoryEntry {
  oid: string;
  message: string;
  author: string;
  date: string;
  additions: number;
  deletions: number;
  old_path: string | null;
}

// ---------------------------------------------------------------------------
// Interactive rebase
// ---------------------------------------------------------------------------

/** A commit in the rebase todo list. */
export interface RebaseCommit {
  oid: string;
  message: string;
  author: string;
  date: string;
}

/** An action for a commit in the interactive rebase. */
export interface RebaseAction {
  oid: string;
  action: string;
}

// ---------------------------------------------------------------------------
// Hunk-level staging
// ---------------------------------------------------------------------------

/** Persisted graph column setting — matches Rust `GraphColumnConfig`. */
export interface GraphColumnConfig {
  id: string;
  width: number;
  visible: boolean;
}

// ---------------------------------------------------------------------------
// Reflog
// ---------------------------------------------------------------------------

/** A single entry from the HEAD reflog. */
export interface ReflogEntry {
  oid: string;
  prev_oid: string;
  action: string;
  summary: string;
  author: string;
  email: string;
  timestamp: number;
}

// Submodules
// ---------------------------------------------------------------------------

/** Status of a submodule relative to the superproject. */
export type SubmoduleStatus = "uninitialized" | "clean" | "outdated" | "dirty";

/** Information about a single submodule. */
export interface SubmoduleInfo {
  name: string;
  path: string;
  url: string;
  oid: string | null;
  registered_oid: string;
  status: SubmoduleStatus;
}

/** Describes which hunks/lines the user selected for staging/unstaging. */
export interface HunkSelection {
  /** Index into the FileDiff.hunks array. */
  hunk_index: number;
  /** If null, the entire hunk is selected. Otherwise inclusive 0-based line ranges within the hunk. */
  line_ranges: [number, number][] | null;
}

// ---------------------------------------------------------------------------
// Patch management
// ---------------------------------------------------------------------------

/** Per-file diff statistics from a patch. */
export interface PatchStat {
  path: string;
  insertions: number;
  deletions: number;
}

/** Preview result for a patch file before applying. */
export interface PatchPreview {
  applies_cleanly: boolean;
  stats: PatchStat[];
  total_files: number;
  total_insertions: number;
  total_deletions: number;
}

// ---------------------------------------------------------------------------
// Merge Requests / Pull Requests
// ---------------------------------------------------------------------------

export type MrPrState = "open" | "closed" | "merged";

export type ReviewStatus = "pending" | "approved" | "changes_requested" | "commented";

export type MergeStrategy = "merge" | "squash" | "rebase";

export interface MrPr {
  number: number;
  title: string;
  state: MrPrState;
  author: string;
  source_branch: string;
  target_branch: string;
  url: string;
  draft: boolean;
  labels: string[];
  reviewers: string[];
  created_at: string;
  updated_at: string;
  additions: number | null;
  deletions: number | null;
  changed_files: number | null;
}

export interface MrPrDetail {
  summary: MrPr;
  body: string;
  comments: MrPrComment[];
  review_status: ReviewStatus;
  mergeable: boolean | null;
}

export interface MrPrComment {
  id: number;
  author: string;
  body: string;
  created_at: string;
  path: string | null;
  line: number | null;
  is_review: boolean;
  /** GitLab-only: whether the discussion is marked resolvable. `null` on GitHub. */
  resolvable: boolean | null;
  /** GitLab-only: whether the discussion is currently resolved. `null` on GitHub. */
  resolved: boolean | null;
  /** GitLab-only: discussion ID used by resolve/unresolve calls. `null` on GitHub. */
  discussion_id: string | null;
}

export interface MrPrDiffFile {
  path: string;
  old_path: string | null;
  status: string;
  additions: number;
  deletions: number;
  patch: string | null;
}

/** A repository label for use with the label picker. */
export interface Label {
  name: string;
  color: string | null;
  description: string | null;
}

// ---------------------------------------------------------------------------
// Issues (Phase 8.3)
// ---------------------------------------------------------------------------

/** Open/closed lifecycle state for an issue. */
export type IssueState = "open" | "closed";

/** Open/closed lifecycle state for a milestone. */
export type MilestoneState = "open" | "closed";

/** A milestone. `id` is the provider-specific numeric identifier. */
export interface Milestone {
  id: number;
  title: string;
  state: MilestoneState;
  /** ISO-8601 due date — `null` if no due date set. */
  due_on: string | null;
}

/** Issue summary (list view). */
export interface Issue {
  number: number;
  title: string;
  state: IssueState;
  author: string;
  labels: Label[];
  assignees: string[];
  milestone: Milestone | null;
  comments_count: number;
  created_at: string;
  updated_at: string;
  url: string;
}

/** Full issue detail with body + comments. */
export interface IssueDetail {
  summary: Issue;
  body: string;
  /** Reuses the existing MrPrComment shape — structurally identical. */
  comments: MrPrComment[];
}

/** Filter for [`listIssues`]. */
export interface IssueFilter {
  state?: IssueState;
  author?: string;
  assignee?: string;
  label?: string;
  milestone?: number;
  text?: string;
}

/** Result of checking out a MR/PR branch locally. */
export interface CheckoutResult {
  branch_name: string;
  is_fork: boolean;
  remote_added: string | null;
}

// ── CLI Auth ─────────────────────────────────────────────────────────

/** Authentication status for a CLI tool (gh or glab). */
export interface CliAuthStatus {
  tool: string;
  installed: boolean;
  authenticated: boolean;
  username: string | null;
  error: string | null;
}

// ── Tabs ─────────────────────────────────────────────────────────────

/** Metadata for a terminal tab. */
export interface TerminalTabInfo {
  sessionId: number;
  title: string;
  cwd: string;
  /** When set, the tab was launched by an AI provider (shows brand icon). */
  provider?: AiProviderKind;
}

/** A segment linked to a project in a composite tab. */
export type LinkedSegment =
  | { type: "terminal"; info: TerminalTabInfo }
  | { type: "worktree"; path: string; branch: string };

/** Discriminated union for all tab types. */
export type Tab =
  | { kind: "project"; project: ProjectInfo }
  | { kind: "terminal"; terminal: TerminalTabInfo }
  | { kind: "composite"; project: ProjectInfo; segments: LinkedSegment[]; activeSegmentIndex: number };

// ─── AI Provider Types ───

export type AiProviderKind = "claude_code" | "codex" | "open_code";

export interface AvailableAiProvider {
  kind: AiProviderKind;
  binary_path: string;
  version: string | null;
}

export interface RepoAiStatus {
  kind: AiProviderKind;
  has_config: boolean;
  session_count: number;
  worktree_count: number;
}

export interface AiSession {
  id: string;
  provider: AiProviderKind;
  cwd: string;
  started_at: number | null;
  kind: "interactive" | "headless";
  is_active: boolean;
}

export interface AiWorktree {
  path: string;
  branch: string;
  provider: AiProviderKind;
  session_id: string | null;
  status: "active" | "clean" | "orphaned";
}

export interface AiConfigFile {
  path: string;
  kind: "settings" | "instructions" | "agent" | "skill";
  scope: "user" | "project" | "local";
}

// ---------------------------------------------------------------------------
// Bisect
// ---------------------------------------------------------------------------

/** Current state of a git bisect session. */
export interface BisectState {
  active: boolean;
  current_commit: string | null;
  steps_remaining: number | null;
  good_commits: string[];
  bad_commits: string[];
}

// ---------------------------------------------------------------------------
// Debug / Logging
// ---------------------------------------------------------------------------

/** Debug information for error reports. */
export interface DebugInfo {
  app_version: string;
  os: string;
  arch: string;
  git_version: string | null;
  log_path: string;
}
