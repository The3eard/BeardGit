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
}

export interface CiStage {
  name: string;
  jobs: CiJob[];
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

// ── Theme types ──────────────────────────────────────────────────────

export interface ThemeMeta {
  id: string;
  name: string;
  mode: string;
}

export interface ThemeColors {
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

export interface ThemeData {
  meta: ThemeMeta;
  colors: ThemeColors;
  graph: ThemeGraphData;
}
