//! Terminal session manager — spawns and manages PTY sessions.

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::shell::detect_shell;
use crate::sink::TerminalEventSink;
use crate::types::{SessionId, TerminalConfig, TerminalError};

/// Handle to a running terminal session.
struct Session {
    /// Writer for sending input to the PTY.
    writer: Box<dyn std::io::Write + Send>,
    /// The PTY pair (kept alive to prevent the PTY from closing).
    _pair: portable_pty::PtyPair,
    /// The child process.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

/// Manages terminal PTY sessions.
pub struct TerminalManager {
    sessions: Mutex<HashMap<SessionId, Session>>,
    next_id: AtomicU64,
    sink: Arc<dyn TerminalEventSink>,
}

impl TerminalManager {
    /// Create a new terminal manager that emits events through the given sink.
    pub fn new(sink: Arc<dyn TerminalEventSink>) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            sink,
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

        // Spawn OS thread to read PTY output (byte-oriented, not line-buffered)
        let sink = Arc::clone(&self.sink);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => sink.on_output(id, &buf[..n]),
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

    /// Kill all active terminal sessions.
    pub fn kill_all(&self) {
        let mut sessions = self.sessions.lock().expect("sessions lock poisoned");
        sessions.clear();
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
    }

    impl CollectingSink {
        fn new() -> Self {
            Self {
                received_output: Mutex::new(Vec::new()),
                received_exit: AtomicBool::new(false),
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
    }

    #[test]
    fn spawn_and_receive_output() {
        let sink = Arc::new(CollectingSink::new());
        let mgr = TerminalManager::new(Arc::clone(&sink) as Arc<dyn TerminalEventSink>);

        let config = TerminalConfig {
            cwd: std::env::temp_dir(),
            shell: None,
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
}
