//! Tauri commands for terminal session management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;
use terminal::{SessionId, TerminalConfig, TerminalManager};

/// Spawn a new terminal session in the given directory.
#[tauri::command]
pub fn terminal_spawn(
    cwd: String,
    cols: u16,
    rows: u16,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<SessionId, String> {
    let config = TerminalConfig {
        cwd: PathBuf::from(cwd),
        shell: None,
        env: HashMap::new(),
        cols,
        rows,
    };
    terminal_manager.spawn(config).map_err(|e| e.to_string())
}

/// Write input bytes to a terminal session (base64-encoded from frontend).
#[tauri::command]
pub fn terminal_write(
    id: SessionId,
    data: String,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<(), String> {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64;

    let bytes = BASE64.decode(&data).map_err(|e| e.to_string())?;
    terminal_manager
        .write(id, &bytes)
        .map_err(|e| e.to_string())
}

/// Resize a terminal session.
#[tauri::command]
pub fn terminal_resize(
    id: SessionId,
    cols: u16,
    rows: u16,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<(), String> {
    terminal_manager
        .resize(id, cols, rows)
        .map_err(|e| e.to_string())
}

/// Kill a terminal session.
#[tauri::command]
pub fn terminal_kill(
    id: SessionId,
    terminal_manager: State<'_, Arc<TerminalManager>>,
) -> Result<(), String> {
    terminal_manager.kill(id).map_err(|e| e.to_string())
}
