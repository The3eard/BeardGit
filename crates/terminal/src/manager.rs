//! Terminal session manager — spawns and manages PTY sessions.

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::shell::detect_shell;
use crate::sink::TerminalEventSink;
use crate::types::{SessionId, TerminalConfig, TerminalError};

/// Interval between foreground-process polls on the active session.
const PROCESS_POLL_INTERVAL: Duration = Duration::from_secs(3);

/// Handle to a running terminal session.
struct Session {
    /// Writer for sending input to the PTY.
    writer: Box<dyn std::io::Write + Send>,
    /// The PTY pair (kept alive to prevent the PTY from closing).
    _pair: portable_pty::PtyPair,
    /// The child process.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Master PTY file descriptor (Unix only, for `tcgetpgrp`).
    #[cfg(unix)]
    master_fd: Option<i32>,
    /// Last known foreground process name — used to detect changes. Unix
    /// only; Windows has no equivalent of `tcgetpgrp` so the polling
    /// branch that reads this field is `#[cfg(unix)]`-gated. Without the
    /// matching gate here Windows builds emit a `dead_code` warning.
    #[cfg(unix)]
    last_fg_process: Option<String>,
}

/// Manages terminal PTY sessions.
pub struct TerminalManager {
    sessions: Mutex<HashMap<SessionId, Session>>,
    next_id: AtomicU64,
    sink: Arc<dyn TerminalEventSink>,
    /// The session ID of the currently visible terminal (for process polling).
    active_session: Mutex<Option<SessionId>>,
    /// Flag controlling the polling thread lifecycle.
    polling_active: Arc<AtomicBool>,
}

impl TerminalManager {
    /// Create a new terminal manager that emits events through the given sink.
    pub fn new(sink: Arc<dyn TerminalEventSink>) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            sink,
            active_session: Mutex::new(None),
            polling_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn a new terminal session. Returns the session ID immediately.
    pub fn spawn(&self, config: TerminalConfig) -> Result<SessionId, TerminalError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let shell = config
            .shell
            .unwrap_or_else(|| detect_shell().unwrap_or_else(|_| "/bin/sh".to_string()));

        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::SpawnFailed(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&config.cwd);
        for arg in &config.args {
            cmd.arg(arg);
        }
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::SpawnFailed(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::SpawnFailed(e.to_string()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::SpawnFailed(e.to_string()))?;

        // Capture master fd for foreground-process detection on Unix.
        // `MasterPty::as_raw_fd` is the trait method; no std import needed.
        #[cfg(unix)]
        let master_fd: Option<i32> = pair.master.as_raw_fd();

        // Spawn OS thread to read PTY output (byte-oriented, not line-buffered)
        let sink = Arc::clone(&self.sink);
        thread::spawn(move || {
            use crate::osc::scan_osc7;

            let mut buf = [0u8; 4096];
            let mut osc_pending: Vec<u8> = Vec::new();

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let chunk = &buf[..n];

                        // Scan for OSC 7 cwd changes
                        let result = scan_osc7(&osc_pending, chunk);
                        if let Some(cwd) = result.cwd {
                            sink.on_cwd_changed(id, cwd);
                        }

                        // Carry over incomplete OSC 7 prefix for the next chunk.
                        if result.pending_bytes > 0 {
                            let mut combined = std::mem::take(&mut osc_pending);
                            combined.extend_from_slice(chunk);
                            let start = combined.len().saturating_sub(result.pending_bytes);
                            osc_pending = combined[start..].to_vec();
                        } else {
                            osc_pending.clear();
                        }

                        // Forward all bytes to frontend (xterm.js handles/ignores OSC 7)
                        sink.on_output(id, chunk);
                    }
                    Err(_) => break,
                }
            }
            // PTY closed — notify exit. Exit code retrieval is best-effort.
            sink.on_exit(id, None);
        });

        let session = Session {
            writer,
            _pair: pair,
            _child: child,
            #[cfg(unix)]
            master_fd,
            #[cfg(unix)]
            last_fg_process: None,
        };

        self.sessions
            .lock()
            .expect("sessions lock poisoned")
            .insert(id, session);

        Ok(id)
    }

    /// Write input data to a terminal session.
    pub fn write(&self, id: SessionId, data: &[u8]) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.lock().expect("sessions lock poisoned");
        let session = sessions.get_mut(&id).ok_or(TerminalError::NotFound(id))?;
        session.writer.write_all(data).map_err(TerminalError::Io)?;
        session.writer.flush().map_err(TerminalError::Io)?;
        Ok(())
    }

    /// Resize a terminal session.
    pub fn resize(&self, id: SessionId, cols: u16, rows: u16) -> Result<(), TerminalError> {
        let sessions = self.sessions.lock().expect("sessions lock poisoned");
        let session = sessions.get(&id).ok_or(TerminalError::NotFound(id))?;
        session
            ._pair
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::SpawnFailed(e.to_string()))?;
        Ok(())
    }

    /// Kill a terminal session and remove it.
    pub fn kill(&self, id: SessionId) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.lock().expect("sessions lock poisoned");
        sessions.remove(&id).ok_or(TerminalError::NotFound(id))?;
        // Dropping the Session closes the PTY and kills the child
        Ok(())
    }

    /// Kill all active terminal sessions and stop the polling thread.
    pub fn kill_all(&self) {
        self.stop_process_polling();
        let mut sessions = self.sessions.lock().expect("sessions lock poisoned");
        sessions.clear();
    }

    /// Set which terminal session is currently visible in the UI.
    ///
    /// The process polling thread only polls this session, minimizing
    /// syscalls when many terminals are open but only one is shown.
    pub fn set_active_session(&self, session_id: Option<SessionId>) {
        *self
            .active_session
            .lock()
            .expect("active_session lock poisoned") = session_id;
    }

    /// Get the currently active session ID.
    pub fn get_active_session(&self) -> Option<SessionId> {
        *self
            .active_session
            .lock()
            .expect("active_session lock poisoned")
    }

    /// Start the background process-polling thread.
    ///
    /// Polls the foreground process of the active terminal session every
    /// `PROCESS_POLL_INTERVAL`. Emits `on_foreground_process_changed` when
    /// the process name changes. Idempotent: calling twice has no effect.
    /// The thread auto-exits when `polling_active` is cleared.
    pub fn start_process_polling(self: &Arc<Self>) {
        // Idempotency: if already started, do nothing.
        if self.polling_active.swap(true, Ordering::SeqCst) {
            return;
        }

        let manager = Arc::clone(self);

        thread::spawn(move || {
            while manager.polling_active.load(Ordering::Relaxed) {
                thread::sleep(PROCESS_POLL_INTERVAL);

                if !manager.polling_active.load(Ordering::Relaxed) {
                    break;
                }

                let active_id = match manager.get_active_session() {
                    Some(id) => id,
                    None => continue,
                };

                #[cfg(unix)]
                {
                    // Compute the diff under the lock, then drop it before
                    // invoking the sink to avoid holding the mutex across an
                    // arbitrary-duration callback.
                    let change: Option<(SessionId, Option<String>)> = {
                        let mut sessions = manager.sessions.lock().expect("sessions lock poisoned");
                        sessions.get_mut(&active_id).and_then(|session| {
                            let fd = session.master_fd?;
                            let current = crate::process::get_foreground_process_name(fd);
                            if current == session.last_fg_process {
                                None
                            } else {
                                session.last_fg_process = current.clone();
                                Some((active_id, current))
                            }
                        })
                    };
                    if let Some((id, name)) = change {
                        manager.sink.on_foreground_process_changed(id, name);
                    }
                }

                #[cfg(not(unix))]
                {
                    // Windows: foreground-process detection unsupported.
                    let _ = active_id;
                }
            }
        });
    }

    /// Stop the process polling thread.
    pub fn stop_process_polling(&self) {
        self.polling_active.store(false, Ordering::Relaxed);
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        self.stop_process_polling();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    /// Test sink that collects output.
    struct CollectingSink {
        received_output: Mutex<Vec<Vec<u8>>>,
        received_exit: AtomicBool,
        received_cwds: Mutex<Vec<String>>,
        received_processes: Mutex<Vec<Option<String>>>,
    }

    impl CollectingSink {
        fn new() -> Self {
            Self {
                received_output: Mutex::new(Vec::new()),
                received_exit: AtomicBool::new(false),
                received_cwds: Mutex::new(Vec::new()),
                received_processes: Mutex::new(Vec::new()),
            }
        }

        fn output_bytes(&self) -> Vec<u8> {
            self.received_output
                .lock()
                .unwrap()
                .iter()
                .flatten()
                .copied()
                .collect()
        }
    }

    impl TerminalEventSink for CollectingSink {
        fn on_output(&self, _id: SessionId, data: &[u8]) {
            self.received_output.lock().unwrap().push(data.to_vec());
        }

        fn on_exit(&self, _id: SessionId, _code: Option<u32>) {
            self.received_exit.store(true, Ordering::Relaxed);
        }

        fn on_cwd_changed(&self, _id: SessionId, cwd: String) {
            self.received_cwds.lock().unwrap().push(cwd);
        }

        fn on_foreground_process_changed(&self, _id: SessionId, name: Option<String>) {
            self.received_processes.lock().unwrap().push(name);
        }
    }

    #[test]
    fn spawn_and_receive_output() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = TerminalManager::new(Arc::clone(&sink) as Arc<dyn TerminalEventSink>);

        let config = TerminalConfig {
            cwd: std::env::temp_dir(),
            shell: None,
            args: Vec::new(),
            env: HashMap::new(),
            cols: 80,
            rows: 24,
        };

        let id = mgr.spawn(config).expect("spawn should succeed");
        assert!(id > 0);

        // Write a command and wait for output
        mgr.write(id, b"echo hello_terminal_test\n")
            .expect("write should succeed");
        thread::sleep(Duration::from_millis(500));

        let bytes = sink.output_bytes();
        let output = String::from_utf8_lossy(&bytes);
        assert!(
            output.contains("hello_terminal_test"),
            "output should contain our echo: {output}"
        );

        mgr.kill(id).expect("kill should succeed");
    }

    #[test]
    fn kill_nonexistent_returns_not_found() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = TerminalManager::new(sink);

        let result = mgr.kill(999);
        assert!(matches!(result, Err(TerminalError::NotFound(999))));
    }

    #[test]
    fn write_to_nonexistent_returns_not_found() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = TerminalManager::new(sink);

        let result = mgr.write(999, b"test");
        assert!(matches!(result, Err(TerminalError::NotFound(999))));
    }

    #[test]
    fn kill_all_clears_sessions() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = TerminalManager::new(Arc::clone(&sink) as Arc<dyn TerminalEventSink>);

        let config = TerminalConfig {
            cwd: std::env::temp_dir(),
            shell: None,
            args: Vec::new(),
            env: HashMap::new(),
            cols: 80,
            rows: 24,
        };

        let id1 = mgr.spawn(config.clone()).unwrap();
        let id2 = mgr.spawn(config).unwrap();
        assert_ne!(id1, id2);

        mgr.kill_all();

        assert!(matches!(mgr.kill(id1), Err(TerminalError::NotFound(_))));
        assert!(matches!(mgr.kill(id2), Err(TerminalError::NotFound(_))));
    }

    #[test]
    fn set_active_session_tracks_visibility() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = TerminalManager::new(Arc::clone(&sink) as Arc<dyn TerminalEventSink>);

        let config = TerminalConfig {
            cwd: std::env::temp_dir(),
            shell: None,
            args: Vec::new(),
            env: HashMap::new(),
            cols: 80,
            rows: 24,
        };

        let id = mgr.spawn(config).expect("spawn should succeed");
        mgr.set_active_session(Some(id));
        assert_eq!(mgr.get_active_session(), Some(id));

        mgr.set_active_session(None);
        assert_eq!(mgr.get_active_session(), None);

        mgr.kill(id).expect("kill should succeed");
    }

    #[test]
    fn start_process_polling_is_idempotent() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = Arc::new(TerminalManager::new(
            Arc::clone(&sink) as Arc<dyn TerminalEventSink>
        ));

        // Calling start twice must not panic or spawn duplicate threads.
        mgr.start_process_polling();
        mgr.start_process_polling();

        mgr.stop_process_polling();
    }
}
