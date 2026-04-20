/**
 * TypeScript mirrors of the Rust `RemoteRepoConfig*` shapes defined in
 * `crates/app-core/src/commands/repo_config.rs`.
 *
 * Names follow the Rust types verbatim (snake_case field names) because
 * Tauri's IPC layer serialises `#[derive(Serialize, Deserialize)]`
 * structs with the default `serde` layout — the frontend sees exactly
 * the Rust field names unless an explicit `#[serde(rename = ...)]` is
 * applied (none of these shapes use one).
 *
 * The tri-state [`PatchValue`] mirror matches the
 * `#[serde(tag = "kind", content = "value", rename_all = "snake_case")]`
 * representation of the Rust enum:
 *
 *   - `{ kind: "unchanged" }`
 *   - `{ kind: "clear" }`
 *   - `{ kind: "set", value: <T> }`
 *
 * Building a patch must emit that exact shape; otherwise the backend
 * collapses `Clear`/`Unchanged` to the same thing and silently drops
 * the "user explicitly cleared this field" signal.
 */

/** Visibility of a remote repository. Matches `Visibility` in Rust. */
export type Visibility = "public" | "private" | "internal";

/** Branch-protection rules surfaced by the Protection tab. */
export interface BranchProtection {
  require_pull_request: boolean;
  required_approvals: number;
  require_status_checks: boolean;
  status_check_contexts: string[];
  require_up_to_date: boolean;
  require_conversation_resolution: boolean;
  enforce_admins: boolean;
}

/** Repository label. Matches `Label` in `repo_config.rs`. */
export interface RepoConfigLabel {
  name: string;
  /** Hex color without the leading `#`, or `null` if unset. */
  color: string | null;
  description: string | null;
}

/** Full remote repo config loaded from the forge. */
export interface RemoteRepoConfig {
  description: string;
  homepage: string | null;
  topics: string[];
  visibility: Visibility;
  default_branch: string;
  issues_enabled: boolean;
  wiki_enabled: boolean;
  archived: boolean;
  branch_protection: BranchProtection | null;
  labels: RepoConfigLabel[];
}

/**
 * Tri-state patch value mirroring the Rust `PatchValue<T>` enum.
 *
 * Use {@link patchUnchanged}, {@link patchClear}, {@link patchSet} to
 * construct — never build the shape by hand so the `kind` string stays
 * in lock-step with the Rust serde representation.
 */
export type PatchValue<T> =
  | { kind: "unchanged" }
  | { kind: "clear" }
  | { kind: "set"; value: T };

/** The always-unchanged singleton. Safe to reuse. */
export const patchUnchanged = <T>(): PatchValue<T> => ({ kind: "unchanged" });
/** The always-clear singleton. Safe to reuse. */
export const patchClear = <T>(): PatchValue<T> => ({ kind: "clear" });
/** Construct a `Set` patch carrying the given value. */
export const patchSet = <T>(value: T): PatchValue<T> => ({
  kind: "set",
  value,
});

/**
 * Diff-driven patch describing the fields a user changed.
 *
 * `undefined` / omitted fields are left unchanged on the forge.
 * `homepage` uses a {@link PatchValue} tri-state so the UI can
 * distinguish "leave unchanged" from "explicitly clear".
 */
export interface RemoteRepoConfigPatch {
  description?: string;
  homepage: PatchValue<string>;
  topics_added: string[];
  topics_removed: string[];
  visibility?: Visibility;
  default_branch?: string;
  issues_enabled?: boolean;
  wiki_enabled?: boolean;
  archive?: boolean;
}

/** Structured result of applying a patch — per-field successes + failures. */
export interface FieldError {
  field: string;
  message: string;
}

/** Output of `apply_remote_repo_config`. */
export interface ApplyResult {
  fields_updated: string[];
  failures: FieldError[];
}

/**
 * Forge CLI probe outcome. Returned from the `probe_forge_cli_status`
 * Tauri command. Serde emits one of three tagged variants:
 *
 *   - `{ kind: "installed", authenticated: bool, account: string | null }`
 *   - `{ kind: "not_installed" }`
 *   - `{ kind: "unsupported_forge" }`
 */
export type ForgeCliStatus =
  | { kind: "installed"; authenticated: boolean; account: string | null }
  | { kind: "not_installed" }
  | { kind: "unsupported_forge" };
