use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct ClaudeAdapter;

impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> &str {
        "claude"
    }

    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value> {
        // Claude Code stores global MCP servers in ~/.claude.json (NOT settings.json)
        let mut result = HashMap::new();

        if let Some(home) = dirs::home_dir() {
            let claude_json = home.join(".claude.json");
            if let Ok(content) = std::fs::read_to_string(&claude_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
                        for (k, v) in servers {
                            result.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            // Also check ~/.claude/settings.json (older location, still supported)
            let settings = home.join(".claude").join("settings.json");
            if let Ok(content) = std::fs::read_to_string(&settings) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
                        for (k, v) in servers {
                            result.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                    }
                }
            }
        }

        result
    }

    fn skill_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let global_skills = home.join(".claude").join("skills");
            if global_skills.exists() {
                paths.push(global_skills.to_string_lossy().to_string());
            }
        }
        paths
    }

    fn global_config_path(&self) -> Option<String> {
        dirs::home_dir().map(|h| {
            h.join(".claude")
                .join("settings.json")
                .to_string_lossy()
                .to_string()
        })
    }

    fn project_config_dir(&self) -> &str {
        ".claude"
    }

    fn instruction_filename(&self) -> Option<&str> {
        Some("CLAUDE.md")
    }

    fn write_session_config(
        &self,
        session_dir: &Path,
        work_dir: &Path,
        mcps: &HashMap<String, serde_json::Value>,
        instruction_content: &str,
        skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let config_dir = work_dir.join(".claude");
        std::fs::create_dir_all(&config_dir)?;

        // Write session-scoped MCP config (only selected servers; --strict-mcp-config
        // makes Claude ignore global ~/.claude.json and project .mcp.json entirely)
        let mcp_config_path = session_dir.join("mcp.json");
        let mcp_config = serde_json::json!({ "mcpServers": mcps });
        std::fs::write(&mcp_config_path, serde_json::to_string_pretty(&mcp_config)?)?;

        // Build a session plugin dir containing ONLY the selected skills.
        // --plugin-dir loads skills from <plugin>/skills/<name>/SKILL.md (symlinks OK).
        // This avoids touching the project's .claude/skills (no pollution) and, combined
        // with --setting-sources project,local, keeps global ~/.claude/skills out.
        let mut args = vec![
            "--strict-mcp-config".to_string(),
            "--mcp-config".to_string(),
            mcp_config_path.to_string_lossy().to_string(),
        ];

        if !skill_paths.is_empty() {
            let plugin_dir = session_dir.join("skills-plugin");
            let plugin_skills_dir = plugin_dir.join("skills");
            std::fs::create_dir_all(&plugin_skills_dir)?;
            let plugin_json_dir = plugin_dir.join(".claude-plugin");
            std::fs::create_dir_all(&plugin_json_dir)?;
            std::fs::write(
                plugin_json_dir.join("plugin.json"),
                r#"{"name": "agenthub-session"}"#,
            )?;
            for skill_path in skill_paths {
                let skill_src = std::path::Path::new(skill_path);
                if let Some(skill_name) = skill_src.file_name() {
                    let skill_dst = plugin_skills_dir.join(skill_name);
                    if !skill_dst.exists() {
                        #[cfg(unix)]
                        std::os::unix::fs::symlink(skill_src, &skill_dst)?;
                    }
                }
            }
            args.push("--plugin-dir".to_string());
            args.push(plugin_dir.to_string_lossy().to_string());
        }

        // Exclude the "user" settings source: without it, Claude auto-loads ALL global
        // ~/.claude/skills regardless of selection. project+local keeps project-level
        // .claude/settings.json, .claude/skills and CLAUDE.md working normally.
        args.push("--setting-sources".to_string());
        args.push("project,local".to_string());

        // Re-inject pieces of the user's global settings.json that --setting-sources
        // no longer loads: env (auth tokens, base URL, model defaults) and
        // enabledPlugins (otherwise user-installed plugins would vanish).
        let global_settings = self.read_global_settings();
        let has_env = global_settings
            .get("env")
            .and_then(|v| v.as_object())
            .map(|o| !o.is_empty())
            .unwrap_or(false);
        let has_plugins = global_settings
            .get("enabledPlugins")
            .and_then(|v| v.as_object())
            .map(|o| !o.is_empty())
            .unwrap_or(false);
        if has_env || has_plugins {
            let settings_path = session_dir.join("settings.json");
            let mut extra = serde_json::Map::new();
            if let Some(env) = global_settings.get("env") {
                extra.insert("env".to_string(), env.clone());
            }
            if let Some(plugins) = global_settings.get("enabledPlugins") {
                extra.insert("enabledPlugins".to_string(), plugins.clone());
            }
            std::fs::write(
                &settings_path,
                serde_json::to_string_pretty(&serde_json::Value::Object(extra))?,
            )?;
            args.push("--settings".to_string());
            args.push(settings_path.to_string_lossy().to_string());
        }

        // Inject instructions into work_dir/CLAUDE.md inside a marked block.
        // Marker-based so re-injection (restart, relaunch in same dir) REPLACES
        // the previous block instead of appending duplicates.
        if !instruction_content.is_empty() {
            let instruction_path = work_dir.join("CLAUDE.md");
            let existing = std::fs::read_to_string(&instruction_path).unwrap_or_default();
            let cleaned = strip_agenthub_block(&existing);
            let combined = if cleaned.trim().is_empty() {
                format!("{}\n{}\n{}\n", BLOCK_START, instruction_content.trim(), BLOCK_END)
            } else {
                format!(
                    "{}\n\n{}\n{}\n{}\n",
                    cleaned.trim_end(),
                    BLOCK_START,
                    instruction_content.trim(),
                    BLOCK_END
                )
            };
            std::fs::write(&instruction_path, combined)?;
        }

        Ok((HashMap::new(), args))
    }
}

impl ClaudeAdapter {
    /// Read the user's global ~/.claude/settings.json. Needed because
    /// --setting-sources project,local drops the user source, which would
    /// otherwise lose env (auth/model config) and enabledPlugins.
    fn read_global_settings(&self) -> serde_json::Value {
        if let Some(home) = dirs::home_dir() {
            let settings = home.join(".claude").join("settings.json");
            if let Ok(content) = std::fs::read_to_string(&settings) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    return json;
                }
            }
        }
        serde_json::json!({})
    }
}

const BLOCK_START: &str = "<!-- agenthub:session-instructions:start -->";
const BLOCK_END: &str = "<!-- agenthub:session-instructions:end -->";

/// Remove a previously injected AgentHub instruction block from CLAUDE.md content
fn strip_agenthub_block(content: &str) -> String {
    match (content.find(BLOCK_START), content.find(BLOCK_END)) {
        (Some(start), Some(end)) if end >= start => {
            let head = &content[..start];
            let tail = &content[end + BLOCK_END.len()..];
            // Our block is always written at the end, so tail is usually just a newline
            format!("{}{}", head.trim_end(), tail)
        }
        _ => content.to_string(),
    }
}
