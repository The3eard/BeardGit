//! Tauri event sink that bridges the `terminal` crate to the Tauri event system.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::{AppHandle, Emitter};
use terminal::{
    SessionId, TerminalCwdChangedEvent, TerminalEventSink, TerminalExitEvent, TerminalOutputEvent,
    TerminalProcessChangedEvent,
};

/// Forwards terminal events to the Tauri frontend via `AppHandle::emit`.
pub struct TauriTerminalSink {
    app_handle: AppHandle,
}

impl TauriTerminalSink {
    /// Create a new [`TauriTerminalSink`] wrapping the given app handle.
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl TerminalEventSink for TauriTerminalSink {
    fn on_output(&self, session_id: SessionId, data: &[u8]) {
        let encoded = BASE64.encode(data);
        let _ = self.app_handle.emit(
            "terminal-output",
            &TerminalOutputEvent {
                session_id,
                data: encoded,
            },
        );
    }

    fn on_exit(&self, session_id: SessionId, exit_code: Option<u32>) {
        let _ = self.app_handle.emit(
            "terminal-exit",
            &TerminalExitEvent {
                session_id,
                exit_code,
            },
        );
    }

    fn on_cwd_changed(&self, session_id: SessionId, cwd: String) {
        let _ = self.app_handle.emit(
            "terminal-cwd-changed",
            &TerminalCwdChangedEvent { session_id, cwd },
        );
    }

    fn on_foreground_process_changed(&self, session_id: SessionId, process_name: Option<String>) {
        let _ = self.app_handle.emit(
            "terminal-process-changed",
            &TerminalProcessChangedEvent {
                session_id,
                process_name,
            },
        );
    }
}
