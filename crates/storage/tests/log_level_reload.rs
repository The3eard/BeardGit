//! Integration coverage for live log-level changes.
//!
//! This lives in `tests/` rather than beside the module for two reasons:
//! it needs its own process so it can install a global subscriber exactly
//! once, and it needs `debug_assertions` on — which is where the bug this
//! guards against actually manifests.
//!
//! ## What this is guarding
//!
//! The natural way to build a reloadable filtered layer is to wrap the
//! whole `Filtered<fmt, EnvFilter>` in one `reload::Layer`. That compiles,
//! passes every unit test, passes `clippy`, and panics on the first event
//! after `set_level` in any debug build — `Filtered` receives its
//! `FilterId` from `Layer::on_layer`, which only runs during subscriber
//! construction, so a reloaded value carries `FilterId::disabled()`.
//!
//! Release builds happened to survive it. `cargo test` does not. So the
//! mere fact that this file runs to completion is the assertion that
//! matters most here.

use std::io;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// An in-memory sink the test can read back. The real appender flushes on
/// a background thread, which would make these assertions timing-dependent.
#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("buffer mutex poisoned")).into_owned()
    }

    fn clear(&self) {
        self.0.lock().expect("buffer mutex poisoned").clear();
    }
}

impl io::Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("buffer mutex poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Emit a span so its close event exercises the fmt-layer swap. The span
/// is INFO (the `#[instrument]` default, like the ~165 real ones), so it
/// only *renders* when `FmtSpan::CLOSE` is active.
#[tracing::instrument]
fn emit_inside_span(marker: &str) {}

/// Emit at every level from both an allowlisted workspace target and a
/// third-party one.
///
/// The target split is the point: `debug` promotes only the crates in
/// `WORKSPACE_TARGETS` and must leave everything else at `info`. Emitting
/// from `hyper` here is what proves that, and it is why the unit test on
/// the directive string alone is not enough — `EnvFilter` has no idea
/// which crates exist, so a bogus allowlist passes that test.
fn emit_all(marker: &str) {
    tracing::error!(target: "storage", marker, "ERROR_EVENT");
    tracing::info!(target: "storage", marker, "INFO_EVENT");
    tracing::debug!(target: "storage", marker, "WORKSPACE_DEBUG_EVENT");
    tracing::debug!(target: "hyper", marker, "THIRD_PARTY_DEBUG_EVENT");
    emit_inside_span(marker);
}

#[test]
fn level_changes_take_effect_without_a_restart() {
    let buffer = SharedBuffer::default();
    storage::logging::init_with_writer("info", buffer.clone())
        .expect("subscriber should install once per test process");

    // ── Start at info: info and error through, debug suppressed ──────────
    emit_all("phase_info");
    let at_info = buffer.contents();
    assert!(at_info.contains("ERROR_EVENT"), "got: {at_info}");
    assert!(at_info.contains("INFO_EVENT"), "got: {at_info}");
    assert!(
        !at_info.contains("DEBUG_EVENT"),
        "debug must be suppressed at info, got: {at_info}"
    );
    assert!(
        !at_info.contains("THIRD_PARTY_DEBUG_EVENT"),
        "third-party debug must be suppressed at info, got: {at_info}"
    );
    // `FmtSpan::NONE` at info — no span close lines.
    assert!(
        !at_info.contains("close"),
        "span close events must be off at info, got: {at_info}"
    );

    // ── Upgrade to debug. Reaching the next line at all is the point: a
    //    `Filtered`-wrapped reload panics here under debug_assertions. ────
    buffer.clear();
    storage::logging::set_level("debug").expect("upgrade to debug should succeed");

    emit_all("phase_debug");
    let at_debug = buffer.contents();
    assert!(
        at_debug.contains("WORKSPACE_DEBUG_EVENT"),
        "workspace debug events must appear after upgrading, got: {at_debug}"
    );
    assert!(
        at_debug.contains("INFO_EVENT"),
        "info must still pass at debug, got: {at_debug}"
    );
    // The allowlist's whole purpose: our events must not be buried under
    // transport and event-loop chatter when the user asks for debug.
    assert!(
        !at_debug.contains("THIRD_PARTY_DEBUG_EVENT"),
        "third-party crates must stay at info even when debug is selected, got: {at_debug}"
    );
    // The span-close timing lines are where the per-command detail comes
    // from, so verify the fmt layer really swapped, not just the filter.
    assert!(
        at_debug.contains("close"),
        "span close events must appear at debug, got: {at_debug}"
    );

    // ── Downgrade to error: info and debug both gone ─────────────────────
    buffer.clear();
    storage::logging::set_level("error").expect("downgrade to error should succeed");

    emit_all("phase_error");
    let at_error = buffer.contents();
    assert!(at_error.contains("ERROR_EVENT"), "got: {at_error}");
    assert!(
        !at_error.contains("INFO_EVENT"),
        "info must be suppressed at error, got: {at_error}"
    );
    assert!(
        !at_error.contains("DEBUG_EVENT"),
        "debug must be suppressed at error, got: {at_error}"
    );

    // ── Back up to info, to prove the swap is not one-way ────────────────
    buffer.clear();
    storage::logging::set_level("info").expect("returning to info should succeed");

    emit_all("phase_info_again");
    let back_at_info = buffer.contents();
    assert!(
        back_at_info.contains("INFO_EVENT"),
        "info must come back, got: {back_at_info}"
    );
    assert!(
        !back_at_info.contains("DEBUG_EVENT"),
        "debug must be suppressed again, got: {back_at_info}"
    );

    // ── An unknown level is rejected and changes nothing ─────────────────
    buffer.clear();
    let err = storage::logging::set_level("shout").expect_err("unknown level must be rejected");
    assert!(err.contains("unknown log level"), "got: {err}");
    emit_all("phase_after_reject");
    assert!(
        buffer.contents().contains("INFO_EVENT"),
        "a rejected level must leave the previous one in place"
    );

    // ── Redaction is still in the path after all those swaps ─────────────
    buffer.clear();
    storage::logging::set_level("debug").expect("upgrade to debug should succeed");
    tracing::info!(
        token = "ghp_EXAMPLE_FAKE_PAT_VALUE_1234567890",
        "credential shaped value"
    );
    let redacted = buffer.contents();
    assert!(
        !redacted.contains("ghp_"),
        "reloading must not drop the redacting writer, got: {redacted}"
    );
    assert!(redacted.contains("<redacted>"), "got: {redacted}");
}
