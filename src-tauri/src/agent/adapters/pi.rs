use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct PiAdapter;

impl AgentAdapter for PiAdapter {
    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }

    /// Session-isolated via PI_CODING_AGENT_DIR: pi resolves its whole global
    /// config dir (skills, settings, sessions) from this env var, so nothing
    /// is written into the project's .pi/ directory (the old behavior
    /// symlinked selected skills into <work_dir>/.pi/skills and left them
    /// there after the session ended).
    ///
    /// The session agent dir gets a settings.json with `skills: [...]`
    /// pointing at the selected skill dirs — pi loads skill paths from
    /// settings directly (docs: skills.md), no symlinks needed.
    fn write_session_config(
        &self,
        session_dir: &Path,
        _work_dir: &Path,
        _mcps: &HashMap<String, serde_json::Value>,
        _instruction_content: &str,
        skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let agent_dir = session_dir.join("pi_agent");
        std::fs::create_dir_all(&agent_dir)?;

        if !skill_paths.is_empty() {
            let settings = serde_json::json!({ "skills": skill_paths });
            std::fs::write(
                agent_dir.join("settings.json"),
                serde_json::to_string_pretty(&settings)?,
            )?;
        }

        let mut env_overrides = HashMap::new();
        env_overrides.insert(
            "PI_CODING_AGENT_DIR".to_string(),
            agent_dir.to_string_lossy().to_string(),
        );

        Ok((env_overrides, Vec::new()))
    }
}
