//! Event sink trait for decoupling terminal events from the transport layer.
//!
//! The `terminal` crate emits events through this trait. Consumers (e.g.
//! `app-core`) implement it to forward events to Tauri, a test harness, etc.

use crate::types::SessionId;

/// Receives terminal output and lifecycle events.
///
/// All methods take `&self` so the sink can be shared across threads.
pub trait TerminalEventSink: Send + Sync {
    /// The terminal session produced output bytes.
    fn on_output(&self, session_id: SessionId, data: &[u8]);

    /// The terminal session exited.
    fn on_exit(&self, session_id: SessionId, exit_code: Option<u32>);
}
