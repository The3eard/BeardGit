//! Re-export of the AI background coordinator.
//!
//! The coordinator (queue, concurrency cap, worktree lifecycle, session
//! registry, `pending_finishes`/`apply_task_finish` race handling) now lives in
//! the tauri-free [`ai_runner`] crate so it can be unit-tested without Tauri.
//! app-core keeps the Tauri event-sink impls (see `crate::event_sink`) and the
//! command glue (see `crate::commands::ai_background`).

pub use ai_runner::*;
