//! Tauri application entry point for BeardGit.
//!
//! This crate (`beardgit-lib`) wires together the Tauri builder with the
//! [`app_core`] command handlers and application state. On mobile targets the
//! `run` function is also used as the mobile entry point via the
//! `tauri::mobile_entry_point` attribute.

/// Build and run the Tauri application.
///
/// Registers all Tauri plugins (file opener, native dialog) and every
/// `#[tauri::command]` defined in [`app_core::commands`], then blocks until
/// the application window is closed.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_core::state::AppState::new())
        .setup(|app| {
            use tauri::Manager as _;
            let sink = std::sync::Arc::new(app_core::event_sink::TauriEventSink::new(
                app.handle().clone(),
            ));
            let task_manager = std::sync::Arc::new(task_runner::TaskManager::new(sink));
            app.manage(task_manager);

            // Listen for OS theme changes and re-emit resolved theme when auto is enabled.
            let main_window = app.get_webview_window("main");
            if let Some(window) = main_window {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::ThemeChanged(theme) = event {
                        let handle = app_handle.clone();
                        let os_dark = matches!(theme, tauri::Theme::Dark);
                        tauri::async_runtime::spawn(async move {
                            let config_dir = dirs::config_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."))
                                .join("beardgit");
                            let config_path = config_dir.join("settings.json");
                            let config = storage::AppConfig::load(&config_path).unwrap_or_default();

                            if config.theme_auto {
                                let new_id = app_core::commands::resolve_theme_for_mode(
                                    &config.theme,
                                    os_dark,
                                );
                                let themes_dir = config_dir.join("themes");
                                let resolved = storage::theme::resolve_theme(&new_id, &themes_dir);
                                use tauri::Emitter as _;
                                let _ = handle.emit("theme-changed", &resolved);
                            }
                        });
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_core::commands::open_repo,
            app_core::commands::get_graph_viewport,
            app_core::commands::get_commit_row,
            app_core::commands::search_commits,
            app_core::commands::get_commit_detail,
            app_core::commands::get_status_summary,
            app_core::commands::get_commit_files,
            app_core::commands::get_diff_between_commits,
            app_core::commands::get_commit_file_diff,
            app_core::commands::get_file_at_commit,
            app_core::commands::get_file_workdir,
            app_core::commands::get_file_index,
            app_core::commands::get_branches,
            app_core::commands::get_branch_commits,
            app_core::commands::get_file_statuses,
            app_core::commands::stage_files,
            app_core::commands::unstage_files,
            app_core::commands::stage_all,
            app_core::commands::unstage_all,
            app_core::commands::create_commit,
            app_core::commands::create_branch,
            app_core::commands::delete_branch,
            app_core::commands::checkout_branch,
            app_core::commands::get_diff_workdir,
            app_core::commands::get_diff_index,
            app_core::commands::merge_branch,
            app_core::commands::cherry_pick,
            app_core::commands::stash_push,
            app_core::commands::stash_pop,
            app_core::commands::stash_list,
            app_core::commands::stash_apply,
            app_core::commands::stash_apply_file,
            app_core::commands::stash_drop,
            app_core::commands::stash_entries,
            app_core::commands::stash_show_parsed,
            app_core::commands::open_project,
            app_core::commands::close_project,
            app_core::commands::switch_project,
            app_core::commands::get_open_projects,
            app_core::commands::get_active_project_index,
            app_core::commands::restore_projects,
            app_core::commands::get_recent_repos,
            app_core::commands::get_remotes,
            app_core::commands::fetch_remote,
            app_core::commands::pull_remote,
            app_core::commands::push_remote,
            app_core::commands::rename_remote,
            app_core::commands::remove_remote,
            app_core::commands::list_tags,
            app_core::commands::create_tag,
            app_core::commands::delete_tag,
            app_core::commands::push_tag,
            app_core::commands::get_commit_stats,
            app_core::commands::list_tags_paginated,
            app_core::commands::search_tags,
            app_core::commands::get_conflict_status,
            app_core::commands::abort_operation,
            app_core::commands::continue_operation,
            app_core::commands::connect_provider,
            app_core::commands::disconnect_provider,
            app_core::commands::try_auto_connect,
            app_core::commands::get_provider_status,
            app_core::commands::list_ci_runs,
            app_core::commands::get_ci_run_detail,
            app_core::commands::get_job_log,
            app_core::commands::preprocess_job_log,
            app_core::commands::detect_project,
            app_core::commands::get_locale,
            app_core::commands::set_locale,
            app_core::commands::get_user_identities,
            app_core::commands::list_themes,
            app_core::commands::get_theme,
            app_core::commands::set_theme,
            app_core::commands::get_theme_auto,
            app_core::commands::set_theme_auto,
            app_core::commands::resolve_startup_theme,
            app_core::task_commands::get_tasks,
            app_core::task_commands::get_task_output,
            app_core::task_commands::cancel_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
