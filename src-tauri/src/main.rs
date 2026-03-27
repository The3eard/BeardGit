//! Binary entry point for the BeardGit desktop application.
//!
//! This file is intentionally minimal — all application logic lives in
//! `beardgit-lib` (`src-tauri/src/lib.rs`). The `windows_subsystem` attribute
//! suppresses the console window on Windows release builds.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Application entry point. Delegates entirely to [`beardgit_lib::run`].
fn main() {
    beardgit_lib::run()
}
