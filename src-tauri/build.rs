//! Tauri build script.
//!
//! Runs at compile time to generate the Tauri context (capability manifest,
//! asset embedding, etc.) required by `tauri::generate_context!()` in the
//! library crate.

/// Invoke the Tauri build-time code generator.
fn main() {
    tauri_build::build()
}
