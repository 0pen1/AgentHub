use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn id(&self) -> &str {
        "codex"
    }

    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value> {
        let config_path = dirs::home_dir().map(|h| h.join(".codex").join("config.toml"));

        if let Some(path) = config_path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(toml_val) = content.parse::<toml::Value>() {
                    if let Some(servers) =
                        toml_val.get("mcp_servers").and_then(|v| v.as_table())
                    {
                        return servers
                            .iter()
                            .map(|(k, v)| {
                                let json_val = toml_to_json(v);
                                (k.clone(), json_val)
                            })
                            .collect();
                    }
                }
            }
        }
        HashMap::new()
    }

    fn skill_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let global_skills = home.join(".codex").join("skills");
            if global_skills.exists() {
                paths.push(global_skills.to_string_lossy().to_string());
            }
        }
        paths
    }

    fn global_config_path(&self) -> Option<String> {
        dirs::home_dir().map(|h| {
            h.join(".codex")
                .join("config.toml")
                .to_string_lossy()
                .to_string()
        })
    }

    fn project_config_dir(&self) -> &str {
        ".codex"
    }

    fn instruction_filename(&self) -> Option<&str> {
        Some("AGENTS.md")
    }

    fn write_session_config(
        &self,
        session_dir: &Path,
        _work_dir: &Path,
        mcps: &HashMap<String, serde_json::Value>,
        instruction_content: &str,
        skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let mut env_overrides = HashMap::new();

        let codex_home = session_dir.join("codex_home");
        std::fs::create_dir_all(&codex_home)?;

        // Copy existing global config as base
        if let Some(home) = dirs::home_dir() {
            let src = home.join(".codex").join("config.toml");
            if src.exists() {
                std::fs::copy(&src, codex_home.join("config.toml"))?;
            }
        }

        // Append MCP servers
        if !mcps.is_empty() {
            let config_path = codex_home.join("config.toml");
            let mut content = std::fs::read_to_string(&config_path).unwrap_or_default();

            for (name, value) in mcps {
                content.push_str(&format!("\n[mcp_servers.{}]\n", name));
                if let Some(obj) = value.as_object() {
                    for (k, v) in obj {
                        match v {
                            serde_json::Value::String(s) => {
                                content.push_str(&format!("{} = {}\n", k, toml_inline_str(s)));
                            }
                            serde_json::Value::Array(arr) => {
                                let items: Vec<String> = arr
                                    .iter()
                                    .filter_map(|item| {
                                        item.as_str().map(|s| toml_inline_str(s))
                                    })
                                    .collect();
                                content
                                    .push_str(&format!("{} = [{}]\n", k, items.join(", ")));
                            }
                            serde_json::Value::Bool(b) => {
                                content.push_str(&format!("{} = {}\n", k, b));
                            }
                            // Nested objects (e.g. env = { KEY = "value" }) must be
                            // emitted as inline tables — silently dropping them
                            // breaks MCP servers that need env vars.
                            serde_json::Value::Object(map) => {
                                let pairs: Vec<String> = map
                                    .iter()
                                    .filter_map(|(ek, ev)| {
                                        ev.as_str().map(|s| {
                                            format!("{} = {}", ek, toml_inline_str(s))
                                        })
                                    })
                                    .collect();
                                if !pairs.is_empty() {
                                    content
                                        .push_str(&format!("{} = {{ {} }}\n", k, pairs.join(", ")));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            std::fs::write(&config_path, content)?;
        }

        if !instruction_content.is_empty() {
            std::fs::write(codex_home.join("AGENTS.md"), instruction_content)?;
        }

        // Symlink skills
        let skills_dir = codex_home.join("skills");
        std::fs::create_dir_all(&skills_dir)?;
        for skill_path in skill_paths {
            let skill_src = Path::new(skill_path);
            if let Some(skill_name) = skill_src.file_name() {
                let skill_dst = skills_dir.join(skill_name);
                if !skill_dst.exists() {
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(skill_src, &skill_dst)?;
                }
            }
        }

        env_overrides.insert(
            "CODEX_HOME".to_string(),
            codex_home.to_string_lossy().to_string(),
        );

        Ok((env_overrides, Vec::new()))
    }
}

pub fn toml_to_json(val: &toml::Value) -> serde_json::Value {
    match val {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(i),
        toml::Value::Float(f) => serde_json::json!(f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
    }
}

/// Escape a string for a TOML basic (single-line) string literal.
fn toml_inline_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
