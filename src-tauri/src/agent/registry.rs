use crate::commands::AgentInfo;
use std::process::Command;

pub struct AgentRegistry;

struct AgentDefinition {
    id: &'static str,
    name: &'static str,
    executable: &'static str,
    config_format: &'static str,
    instruction_file: Option<&'static str>,
    version_flag: &'static str,
}

const KNOWN_AGENTS: &[AgentDefinition] = &[
    AgentDefinition {
        id: "claude",
        name: "Claude Code",
        executable: "claude",
        config_format: "json",
        instruction_file: Some("CLAUDE.md"),
        version_flag: "--version",
    },
    AgentDefinition {
        id: "codex",
        name: "Codex CLI",
        executable: "codex",
        config_format: "toml",
        instruction_file: Some("AGENTS.md"),
        version_flag: "--version",
    },
    AgentDefinition {
        id: "gemini",
        name: "Gemini CLI",
        executable: "gemini",
        config_format: "json",
        instruction_file: Some("GEMINI.md"),
        version_flag: "--version",
    },
    AgentDefinition {
        id: "pi",
        name: "Pi",
        executable: "pi",
        config_format: "json",
        instruction_file: None,
        version_flag: "--version",
    },
    AgentDefinition {
        id: "opencode",
        name: "OpenCode",
        executable: "opencode",
        config_format: "jsonc",
        instruction_file: None,
        version_flag: "--version",
    },
];

impl AgentRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_installed(&self) -> Vec<AgentInfo> {
        KNOWN_AGENTS
            .iter()
            .map(|def| {
                let (installed, version) = self.check_installed(def.executable, def.version_flag);
                AgentInfo {
                    id: def.id.to_string(),
                    name: def.name.to_string(),
                    executable: def.executable.to_string(),
                    version,
                    installed,
                    config_format: def.config_format.to_string(),
                    instruction_file: def.instruction_file.map(|s| s.to_string()),
                }
            })
            .collect()
    }

    pub fn get_agent(&self, id: &str) -> Option<AgentInfo> {
        self.scan_installed().into_iter().find(|a| a.id == id)
    }

    fn check_installed(&self, executable: &str, version_flag: &str) -> (bool, Option<String>) {
        match which::which(executable) {
            Ok(_path) => {
                let version = Command::new(executable)
                    .arg(version_flag)
                    .output()
                    .ok()
                    .and_then(|output| {
                        if output.status.success() {
                            let stdout =
                                String::from_utf8_lossy(&output.stdout).trim().to_string();
                            let version_str = stdout
                                .lines()
                                .next()
                                .unwrap_or(&stdout)
                                .trim()
                                .trim_start_matches(|c: char| !c.is_ascii_digit())
                                .to_string();
                            if version_str.is_empty() {
                                None
                            } else {
                                Some(version_str)
                            }
                        } else {
                            None
                        }
                    });
                (true, version)
            }
            Err(_) => (false, None),
        }
    }
}
