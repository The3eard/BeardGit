/**
 * TS mirrors of the Rust structs exchanged by the `requests_*` IPC commands.
 *
 * Keep in sync with:
 *   - crates/app-core/src/commands/requests.rs (TreeNode, EnvSummary,
 *     RunRequestArgs, RunResult, CopyAsArgs, HistoryRow, DiffPayload)
 *   - crates/requests-runner/src/types.rs (ParsedRequest)
 *   - crates/requests-runner/src/env.rs (EnvFile)
 *
 * Note on casing: Tauri converts *top-level* command args from camelCase to
 * snake_case, but nested struct payloads (`RunRequestArgs`, `CopyAsArgs`) are
 * deserialized by serde with their Rust field names — so those interfaces use
 * snake_case verbatim.
 */

/** A single node in a requests tree (`requestsListProject` / `requestsListGlobal`). */
export interface RequestTreeNode {
  /** `"folder"`, `"file"`, or `"block"` (reserved). */
  kind: "folder" | "file" | "block";
  /** Display name (file name for project entries, item/collection name for global). */
  name: string;
  /** Source-relative path (see the Rust `TreeNode` docs for the encoding). */
  rel_path: string;
  /** HTTP verb of the first parsed block, when this is a `.http` file. */
  method: string | null;
  children: RequestTreeNode[];
}

/** Parsed `.http` request (`requestsLoad` / `requestsPasteCurl`). Mirrors `ParsedRequest`. */
export interface ParsedRequest {
  name: string | null;
  /** `"GET"`..`"OPTIONS"` (UPPERCASE) or null when the block had no method. */
  method: string | null;
  url: string;
  headers: [string, string][];
  body: string | null;
}

/** An environment file (`requestsLoadEnv`, and the `env` arg to `requestsSaveEnv`). Mirrors `EnvFile`. */
export interface RequestEnvFile {
  $schema: string;
  vars: Record<string, string>;
  secrets: string[];
}

/** Env summary row for the switcher (`requestsGetEnvs`). Mirrors `EnvSummary`. */
export interface RequestEnvSummary {
  name: string;
  vars_count: number;
  /** Secret *names* only — values live encrypted in the credential store. */
  secrets: string[];
}

/** Arguments to `requestsRun`. snake_case (nested payload — see the module note). */
export interface RunRequestArgs {
  source_kind: string;
  source_path: string;
  project_path: string | null;
  env_name: string | null;
  overrides: Record<string, string>;
  ticket_id: string;
}

/** Result of `requestsRun`. Mirrors `RunResult`. */
export interface RunResult {
  history_id: number;
  status: number;
  headers: [string, string][];
  /** Base64-encoded response body (raw bytes, may be truncated). */
  body_base64: string;
  truncated: boolean;
  duration_ms: number;
}

/** Arguments to `requestsCopyAs`. snake_case (nested payload — see the module note). */
export interface CopyAsArgs {
  source_kind: string;
  source_path: string;
  project_path: string | null;
  env_name: string | null;
  /** One of `"curl"`, `"fetch"`, `"httpie"`, `"wget"`. */
  target: string;
  overrides: Record<string, string>;
}

/** A single History-panel row (`requestsHistory`). Mirrors `HistoryRow`. */
export interface RequestHistoryRow {
  id: number;
  status: number | null;
  duration_ms: number;
  executed_at: number;
  env_name: string | null;
  truncated: boolean;
}

/** Response-diff payload (`requestsDiffResponses`). Mirrors `DiffPayload`. */
export interface RequestDiffPayload {
  left: string;
  right: string;
  content_type_hint: string | null;
}
