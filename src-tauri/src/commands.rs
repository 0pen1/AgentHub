use base64::Engine as _;
use crate::agent::registry::AgentRegistry;
use crate::config::injector::ConfigInjector;
use crate::config::mcp::McpScanner;
use crate::config::skill::SkillScanner;
use crate::pty::manager::PtyManager;
use crate::pty::stream::start_reader_task;
use crate::session::store::SessionStore;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

pub mod library;

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub version: Option<String>,
    pub installed: bool,
    pub config_format: String,
    pub instruction_file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerInfo {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    /// Remote transport type (sse/http) when present in the config value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    /// Remote URL when present in the config value (command is "(http)").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub name: String,
    pub work_dir: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LaunchConfig {
    pub agent_id: String,
    pub work_dir: String,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub instructions: Vec<String>,
}

// --- Config library structs ------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LibrarySkillDetail {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub path: String,
    pub content: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstructionInfo {
    pub name: String,
    pub path: String,
    pub description: String,
    pub tags: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstructionDetail {
    pub name: String,
    pub path: String,
    pub content: String,
}

/// MCP server form input. `extra` (flatten) preserves unknown fields like
/// `headers` through round-trips.
#[derive(Debug, Clone, Deserialize)]
pub struct McpEntryInput {
    pub name: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(rename = "type", default)]
    pub server_type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpImportCandidate {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub exists_in_library: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpImportResult {
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
}

/// Expert preset: a fixed launch combo (agent + skills + MCPs + prompts).
#[derive(Debug, Clone, Serialize)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub agent_id: String,
    pub tags: Vec<String>,
    pub work_dir: String,
    pub icon: String,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub instructions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PresetInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_agent")]
    pub agent_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub work_dir: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub mcps: Vec<String>,
    #[serde(default)]
    pub instructions: Vec<String>,
}

fn default_agent() -> String {
    "claude".to_string()
}

fn default_icon() -> String {
    "🤖".to_string()
}

#[tauri::command]
pub fn list_presets(
    session_state: State<'_, Mutex<SessionStore>>,
) -> Result<Vec<PresetInfo>, String> {
    let store = session_state.lock().map_err(|e| e.to_string())?;
    let rows = store.list_presets().map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| PresetInfo {
            name: r.name,
            description: r.description,
            agent_id: r.agent_id,
            tags: r.tags,
            work_dir: r.work_dir,
            icon: r.icon,
            skills: r.skills,
            mcps: r.mcps,
            instructions: r.instructions,
        })
        .collect())
}

#[tauri::command]
pub fn save_preset(
    input: PresetInput,
    session_state: State<'_, Mutex<SessionStore>>,
) -> Result<(), String> {
    let store = session_state.lock().map_err(|e| e.to_string())?;
    store
        .save_preset(
            &input.name,
            &input.description,
            &input.agent_id,
            &input.tags,
            &input.work_dir,
            &input.icon,
            &input.skills,
            &input.mcps,
            &input.instructions,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_preset(
    name: String,
    session_state: State<'_, Mutex<SessionStore>>,
) -> Result<(), String> {
    let store = session_state.lock().map_err(|e| e.to_string())?;
    store.delete_preset(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_agents() -> Vec<AgentInfo> {
    let registry = AgentRegistry::new();
    registry.scan_installed()
}

#[tauri::command]
pub fn list_skills() -> Vec<SkillInfo> {
    let scanner = SkillScanner::new();
    scanner.scan_all()
}

#[tauri::command]
pub fn list_mcp_servers() -> Vec<McpServerInfo> {
    let scanner = McpScanner::new();
    scanner.scan_all()
}

#[tauri::command]
pub fn launch_session(
    config: LaunchConfig,
    pty_state: State<'_, Mutex<PtyManager>>,
    session_state: State<'_, Mutex<SessionStore>>,
) -> Result<SessionInfo, String> {
    // 1. Inject config (returns env overrides + agent-specific launch args)
    let injector = ConfigInjector::new();
    let session_id = uuid::Uuid::new_v4().to_string();
    let (env_overrides, launch_args) = injector
        .prepare(
            &session_id,
            &config.agent_id,
            &config.work_dir,
            &config.skills,
            &config.mcps,
            &config.instructions,
        )
        .map_err(|e| e.to_string())?;

    // 2. Look up agent (PTY is spawned later in pty_attach with the real terminal size,
    //    so the agent's first TUI frame renders at the correct dimensions — no reflow ghosting)
    let registry = AgentRegistry::new();
    let agent = registry
        .get_agent(&config.agent_id)
        .ok_or("Agent not found")?;
    // Cheap installed check (cached, no version probe): failing here leaves no
    // zombie "running" DB row, and the user gets an actionable error instead of
    // a green dot for a session that can never spawn.
    if !registry.is_agent_installed(&config.agent_id) {
        return Err(format!(
            "{} 未安装或不在 PATH 中（executable: {}）",
            agent.name, agent.executable
        ));
    }

    {
        let mut pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;
        pty_mgr.stage_prepared(
            &session_id,
            agent.executable.clone(),
            launch_args,
            config.work_dir.clone(),
            env_overrides,
        );
    }

    // 3. Build session info
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_info = SessionInfo {
        id: session_id.clone(),
        agent_id: config.agent_id.clone(),
        agent_name: agent.name.clone(),
        name: format!(
            "{} - {}",
            agent.name,
            config.work_dir.split('/').last().unwrap_or("unknown")
        ),
        work_dir: config.work_dir.clone(),
        status: "running".to_string(),
        created_at: now.to_string(),
    };

    // 4. Persist to SQLite
    let store = session_state.lock().map_err(|e| e.to_string())?;
    store
        .insert_session(
            &session_info.id,
            &session_info.agent_id,
            &session_info.agent_name,
            &session_info.name,
            &session_info.work_dir,
            &config.skills,
            &config.mcps,
            &config.instructions,
        )
        .map_err(|e| e.to_string())?;

    Ok(session_info)
}

/// Restart a (dead or alive) session: re-inject config from DB, re-stage the PTY.
/// The frontend Terminal then calls pty_attach as usual to spawn and stream.
/// The agent resumes its previous conversation for this session (claude: --resume).
#[tauri::command]
pub fn restart_session(
    session_id: String,
    pty_state: State<'_, Mutex<PtyManager>>,
    session_state: State<'_, Mutex<SessionStore>>,
) -> Result<SessionInfo, String> {
    // 1. Load full config from DB
    let store = session_state.lock().map_err(|e| e.to_string())?;
    let (agent_id, agent_name, name, work_dir, skills, mcps, instructions) = store
        .get_session(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or("Session not found")?;
    drop(store);

    // 2. Re-inject config into a fresh session dir (reuse the SAME session id so
    //    the session keeps its identity; a new session_dir name avoids colliding
    //    with the old config, so overwrite is always clean)
    let injector = ConfigInjector::new();
    let (env_overrides, launch_args) = injector
        .prepare(
            &session_id,
            &agent_id,
            &work_dir,
            &skills,
            &mcps,
            &instructions,
        )
        .map_err(|e| e.to_string())?;

    // 3. Look up agent executable
    let registry = AgentRegistry::new();
    let agent = registry
        .get_agent(&agent_id)
        .ok_or("Agent not found")?;

    // 4. Stage the launch (PTY is spawned later by pty_attach at real terminal size)
    {
        let mut pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;
        // A stale live PTY would mean is_spawned()==true and pty_attach would no-op —
        // remove it first so the new spawn takes its place.
        pty_mgr.remove(&session_id);
        pty_mgr.stage_prepared(
            &session_id,
            agent.executable.clone(),
            launch_args,
            work_dir.clone(),
            env_overrides,
        );
    }

    // 5. Update DB status to running
    let store = session_state.lock().map_err(|e| e.to_string())?;
    store
        .update_status(&session_id, "running")
        .map_err(|e| e.to_string())?;
    drop(store);

    Ok(SessionInfo {
        id: session_id,
        agent_id,
        agent_name,
        name,
        work_dir,
        status: "running".to_string(),
        created_at: String::new(),
    })
}
/// and start streaming output. Called by the Terminal component on mount.
/// Idempotent: if the session is already spawned, this is a no-op (guards against
/// React Strict Mode double-mount in dev).
#[tauri::command]
pub fn pty_attach(
    app: AppHandle,
    session_id: String,
    cols: u16,
    rows: u16,
    pty_state: State<'_, Mutex<PtyManager>>,
    session_state: State<'_, Mutex<SessionStore>>,
) -> Result<(), String> {
    let mut pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;

    // If already spawned, no-op (prevents duplicate reader tasks in Strict Mode)
    if pty_mgr.is_spawned(&session_id) {
        return Ok(());
    }

    if let Err(e) = pty_mgr.spawn_prepared(&session_id, cols, rows) {
        // Spawn failed (e.g. CLI uninstalled since launch was staged): the DB
        // says "running" from launch_session — flip it so the UI doesn't show
        // a green dot for a session that never started.
        if let Ok(store) = session_state.lock() {
            let _ = store.update_status_if_running(&session_id, "exited");
        }
        let _ = app.emit("sessions-changed", ());
        return Err(e.to_string());
    }

    // Start streaming PTY output to the frontend
    if let Ok(reader) = pty_mgr.get_reader(&session_id) {
        start_reader_task(app, session_id, reader);
    }
    Ok(())
}

#[tauri::command]
pub fn list_sessions(session_state: State<'_, Mutex<SessionStore>>) -> Vec<SessionInfo> {
    let store = match session_state.lock() {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    store
        .list_sessions()
        .into_iter()
        .map(
            |(id, agent_id, agent_name, name, work_dir, status, created_at)| SessionInfo {
                id,
                agent_id,
                agent_name,
                name,
                work_dir,
                status,
                created_at,
            },
        )
        .collect()
}

#[tauri::command]
pub fn kill_session(
    session_id: String,
    pty_state: State<'_, Mutex<PtyManager>>,
    session_state: State<'_, Mutex<SessionStore>>,
    app: AppHandle,
) -> Result<(), String> {
    let mut pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;
    pty_mgr.kill(&session_id).map_err(|e| e.to_string())?;
    drop(pty_mgr);

    // Update status in DB
    let store = session_state.lock().map_err(|e| e.to_string())?;
    store
        .update_status(&session_id, "killed")
        .map_err(|e| e.to_string())?;
    drop(store);

    let _ = app.emit("sessions-changed", ());
    Ok(())
}

/// Delete a session: kill its PTY if running, remove the DB row, and delete
/// the session's temp config directory (~/.agenthub/sessions/<id>/).
#[tauri::command]
pub fn delete_session(
    session_id: String,
    pty_state: State<'_, Mutex<PtyManager>>,
    session_state: State<'_, Mutex<SessionStore>>,
    app: AppHandle,
) -> Result<(), String> {
    // 1. Kill the PTY if it's alive (also removes it from the manager)
    {
        let mut pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;
        if pty_mgr.is_spawned(&session_id) {
            pty_mgr.kill(&session_id).map_err(|e| e.to_string())?;
        } else {
            pty_mgr.remove(&session_id);
        }
    }

    // 2. Remove the DB row
    let store = session_state.lock().map_err(|e| e.to_string())?;
    store
        .delete_session(&session_id)
        .map_err(|e| e.to_string())?;
    drop(store);

    // 3. Delete the session's temp config dir
    if let Some(home) = dirs::home_dir() {
        let session_dir = home.join(".agenthub").join("sessions").join(&session_id);
        if session_dir.exists() {
            std::fs::remove_dir_all(&session_dir)
                .map_err(|e| format!("Failed to remove session dir: {}", e))?;
        }
    }

    // 4. Drop the in-memory tail buffer and its persisted scrollback snapshot
    crate::pty::stream::drop_session_buffer(&app, &session_id);
    crate::pty::scrollback::delete_snapshot(&session_id);

    let _ = app.emit("sessions-changed", ());
    Ok(())
}

/// Return the previous session's terminal output as PLAIN TEXT (ANSI stripped),
/// or None. The Terminal component writes this before attaching: TUI agents
/// clear the screen on resume, so a raw byte replay would be wiped — plain text
/// lands in the normal scrollback and survives.
#[tauri::command]
pub fn get_scrollback_snapshot(session_id: String) -> Option<String> {
    let data = crate::pty::scrollback::load_snapshot(&session_id)?;
    let text = crate::pty::scrollback::snapshot_to_plain_text(&data);
    if text.is_empty() {
        return None;
    }
    Some(base64::engine::general_purpose::STANDARD.encode(text))
}

#[tauri::command]
pub fn pty_write(
    session_id: String,
    data: String,
    pty_state: State<'_, Mutex<PtyManager>>,
) -> Result<(), String> {
    let pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;
    pty_mgr
        .write(&session_id, data.as_bytes())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_resize(
    session_id: String,
    cols: u16,
    rows: u16,
    pty_state: State<'_, Mutex<PtyManager>>,
) -> Result<(), String> {
    let pty_mgr = pty_state.lock().map_err(|e| e.to_string())?;
    pty_mgr
        .resize(&session_id, cols, rows)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn select_directory() -> Result<Option<String>, String> {
    // Placeholder — will use tauri-plugin-dialog in frontend
    Ok(None)
}
