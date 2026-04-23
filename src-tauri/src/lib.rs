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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(app_core::state::AppState::new())
        .setup(|app| {
            // Initialize structured file logging (best-effort — don't crash if it fails)
            storage::logging::init_logging().ok();

            use tauri::Manager as _;
            let sink = std::sync::Arc::new(app_core::event_sink::TauriEventSink::new(
                app.handle().clone(),
            ));
            let task_manager = std::sync::Arc::new(task_runner::TaskManager::new(sink.clone()));
            // Plug the Tauri snapshot emitter into the task manager so git
            // fetch/pull/push/clone lifecycle events stream to the unified
            // tasks drawer via the `task://update` event.
            task_manager.set_emitter(std::sync::Arc::new(
                app_core::task_events::TauriEmitter::new(app.handle().clone()),
            ));
            app.manage(task_manager.clone());

            // AI background coordinator: shares the task manager with the rest
            // of the app and emits `ai-background-*` events through a
            // Tauri-backed sink.
            let ai_sink = std::sync::Arc::new(
                app_core::event_sink::TauriAiBackgroundEventSink::new(app.handle().clone()),
            );
            let ai_coord =
                std::sync::Arc::new(app_core::ai_background::AiBackgroundCoordinator::new(
                    task_manager.clone(),
                    ai_sink,
                ));
            // Let the TaskEventSink route AI background lifecycle events
            // into the coordinator.
            sink.install_ai_background_coordinator(ai_coord.clone());
            // Stash the coordinator in AppState so commands can reach it.
            {
                let state: tauri::State<'_, app_core::state::AppState> = app.state();
                *state
                    .ai_background_coordinator
                    .lock()
                    .expect("coordinator mutex poisoned") = Some(ai_coord);
            }

            let terminal_sink = std::sync::Arc::new(
                app_core::terminal_sink::TauriTerminalSink::new(app.handle().clone()),
            );
            let terminal_manager =
                std::sync::Arc::new(terminal::TerminalManager::new(terminal_sink));
            // Kick off the background foreground-process polling thread. It is
            // idempotent and cheap (a single thread that sleeps for 3 s between
            // checks and only polls when a session is marked active).
            terminal_manager.start_process_polling();
            app.manage(terminal_manager);

            // Lazy log purge — delete logs older than 7 days, 30s after startup.
            {
                let log_dir = storage::logging::log_directory();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    tokio::task::spawn_blocking(move || {
                        let _ = storage::logging::purge_old_logs(&log_dir, 7);
                    });
                });
            }

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
            app_core::commands::load_graph_chunk,
            app_core::commands::refresh_graph_layout,
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
            app_core::commands::stage_hunks,
            app_core::commands::unstage_hunks,
            app_core::commands::discard_hunks,
            app_core::commands::create_commit,
            app_core::commands::create_branch,
            app_core::commands::create_branch_at,
            app_core::commands::checkout_detached,
            app_core::commands::delete_branch,
            app_core::commands::rename_branch,
            app_core::commands::checkout_branch,
            app_core::commands::get_diff_workdir,
            app_core::commands::get_diff_index,
            app_core::commands::merge_branch,
            app_core::commands::rebase_branch,
            app_core::commands::cherry_pick,
            app_core::commands::revert_commit,
            app_core::commands::reset_to_commit,
            app_core::commands::amend_commit,
            app_core::commands::get_head_message,
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
            app_core::commands::ensure_commit_local,
            app_core::commands::list_tags,
            app_core::commands::create_tag,
            app_core::commands::delete_tag,
            app_core::commands::push_tag,
            app_core::commands::get_commit_stats,
            app_core::commands::list_tags_paginated,
            app_core::commands::search_tags,
            app_core::commands::get_conflict_status,
            app_core::commands::get_conflict_file_contents,
            app_core::commands::write_resolved_file,
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
            app_core::commands::trigger_workflow,
            app_core::commands::retry_ci_run,
            app_core::commands::retry_ci_failed_jobs,
            app_core::commands::retry_ci_job,
            app_core::commands::cancel_ci_run,
            app_core::commands::list_ci_workflows,
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
            app_core::commands::get_ui_scale,
            app_core::commands::set_ui_scale,
            app_core::commands::get_graph_columns,
            app_core::commands::set_graph_columns,
            app_core::task_commands::get_tasks,
            app_core::task_commands::get_task_output,
            app_core::task_commands::cancel_task,
            app_core::task_commands::task_cancel,
            app_core::commands::list_worktrees,
            app_core::commands::create_worktree,
            app_core::commands::remove_worktree,
            app_core::commands::worktree_lock,
            app_core::commands::worktree_unlock,
            app_core::commands::blame_file,
            app_core::commands::file_history,
            app_core::commands::get_rebase_commits,
            app_core::commands::start_interactive_rebase,
            app_core::commands::clear_layout_cache,
            app_core::commands::get_reflog,
            app_core::commands::clean_dry_run,
            app_core::commands::clean_paths,
            app_core::commands::list_config,
            app_core::commands::set_config,
            app_core::commands::unset_config,
            app_core::commands::add_config,
            app_core::commands::read_gitignore,
            app_core::commands::write_gitignore,
            app_core::commands::add_gitignore_pattern,
            app_core::commands::save_patch_to_file,
            app_core::commands::create_commit_patches,
            app_core::commands::create_working_tree_patch,
            app_core::commands::preview_patch,
            app_core::commands::apply_patch,
            app_core::commands::list_submodules,
            app_core::commands::init_submodule,
            app_core::commands::update_submodule,
            app_core::commands::update_all_submodules,
            app_core::commands::deinit_submodule,
            app_core::commands::add_submodule,
            app_core::commands::remove_submodule,
            app_core::commands::submodule_abs_path,
            app_core::commands::is_cli_authenticated,
            // MR/PR management
            app_core::commands::list_mr_prs,
            app_core::commands::get_mr_pr_detail,
            app_core::commands::get_mr_pr_diff,
            app_core::commands::create_mr_pr,
            app_core::commands::edit_mr_pr,
            app_core::commands::merge_mr_pr,
            app_core::commands::close_mr_pr,
            app_core::commands::approve_mr_pr,
            app_core::commands::request_changes_mr_pr,
            app_core::commands::add_mr_pr_comment,
            app_core::commands::add_mr_pr_inline_comment,
            // Phase 8.2 — MR/PR enhancements
            app_core::commands::add_mr_pr_labels,
            app_core::commands::remove_mr_pr_labels,
            app_core::commands::add_mr_pr_reviewers,
            app_core::commands::remove_mr_pr_reviewers,
            app_core::commands::mark_mr_pr_ready,
            app_core::commands::mark_mr_pr_draft,
            app_core::commands::reopen_mr_pr,
            app_core::commands::resolve_discussion,
            app_core::commands::unresolve_discussion,
            app_core::commands::list_labels,
            app_core::commands::checkout_mr_pr_locally,
            // Phase 8.3 — Issues
            app_core::commands::list_issues,
            app_core::commands::get_issue,
            app_core::commands::create_issue,
            app_core::commands::edit_issue,
            app_core::commands::close_issue,
            app_core::commands::reopen_issue,
            app_core::commands::add_issue_comment,
            app_core::commands::add_issue_labels,
            app_core::commands::remove_issue_labels,
            app_core::commands::add_issue_assignees,
            app_core::commands::remove_issue_assignees,
            app_core::commands::set_issue_milestone,
            app_core::commands::list_milestones,
            // Phase 8.5 — Releases
            app_core::commands::list_releases,
            app_core::commands::get_release_detail,
            app_core::commands::list_release_assets,
            app_core::commands::create_release,
            app_core::commands::edit_release,
            app_core::commands::delete_release,
            app_core::commands::publish_release,
            app_core::commands::delete_release_asset,
            app_core::commands::upload_release_asset,
            app_core::commands::create_tag_and_release,
            // Sidebar
            app_core::commands::get_sidebar_collapsed,
            app_core::commands::set_sidebar_collapsed,
            app_core::commands::get_sidebar_nav_layout,
            app_core::commands::set_sidebar_nav_layout,
            // Auto-update preference
            app_core::commands::get_auto_check_updates,
            app_core::commands::set_auto_check_updates,
            app_core::commands::get_reauth_dismissed,
            app_core::commands::set_reauth_dismissed,
            // Terminal
            app_core::terminal_commands::terminal_spawn,
            app_core::terminal_commands::terminal_write,
            app_core::terminal_commands::terminal_resize,
            app_core::terminal_commands::terminal_kill,
            app_core::terminal_commands::terminal_set_active,
            // Project snapshot cache
            app_core::commands::get_project_snapshot,
            app_core::commands::save_project_snapshot,
            // AI provider
            app_core::ai_commands::ai_get_providers,
            app_core::ai_commands::ai_get_repo_status,
            app_core::ai_commands::ai_refresh_detection,
            app_core::ai_commands::ai_generate_commit_message,
            app_core::ai_commands::ai_analyze_code,
            app_core::ai_commands::ai_generate_pr_description,
            app_core::ai_commands::ai_review_code,
            app_core::ai_commands::ai_review_pr,
            app_core::ai_commands::ai_launch_interactive,
            app_core::ai_commands::ai_launch_worktree,
            app_core::ai_commands::ai_resume_conversation,
            app_core::ai_commands::ai_list_conversations,
            app_core::ai_commands::ai_list_worktrees,
            app_core::ai_commands::ai_cleanup_worktree,
            app_core::ai_commands::ai_get_config_files,
            app_core::ai_commands::ai_read_config_file,
            app_core::ai_commands::ai_write_config_file,
            app_core::ai_commands::ai_create_config_file,
            app_core::ai_commands::ai_get_preferred_provider,
            app_core::ai_commands::ai_set_preferred_provider,
            app_core::ai_commands::ai_watch_config_dirs,
            app_core::ai_commands::ai_stop_config_watcher,
            // Phase 10 — AI Background Worktree
            app_core::commands::ai_start_background_run,
            app_core::commands::ai_cancel_background_run,
            app_core::commands::ai_list_background_runs,
            app_core::commands::ai_get_background_run,
            app_core::commands::ai_discard_background_run_worktree,
            app_core::commands::ai_open_background_terminal,
            app_core::commands::ai_background_get_settings,
            app_core::commands::ai_background_set_settings,
            // Bisect
            app_core::commands::bisect_start,
            app_core::commands::bisect_good,
            app_core::commands::bisect_bad,
            app_core::commands::bisect_skip,
            app_core::commands::bisect_reset,
            app_core::commands::bisect_get_state,
            app_core::commands::bisect_get_log,
            app_core::commands::bisect_run_auto,
            app_core::commands::cli_check_auth_status,
            app_core::commands::cli_get_auth_command,
            app_core::commands::cli_get_logout_command,
            // Remote repo configuration (gh/glab)
            app_core::commands::load_remote_repo_config,
            app_core::commands::apply_remote_repo_config,
            app_core::commands::create_label,
            app_core::commands::update_label,
            app_core::commands::delete_label,
            app_core::commands::get_branch_protection,
            app_core::commands::set_branch_protection,
            app_core::commands::probe_forge_cli_status,
            app_core::commands::get_debug_info,
            app_core::commands::get_log_path,
            app_core::commands::open_log_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
