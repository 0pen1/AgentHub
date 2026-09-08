use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct OpenCodeAdapter;

impl AgentAdapter for OpenCodeAdapter {
    fn id(&self) -> &str {
        "opencode"
    }

    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value> {
        let config_path =
            dirs::home_dir().map(|h| h.join(".config").join("opencode").join("opencode.json"));

        if let Some(path) = config_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(servers) = json.get("mcp").and_then(|v| v.as_object()) {
                        return servers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    }
                }
            }
        }
        HashMap::new()
    }

    fn skill_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            for dir in &[
                ".config/opencode/skills",
                ".agents/skills",
                ".claude/skills",
            ] {
                let p = home.join(dir);
                if p.exists() {
                    paths.push(p.to_string_lossy().to_string());
                }
            }
        }
        paths
    }

    fn global_config_path(&self) -> Option<String> {
        dirs::home_dir().map(|h| {
            h.join(".config")
                .join("opencode")
                .join("opencode.json")
                .to_string_lossy()
                .to_string()
        })
    }

    fn project_config_dir(&self) -> &str {
        ".opencode"
    }

    fn instruction_filename(&self) -> Option<&str> {
        None
    }

    fn write_session_config(
        &self,
        session_dir: &Path,
        _work_dir: &Path,
        mcps: &HashMap<String, serde_json::Value>,
        instruction_content: &str,
        _skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let mut env_overrides = HashMap::new();

        let config_path = session_dir.join("opencode.json");

        // Read existing global config as base
        let mut config: serde_json::Value = if let Some(home) = dirs::home_dir() {
            let src = home.join(".config").join("opencode").join("opencode.json");
            if src.exists() {
                serde_json::from_str(&std::fs::read_to_string(&src)?)?
            } else {
                serde_json::json!({})
            }
        } else {
            serde_json::json!({})
        };

        if !mcps.is_empty() {
            if let Some(obj) = config.as_object_mut() {
                let mcp_obj = obj.entry("mcp").or_insert_with(|| serde_json::json!({}));
                if let Some(mcp_map) = mcp_obj.as_object_mut() {
                    for (name, value) in mcps {
                        mcp_map.insert(name.clone(), value.clone());
                    }
                }
            }
        }

        if !instruction_content.is_empty() {
            let inst_path = session_dir.join("instructions.md");
            std::fs::write(&inst_path, instruction_content)?;

            if let Some(obj) = config.as_object_mut() {
                let instructions = obj
                    .entry("instructions")
                    .or_insert_with(|| serde_json::json!([]));
                if let Some(arr) = instructions.as_array_mut() {
                    arr.push(serde_json::Value::String(
                        inst_path.to_string_lossy().to_string(),
                    ));
                }
            }
        }

        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        env_overrides.insert(
            "OPENCODE_CONFIG".to_string(),
            config_path.to_string_lossy().to_string(),
        );

        Ok((env_overrides, Vec::new()))
    }
}
