pub mod claude;
pub mod codex;
pub mod gemini;
pub mod opencode;
pub mod pi;

use std::collections::HashMap;
use std::path::Path;

/// Trait that each agent adapter must implement.
/// Handles reading existing config and writing session-specific config.
pub trait AgentAdapter {
    /// Agent identifier
    fn id(&self) -> &str;

    /// Read existing MCP servers from this agent's global config
    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value>;

    /// Read existing skills paths
    fn skill_paths(&self) -> Vec<String>;

    /// Global config file path
    fn global_config_path(&self) -> Option<String>;

    /// Project config directory name (e.g., ".claude", ".codex")
    fn project_config_dir(&self) -> &str;

    /// Instruction file name (e.g., "CLAUDE.md", "AGENTS.md")
    fn instruction_filename(&self) -> Option<&str>;

    /// Write merged config for a session.
    /// Returns (env_overrides, launch_args) to use when spawning the agent CLI.
    fn write_session_config(
        &self,
        session_dir: &Path,
        work_dir: &Path,
        mcps: &HashMap<String, serde_json::Value>,
        instruction_content: &str,
        skill_paths: &[String],
    ) -> Result<(HashMap<String, String>, Vec<String>), Box<dyn std::error::Error>>;
}

pub fn get_adapter(agent_id: &str) -> Option<Box<dyn AgentAdapter>> {
    match agent_id {
        "claude" => Some(Box::new(claude::ClaudeAdapter)),
        "codex" => Some(Box::new(codex::CodexAdapter)),
        "gemini" => Some(Box::new(gemini::GeminiAdapter)),
        "pi" => Some(Box::new(pi::PiAdapter)),
        "opencode" => Some(Box::new(opencode::OpenCodeAdapter)),
        _ => None,
    }
}
