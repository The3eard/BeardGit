//! Mutation event system — snapshot + diff + guard + emit for
//! repository state changes.
//!
//! Every mutating command / watcher / AI run uses this crate to emit
//! one `project-mutated` Tauri event with a precise [`MutationFlags`]
//! describing what changed. The TS listener dispatches the minimal
//! refresh set from the flags.

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
