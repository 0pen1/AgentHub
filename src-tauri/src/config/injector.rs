use crate::agent::adapters;
use crate::config::instruction::InstructionMerger;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ConfigInjector;

impl ConfigInjector {
    pub fn new() -> Self {
        Self
    }

    /// Prepare session config: merge selected items, write to temp dir, return (env, launch args)
    pub fn prepare(
        &self,
        session_id: &str,
        agent_id: &str,
        work_dir: &str,
        skills: &[String],
        mcps: &[String],
        instructions: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>> {
        let adapter = adapters::get_adapter(agent_id)
            .ok_or_else(|| format!("No adapter for agent: {}", agent_id))?;

        // Create session directory
        let session_dir = self.session_dir(session_id);
        std::fs::create_dir_all(&session_dir)?;

        // Collect MCP configs filtered by selected names
        let all_mcps = adapter.read_mcp_servers();
        let mut selected_mcps: HashMap<String, serde_json::Value> = all_mcps
            .into_iter()
            .filter(|(name, _)| mcps.contains(name))
            .collect();
        // Managed (library) MCP entries override same-name scanned entries —
        // the user checked the 托管 entry shown in the picker, so its config
        // is the one that must be injected.
        for (name, value) in crate::config::library::load_library_mcps() {
            if mcps.contains(&name) {
                selected_mcps.insert(name, value);
            }
        }

        // Merge instruction files
        let merger = InstructionMerger::new();
        let instruction_content = merger.merge(instructions);

        // Resolve skill paths
        let skill_dirs = self.resolve_skill_paths(skills);

        // Write session config using the adapter
        let work_path = PathBuf::from(work_dir);
        let (env_overrides, mut launch_args) = adapter.write_session_config(
            &session_dir,
            &work_path,
            &selected_mcps,
            &instruction_content,
            &skill_dirs,
        )?;

        // Continuity: pin the AgentHub session id as claude's conversation id.
        // If a history file for this id exists, resume it (真实"继续上次对话");
        // otherwise start fresh but pin the id (--session-id) so FUTURE restarts
        // can resume. Covers sessions created before this feature existed.
        // (Only claude supports this today; other adapters start fresh.)
        if agent_id == "claude" {
            if claude_history_exists(work_dir, session_id) {
                launch_args.push("--resume".to_string());
                launch_args.push(session_id.to_string());
            } else {
                launch_args.push("--session-id".to_string());
                launch_args.push(session_id.to_string());
            }
        }

        Ok((env_overrides, launch_args))
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".agenthub")
            .join("sessions")
            .join(session_id)
    }

    fn resolve_skill_paths(&self, skill_names: &[String]) -> Vec<String> {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return vec![],
        };

        let search_dirs = crate::config::skill::skill_search_dirs(&home);

        let mut result = Vec::new();
        for name in skill_names {
            for (dir, _source) in &search_dirs {
                let skill_dir = dir.join(name);
                if skill_dir.join("SKILL.md").exists() {
                    result.push(skill_dir.to_string_lossy().to_string());
                    break;
                }
            }
        }
        result
    }
}

/// Check whether Claude Code has a conversation history file for this session id
/// in the given work_dir. Claude stores transcripts at
/// `~/.claude/projects/<munged-work-dir>/<session-id>.jsonl`, where the project
/// directory name is the absolute work_dir path with every non-alphanumeric
/// character replaced by '-'.
fn claude_history_exists(work_dir: &str, session_id: &str) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let munged: String = work_dir
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let history = home
        .join(".claude")
        .join("projects")
        .join(munged)
        .join(format!("{}.jsonl", session_id));
    history.exists()
}
