use super::AgentAdapter;
use std::collections::HashMap;
use std::path::Path;

pub struct PiAdapter;

impl AgentAdapter for PiAdapter {
    fn id(&self) -> &str {
        "pi"
    }

    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }

    fn skill_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            for dir in &[".pi/agent/skills", ".agents/skills", ".claude/skills"] {
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
            h.join(".pi")
                .join("agent")
                .join("settings.json")
                .to_string_lossy()
                .to_string()
        })
    }

    fn project_config_dir(&self) -> &str {
        ".pi"
    }

    fn instruction_filename(&self) -> Option<&str> {
        None
    }

    fn write_session_config(
        &self,
        _session_dir: &Path,
        work_dir: &Path,
        _mcps: &HashMap<String, serde_json::Value>,
        _instruction_content: &str,
        skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let skills_dir = work_dir.join(".pi").join("skills");
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

        Ok((HashMap::new(), Vec::new()))
    }
}
