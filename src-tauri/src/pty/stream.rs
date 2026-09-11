use base64::Engine;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex as StdMutex;
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

use super::scrollback::{save_snapshot, ScrollbackBuffer};

/// App-wide registry of per-session PTY tail buffers, managed as Tauri state.
pub struct ScrollbackMap(pub StdMutex<HashMap<String, ScrollbackBuffer>>);

impl ScrollbackMap {
    pub fn new() -> Self {
        Self(StdMutex::new(HashMap::new()))
    }
}

/// Feed a chunk into the session's tail buffer and emit it to the frontend.
fn emit_chunk(app: &AppHandle, session_id: &str, buffers: &ScrollbackMap, data: &[u8]) {
    if let Ok(mut map) = buffers.0.lock() {
        map.entry(session_id.to_string())
            .or_insert_with(ScrollbackBuffer::default)
            .push(data);
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    let _ = app.emit(&format!("pty-output:{}", session_id), encoded);
}

/// Persist one session's tail buffer to disk (best-effort).
#[allow(dead_code)] // reserved for future per-session flush (e.g. on tab close)
pub fn persist_session(app: &AppHandle, session_id: &str) {
    if let Some(buffers) = app.try_state::<ScrollbackMap>() {
        if let Ok(map) = buffers.0.lock() {
            if let Some(buf) = map.get(session_id) {
                if !buf.is_empty() {
                    save_snapshot(session_id, &buf.snapshot());
                }
            }
        }
    }
}

/// Persist ALL session tail buffers to disk. Called on app quit (window close).
pub fn persist_all(app: &AppHandle) {
    if let Some(buffers) = app.try_state::<ScrollbackMap>() {
        if let Ok(map) = buffers.0.lock() {
            for (session_id, buf) in map.iter() {
                if !buf.is_empty() {
                    save_snapshot(session_id, &buf.snapshot());
                }
            }
        }
    }
}

/// Drop a session's in-memory buffer (delete_session).
pub fn drop_session_buffer(app: &AppHandle, session_id: &str) {
    if let Some(buffers) = app.try_state::<ScrollbackMap>() {
        if let Ok(mut map) = buffers.0.lock() {
            map.remove(session_id);
        }
    }
}

/// Start a background OS thread that reads from the PTY, feeds the session's
/// scrollback tail buffer, and emits output to the frontend.
/// Uses a plain thread (NOT tokio::spawn) because Tauri sync commands run outside
/// the Tokio runtime context, and `tokio::task::spawn` would panic there.
pub fn start_reader_task(app: AppHandle, session_id: String, mut reader: Box<dyn Read + Send>) {
    thread::spawn(move || {
        let buffers = app.state::<ScrollbackMap>();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF: PTY closed (agent exited) — flush this session's tail now
                    // so it's durable even if the app never exits cleanly.
                    let snapshot = buffers
                        .0
                        .lock()
                        .ok()
                        .and_then(|m| m.get(&session_id).map(|b| b.snapshot()))
                        .unwrap_or_default();
                    if !snapshot.is_empty() {
                        save_snapshot(&session_id, &snapshot);
                    }

                    // Sync DB: mark as exited if it was running (preserve "killed" status)
                    if let Some(store) =
                        app.try_state::<StdMutex<crate::session::store::SessionStore>>()
                    {
                        if let Ok(s) = store.lock() {
                            let _ = s.update_status_if_running(&session_id, "exited");
                        }
                    }
                    let _ = app.emit("sessions-changed", ());
                    break;
                }
                Ok(n) => {
                    emit_chunk(&app, &session_id, &buffers, &buf[..n]);
                }
                Err(e) => {
                    eprintln!("PTY read error for {}: {}", session_id, e);
                    break;
                }
            }
        }
        // Emit a final "closed" event
        let _ = app.emit(&format!("pty-closed:{}", session_id), ());
    });
}
