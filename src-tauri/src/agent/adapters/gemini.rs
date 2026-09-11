use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct GeminiAdapter;

impl GeminiAdapter {
    /// Instructions injected via the session settings file's
    /// context.fileName — avoids writing GEMINI.md into the project, which
    /// would overwrite the user's own file (the old behavior silently
    /// clobbered it on every launch).
    fn instruction_content(&self, session_dir: &Path, instruction_content: &str) -> Option<String> {
        if instruction_content.trim().is_empty() {
            return None;
        }
        let path = session_dir.join("instructions.md");
        std::fs::write(&path, instruction_content).ok()?;
        Some(path.to_string_lossy().to_string())
    }
}

impl AgentAdapter for GeminiAdapter {
    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value> {
        let config_path = dirs::home_dir().map(|h| h.join(".gemini").join("settings.json"));

        if let Some(path) = config_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
                        return servers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    }
                }
            }
        }
        HashMap::new()
    }

    /// Session-isolated config: nothing is written into the project dir.
    ///
    /// - MCP servers go into a session `settings.json` surfaced via
    ///   GEMINI_CLI_SYSTEM_SETTINGS_PATH. Per Gemini CLI docs, system settings
    ///   merge with user/workspace settings and win on same-name mcpServers,
    ///   so selected (managed) entries override the user's global config —
    ///   while everything else keeps working untouched.
    /// - Selected MCP names are NOT readable later for restart-time overrides,
    ///   so restart re-runs this whole path with the same selection — same
    ///   session file, always overwritten cleanly.
    /// - Instructions land in a session file wired through
    ///   context.fileName (project GEMINI.md stays owned by the user).
    ///   Returns the system-settings path as the env override.
    fn write_session_config(
        &self,
        session_dir: &Path,
        _work_dir: &Path,
        mcps: &HashMap<String, serde_json::Value>,
        instruction_content: &str,
        _skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        // Start from the user's global settings as base so auth/UI preferences
        // carry over; the system-settings layer only needs to carry what we
        // inject (mcpServers + context.fileName), but a full base keeps the
        // session visually identical to a normal launch.
        let mut config: serde_json::Value = if let Some(home) = dirs::home_dir() {
            let src = home.join(".gemini").join("settings.json");
            if src.exists() {
                serde_json::from_str(&std::fs::read_to_string(&src)?)?
            } else {
                serde_json::json!({})
            }
        } else {
            serde_json::json!({})
        };

        if let Some(obj) = config.as_object_mut() {
            if !mcps.is_empty() {
                let mcp_obj = obj
                    .entry("mcpServers")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(mcp_map) = mcp_obj.as_object_mut() {
                    for (name, value) in mcps {
                        mcp_map.insert(name.clone(), value.clone());
                    }
                }
            }
            if let Some(file_name) = self.instruction_content(session_dir, instruction_content) {
                // context.fileName merges: project GEMINI.md (workspace layer)
                // plus our session file (system layer).
                let ctx = obj
                    .entry("context")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(ctx_map) = ctx.as_object_mut() {
                    ctx_map.insert(
                        "fileName".to_string(),
                        serde_json::Value::String(file_name),
                    );
                }
            }
        }

        let config_path = session_dir.join("gemini-settings.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

        let mut env_overrides = HashMap::new();
        env_overrides.insert(
            "GEMINI_CLI_SYSTEM_SETTINGS_PATH".to_string(),
            config_path.to_string_lossy().to_string(),
        );

        Ok((env_overrides, Vec::new()))
    }
}
