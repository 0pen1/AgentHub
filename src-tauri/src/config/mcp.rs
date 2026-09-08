use crate::agent::adapters;
use crate::commands::McpServerInfo;
use crate::config::library::{self, LIBRARY_SOURCE};

pub struct McpScanner;

impl McpScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_all(&self) -> Vec<McpServerInfo> {
        let mut servers = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Library (managed) entries first so a same-name managed entry
        // shadows the scanned original — first-wins dedup below.
        for (name, value) in library::load_library_mcps() {
            if seen.insert(name.clone()) {
                servers.push(library::mcp_info_from_value(name, &value, LIBRARY_SOURCE));
            }
        }

        for agent_id in &["claude", "codex", "gemini", "pi", "opencode"] {
            if let Some(adapter) = adapters::get_adapter(agent_id) {
                let agent_servers = adapter.read_mcp_servers();
                for (name, value) in agent_servers {
                    if seen.insert(name.clone()) {
                        servers.push(library::mcp_info_from_value(name, &value, agent_id));
                    }
                }
            }
        }

        servers
    }
}
