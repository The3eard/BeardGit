//! CI run listing, detail, and job log commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// Fetch a paginated list of CI runs for the detected project.
///
/// All filter parameters are forwarded to the provider. Filtering is performed
/// server-side only — there is no client-side filtering.
#[tauri::command]
pub async fn list_ci_runs(
    branch: Option<String>,
    source: Option<String>,
    status: Option<String>,
    per_page: Option<u32>,
    page: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<provider::CiRun>, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    let filters = provider::CiFilters {
        branch,
        status,
        source,
    };
    ci_provider
        .list_ci_runs(
            &project_ref,
            &filters,
            per_page.unwrap_or(20),
            page.unwrap_or(1),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Fetch full detail for a single CI run, including its stages and jobs.
#[tauri::command]
pub async fn get_ci_run_detail(
    run_id: u64,
    state: State<'_, AppState>,
) -> Result<provider::CiRunDetail, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .get_ci_run_detail(&project_ref, run_id)
        .await
        .map_err(|e| e.to_string())
}

/// Fetch the raw log output for a single CI job.
#[tauri::command]
pub async fn get_job_log(job_id: u64, state: State<'_, AppState>) -> Result<String, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .get_job_log(&project_ref, job_id)
        .await
        .map_err(|e| e.to_string())
}

/// Preprocess a raw CI job log, stripping provider-specific noise.
///
/// Delegates to [`provider::log_preprocessor::preprocess_ci_log`] which strips
/// timestamps, stream codes, section markers, and adds line numbers. ANSI
/// color/style codes are preserved for the frontend renderer.
#[tauri::command]
pub fn preprocess_job_log(raw_text: String, provider_kind: String) -> Result<String, String> {
    let kind = match provider_kind.as_str() {
        "gitlab" => provider::ProviderKind::GitLab,
        "github" => provider::ProviderKind::GitHub,
        _ => return Err(format!("Unknown provider kind: {}", provider_kind)),
    };
    Ok(provider::log_preprocessor::preprocess_ci_log(
        &raw_text, kind,
    ))
}

// ---------------------------------------------------------------------------
// CI/CD control commands (Phase 8.4)
// ---------------------------------------------------------------------------

/// Trigger a new CI run for the active provider.
///
/// For GitHub, `workflow_id` must be a workflow file name (e.g. `"ci.yml"`)
/// or numeric ID. For GitLab, `workflow_id` is ignored.
#[tauri::command]
pub async fn trigger_workflow(
    workflow_id: String,
    git_ref: String,
    inputs: std::collections::HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<provider::TriggerResult, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    let input = provider::TriggerWorkflowInput {
        workflow_id,
        git_ref,
        inputs,
    };
    ci_provider
        .trigger_workflow(&project_ref, &input)
        .await
        .map_err(|e| e.to_string())
}

/// Re-run all jobs in a previously completed run.
#[tauri::command]
pub async fn retry_ci_run(run_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .retry_run(&project_ref, &run_id)
        .await
        .map_err(|e| e.to_string())
}

/// Re-run only failed jobs of a completed run.
#[tauri::command]
pub async fn retry_ci_failed_jobs(
    run_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .retry_failed_jobs(&project_ref, &run_id)
        .await
        .map_err(|e| e.to_string())
}

/// Re-run a specific job.
#[tauri::command]
pub async fn retry_ci_job(job_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .retry_job(&project_ref, &job_id)
        .await
        .map_err(|e| e.to_string())
}

/// Cancel an in-progress run.
#[tauri::command]
pub async fn cancel_ci_run(run_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .cancel_run(&project_ref, &run_id)
        .await
        .map_err(|e| e.to_string())
}

/// List workflow definitions for the active project.
///
/// GitLab returns a single placeholder `Workflow`. GitHub returns all
/// workflow files under `.github/workflows/`.
#[tauri::command]
pub async fn list_ci_workflows(
    state: State<'_, AppState>,
) -> Result<Vec<provider::Workflow>, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .list_workflows(&project_ref)
        .await
        .map_err(|e| e.to_string())
}
