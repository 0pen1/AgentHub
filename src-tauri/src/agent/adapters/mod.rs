pub mod claude;
pub mod codex;
pub mod gemini;
pub mod opencode;
pub mod pi;

use std::collections::HashMap;
use std::path::Path;

/// Static per-agent definition — THE single source of truth for agent
/// identity. The registry (registry.rs) probes installability from this list;
/// `get_adapter` dispatches on the same ids. Adding an agent = adding one
/// entry here + one `get_adapter` arm.
pub struct AgentDef {
    pub id: &'static str,
    pub name: &'static str,
    pub executable: &'static str,
    pub config_format: &'static str,
    pub instruction_file: Option<&'static str>,
    pub version_flag: &'static str,
}

pub const KNOWN_AGENTS: &[AgentDef] = &[
    AgentDef {
        id: "claude",
        name: "Claude Code",
        executable: "claude",
        config_format: "json",
        instruction_file: Some("CLAUDE.md"),
        version_flag: "--version",
    },
    AgentDef {
        id: "codex",
        name: "Codex CLI",
        executable: "codex",
        config_format: "toml",
        instruction_file: Some("AGENTS.md"),
        version_flag: "--version",
    },
    AgentDef {
        id: "gemini",
        name: "Gemini CLI",
        executable: "gemini",
        config_format: "json",
        instruction_file: Some("GEMINI.md"),
        version_flag: "--version",
    },
    AgentDef {
        id: "pi",
        name: "Pi",
        executable: "pi",
        config_format: "json",
        instruction_file: None,
        version_flag: "--version",
    },
    AgentDef {
        id: "opencode",
        name: "OpenCode",
        executable: "opencode",
        config_format: "jsonc",
        instruction_file: None,
        version_flag: "--version",
    },
];

/// Trait that each agent adapter must implement.
/// Handles reading existing config and writing session-specific config.
pub trait AgentAdapter {
    /// Read existing MCP servers from this agent's global config
    fn read_mcp_servers(&self) -> HashMap<String, serde_json::Value>;

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

/// Materialize `src` (file or dir) at `dst`: symlink on Unix; on other
/// platforms (where symlinks need privileges) fall back to a copy so skill
/// injection still works instead of silently no-oping under #[cfg(unix)].
pub(crate) fn link_or_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
    }
    #[cfg(not(unix))]
    {
        if src.is_dir() {
            copy_dir_recursive(src, dst)
        } else {
            std::fs::copy(src, dst).map(|_| ())
        }
    }
}

#[cfg(not(unix))]
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Registry and adapters must stay in lockstep: every KNOWN_AGENTS id
    /// resolves to an adapter. A missing arm in get_adapter would only
    /// surface at launch time ("No adapter for agent") — this test catches
    /// it at compile/CI time instead.
    #[test]
    fn every_known_agent_has_an_adapter() {
        for def in KNOWN_AGENTS {
            assert!(
                get_adapter(def.id).is_some(),
                "KNOWN_AGENTS entry '{}' has no adapter in get_adapter()",
                def.id
            );
        }
    }

    /// Adapter ids must equal their KNOWN_AGENTS entry (historically they
    /// were separate tables keyed by string; drift = silent mis-dispatch).
    #[test]
    fn known_agent_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for def in KNOWN_AGENTS {
            assert!(seen.insert(def.id), "duplicate agent id: {}", def.id);
        }
    }
}
