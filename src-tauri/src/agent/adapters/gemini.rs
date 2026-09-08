use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct GeminiAdapter;

impl AgentAdapter for GeminiAdapter {
    fn id(&self) -> &str {
        "gemini"
    }

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

    fn skill_paths(&self) -> Vec<String> {
        vec![]
    }

    fn global_config_path(&self) -> Option<String> {
        dirs::home_dir().map(|h| {
            h.join(".gemini")
                .join("settings.json")
                .to_string_lossy()
                .to_string()
        })
    }

    fn project_config_dir(&self) -> &str {
        ".gemini"
    }

    fn instruction_filename(&self) -> Option<&str> {
        Some("GEMINI.md")
    }

    fn write_session_config(
        &self,
        _session_dir: &Path,
        work_dir: &Path,
        mcps: &HashMap<String, serde_json::Value>,
        instruction_content: &str,
        _skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let config_dir = work_dir.join(".gemini");
        std::fs::create_dir_all(&config_dir)?;

        if !mcps.is_empty() {
            let config_path = config_dir.join("settings.json");
            let mut config: serde_json::Value = if config_path.exists() {
                serde_json::from_str(&std::fs::read_to_string(&config_path)?)?
            } else {
                serde_json::json!({})
            };
            if let Some(obj) = config.as_object_mut() {
                let mcp_obj = obj
                    .entry("mcpServers")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(mcp_map) = mcp_obj.as_object_mut() {
                    for (name, value) in mcps {
                        mcp_map.insert(name.clone(), value.clone());
                    }
                }
            }
            std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        }

        if !instruction_content.is_empty() {
            std::fs::write(work_dir.join("GEMINI.md"), instruction_content)?;
        }

        Ok((HashMap::new(), Vec::new()))
    }
}
