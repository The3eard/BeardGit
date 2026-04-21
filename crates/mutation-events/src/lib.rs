//! Mutation event system — snapshot + diff + guard + emit for
//! repository state changes.
//!
//! Every mutating command / watcher / AI run uses this crate to emit
//! one `project-mutated` Tauri event with a precise [`MutationFlags`]
//! describing what changed. The TS listener dispatches the minimal
//! refresh set from the flags.
//!
//! # Modules
//! - [`kind`] — `MutationKind` enum identifying the source of a mutation
//! - [`flags`] — `MutationFlags` bit-like struct describing what changed
//! - [`snapshot`] — cheap repo state capture + diff used to derive flags
//! - [`emit`] — single `emit_mutation` entrypoint for the Tauri event bus
//! - [`guard`] — RAII `MutationGuard` that snapshots + diffs + emits

mod emit;
mod flags;
mod guard;
mod kind;
mod snapshot;

pub use emit::{emit_mutation, EmitError};
pub use flags::MutationFlags;
pub use guard::{GuardError, MutationGuard};
pub use kind::{AiSource, MutationKind};
pub use snapshot::{Snapshot, SnapshotError};
