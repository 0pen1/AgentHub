use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex as StdMutex};

struct PreparedLaunch {
    executable: String,
    args: Vec<String>,
    work_dir: String,
    env_overrides: HashMap<String, String>,
}

/// A live PTY session.
pub struct PtySession {
    /// Kept for resize and reader cloning
    pub master: Box<dyn MasterPty + Send>,
    /// Separate writer handle (taken from master via take_writer)
    pub writer: Arc<StdMutex<Box<dyn Write + Send>>>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct PtyManager {
    sessions: HashMap<String, PtySession>,
    prepared: HashMap<String, PreparedLaunch>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            prepared: HashMap::new(),
        }
    }

    /// Stage a prepared launch: save config for later spawn with real terminal size
    pub fn stage_prepared(
        &mut self,
        session_id: &str,
        executable: String,
        args: Vec<String>,
        work_dir: String,
        env_overrides: HashMap<String, String>,
    ) {
        self.prepared.insert(
            session_id.to_string(),
            PreparedLaunch {
                executable,
                args,
                work_dir,
                env_overrides,
            },
        );
    }

    /// Spawn a staged session at the frontend's measured terminal size
    pub fn spawn_prepared(
        &mut self,
        session_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let prep = self
            .prepared
            .remove(session_id)
            .ok_or("No prepared launch for this session")?;
        self.spawn(
            session_id,
            &prep.executable,
            &prep.args,
            &prep.work_dir,
            cols,
            rows,
            prep.env_overrides,
        )
    }

    pub fn spawn(
        &mut self,
        session_id: &str,
        executable: &str,
        args: &[String],
        work_dir: &str,
        cols: u16,
        rows: u16,
        env_overrides: HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(executable);
        cmd.args(args);
        cmd.cwd(work_dir);
        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "en_US.UTF-8");

        // AgentHub may itself be launched from inside a Claude Code session (dev
        // workflow). CommandBuilder inherits the whole parent environment, which
        // drags in CLAUDE_CODE_* / ANTHROPIC_* markers — claude then treats the
        // session as a nested child and DISABLES transcript saving (interactive
        // mode), so --resume has nothing to resume. Strip the marker variables
        // so sessions behave like a normal terminal launch, and force
        // persistence on as a belt-and-braces guard for any marker we don't
        // know about.
        for marker in [
            "CLAUDE_CODE_CHILD_SESSION",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_EXECPATH",
            "CLAUDECODE",
            "CLAUDE_PID",
            "CLAUDE_CODE_SKIP_PROMPT_HISTORY",
        ] {
            cmd.env_remove(marker);
        }
        cmd.env("CLAUDE_CODE_FORCE_SESSION_PERSISTENCE", "1");

        for (key, value) in &env_overrides {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd)?;

        // Drop the slave so the PTY closes properly when the child exits
        drop(pair.slave);

        // take_writer() gives us a Write handle separate from the master,
        // so we can write without locking the master (needed for resize).
        let writer = pair.master.take_writer()?;

        let session = PtySession {
            master: pair.master,
            writer: Arc::new(StdMutex::new(writer)),
            child,
        };

        self.sessions.insert(session_id.to_string(), session);
        Ok(())
    }

    pub fn write(
        &self,
        session_id: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.sessions.get(session_id).ok_or("Session not found")?;
        let mut writer = session
            .writer
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.sessions.get(session_id).ok_or("Session not found")?;
        session.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn kill(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut session) = self.sessions.remove(session_id) {
            graceful_kill(&mut session);
        }
        Ok(())
    }

    /// Gracefully terminate every live PTY (app shutdown). Best-effort: a
    /// session that resists SIGTERM still gets SIGKILL'd via graceful_kill's
    /// escalation, but nothing here blocks shutdown indefinitely — the wait
    /// is capped per session inside graceful_kill.
    pub fn kill_all(&mut self) {
        for (_, mut session) in self.sessions.drain() {
            graceful_kill(&mut session);
        }
    }

    /// Remove a session's PTY without killing (used by restart to clear stale entries)
    pub fn remove(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Check if a session has already been spawned (guards against duplicate attach calls)
    pub fn is_spawned(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// Clone a reader handle for the PTY output. Used by the streaming task.
    pub fn get_reader(
        &self,
        session_id: &str,
    ) -> Result<Box<dyn std::io::Read + Send>, Box<dyn std::error::Error>> {
        let session = self.sessions.get(session_id).ok_or("Session not found")?;
        let reader = session.master.try_clone_reader()?;
        Ok(reader)
    }
}

/// Escalating termination: close the PTY writer (EOF — TUI agents exit their
/// input loop cleanly), then SIGTERM, then SIGKILL after a short grace period.
/// `ChildKiller::kill` is SIGKILL-strength on Unix, so it is the LAST resort,
/// not the first — agents get a chance to flush conversation state.
fn graceful_kill(session: &mut PtySession) {
    // 1. EOF: closing the master-side writer makes the child's stdin read
    //    return EOF, which interactive CLIs treat as "exit".
    {
        let mut writer = session.writer.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writer.flush();
    }

    // 2. SIGTERM and wait briefly for a natural exit.
    #[cfg(unix)]
    if let Some(pid) = session.child.process_id() {
        // SAFETY: sending SIGTERM to a pid we own (spawned via our PTY).
        let _ = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(3000);
        while std::time::Instant::now() < deadline {
            if let Ok(Some(_)) = session.child.try_wait() {
                return; // exited on its own
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    // 3. Still alive — SIGKILL via portable-pty's killer.
    let _ = session.child.kill();
}
