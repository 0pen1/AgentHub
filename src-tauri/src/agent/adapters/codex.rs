use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
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
            // Auth must live inside CODEX_HOME or every session starts logged
            // out (verified: `CODEX_HOME=<empty> codex login status` → "Not
            // logged in"). Symlink keeps it live across re-logins on Unix;
            // other platforms (no symlink privileges) get a one-shot copy.
            let auth_src = home.join(".codex").join("auth.json");
            if auth_src.exists() {
                let auth_dst = codex_home.join("auth.json");
                if !auth_dst.exists() {
                    super::link_or_copy(&auth_src, &auth_dst)?;
                }
            }
        }

        // Merge MCP servers into the copied config.toml structurally (not by
        // text-append: a name already present in the global config would
        // produce a duplicate [mcp_servers.x] header and break TOML parsing).
        if !mcps.is_empty() {
            let config_path = codex_home.join("config.toml");
            let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
            let mut doc: toml::Value = existing.parse().unwrap_or(toml::Value::Table(Default::default()));
            if !doc.is_table() {
                doc = toml::Value::Table(Default::default());
            }
            {
                let servers = doc
                    .as_table_mut()
                    .unwrap()
                    .entry("mcp_servers")
                    .or_insert_with(|| toml::Value::Table(Default::default()))
                    .as_table_mut()
                    .ok_or("config.toml: mcp_servers is not a table")?;
                for (name, value) in mcps {
                    servers.insert(name.clone(), json_to_toml(value));
                }
            }
            std::fs::write(&config_path, toml::to_string_pretty(&doc)?)?;
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
                    super::link_or_copy(skill_src, &skill_dst)?;
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

/// Convert a JSON MCP server definition to a TOML value for config.toml.
/// Handles the full JSON value tree, so nested objects (e.g. env maps) and
/// non-string array items survive the round-trip.
pub fn json_to_toml(val: &serde_json::Value) -> toml::Value {
    match val {
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else {
                toml::Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        serde_json::Value::Array(arr) => toml::Value::Array(arr.iter().map(json_to_toml).collect()),
        serde_json::Value::Object(map) => {
            let table: toml::map::Map<String, toml::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_toml(v)))
                .collect();
            toml::Value::Table(table)
        }
        serde_json::Value::Null => toml::Value::String(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_map(pairs: &[(&str, serde_json::Value)]) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        serde_json::Value::Object(map)
    }

    /// Regression: an MCP name already present in the global config must be
    /// merged in place, not appended — a duplicate [mcp_servers.x] header
    /// made the whole config.toml unparseable and codex failed to start.
    #[test]
    fn merges_mcp_over_existing_name() {
        let global = "[mcp_servers.existing]\ncommand = \"old\"\n\n[mcp_servers.other]\ncommand = \"keep\"\n";
        let doc: toml::Value = global.parse().unwrap();

        let mut mcps = HashMap::new();
        mcps.insert(
            "existing".to_string(),
            json_map(&[("command", serde_json::json!("new"))]),
        );

        // Same merge logic as write_session_config (extracted inline here to
        // keep the test independent of $HOME).
        let mut doc = doc;
        let servers = doc
            .as_table_mut()
            .unwrap()
            .entry("mcp_servers")
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .unwrap();
        for (name, value) in &mcps {
            servers.insert(name.clone(), json_to_toml(value));
        }
        let out = toml::to_string_pretty(&doc).unwrap();

        // Parses cleanly and exactly once per header.
        let reparsed: toml::Value = out.parse().unwrap();
        assert_eq!(
            reparsed["mcp_servers"]["existing"]["command"].as_str(),
            Some("new")
        );
        assert_eq!(
            reparsed["mcp_servers"]["other"]["command"].as_str(),
            Some("keep")
        );
        assert_eq!(out.matches("[mcp_servers.existing]").count(), 1);
    }

    #[test]
    fn json_to_toml_preserves_nested_env() {
        let input = json_map(&[
            ("command", serde_json::json!("npx")),
            ("args", serde_json::json!(["-y", "some-server"])),
            (
                "env",
                json_map(&[("API_KEY", serde_json::json!("secret"))]),
            ),
            ("enabled", serde_json::json!(true)),
        ]);
        let toml_val = json_to_toml(&input);
        let text = toml::to_string(&toml_val).unwrap();
        let back: toml::Value = text.parse().unwrap();
        assert_eq!(back["env"]["API_KEY"].as_str(), Some("secret"));
        assert_eq!(back["args"][1].as_str(), Some("some-server"));
        assert_eq!(back["enabled"].as_bool(), Some(true));
    }

    /// auth.json must end up inside the session CODEX_HOME or codex reports
    /// "Not logged in" (verified against `codex login status`). Note: the
    /// adapter reads auth from the REAL home (dirs::home_dir), so this test
    /// only passes on machines with ~/.codex/auth.json present — acceptable
    /// for a dev-machine test suite.
    #[test]
    fn auth_json_is_brought_into_session_home() {
        let real_auth = dirs::home_dir().expect("no home dir").join(".codex").join("auth.json");
        if !real_auth.exists() {
            // No auth on this machine — nothing to bring in; behavior is then
            // "session also has none", which is correct but untestable here.
            return;
        }

        let tmp = std::env::temp_dir().join(format!("agenthub-test-{}", uuid::Uuid::new_v4()));
        let session_dir = tmp.join("session");
        std::fs::create_dir_all(&session_dir).unwrap();

        let adapter = CodexAdapter;
        let (env, _) = adapter
            .write_session_config(&session_dir, &tmp, &HashMap::new(), "", &[])
            .unwrap();

        let codex_home = std::path::PathBuf::from(env["CODEX_HOME"].clone());
        let auth = codex_home.join("auth.json");
        assert!(
            auth.exists(),
            "auth.json missing from session CODEX_HOME (codex would start logged out)"
        );
        // Symlink (unix) or copy (other) — either way contents must match the
        // real credential file.
        assert_eq!(
            std::fs::read_to_string(&auth).unwrap(),
            std::fs::read_to_string(&real_auth).unwrap()
        );

        std::fs::remove_dir_all(&tmp).ok();
    }
}
