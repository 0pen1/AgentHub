use super::adapters::KNOWN_AGENTS;
use crate::commands::AgentInfo;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct AgentRegistry;

impl AgentRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_installed(&self) -> Vec<AgentInfo> {
        KNOWN_AGENTS
            .iter()
            .map(|def| {
                let (installed, version) =
                    check_installed_cached_full(def.executable, def.version_flag);
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

    /// Same as `get_agent` but returns only the installed check result — used
    /// on the launch path where version metadata is irrelevant.
    pub fn is_agent_installed(&self, id: &str) -> bool {
        KNOWN_AGENTS
            .iter()
            .find(|def| def.id == id)
            .map(|def| check_installed_cached_full(def.executable, def.version_flag).0)
            .unwrap_or(false)
    }

    pub fn get_agent(&self, id: &str) -> Option<AgentInfo> {
        self.scan_installed().into_iter().find(|a| a.id == id)
    }
}

// --- Install probe cache ----------------------------------------------------
//
// Probing an agent = one `which` lookup + one `<exe> --version` child process.
// Several CLIs take hundreds of ms on first run, and AgentHub probes on every
// Launcher load AND every launch (get_agent). A short TTL cache keeps
// launch/setup flows fast while still noticing newly installed/removed CLIs
// within a minute.

const PROBE_TTL: Duration = Duration::from_secs(60);

/// (installed, version, probed_at) per executable.
static PROBE_CACHE: Mutex<Option<std::collections::HashMap<&'static str, CachedProbe>>> =
    Mutex::new(None);

struct CachedProbe {
    installed: bool,
    version: Option<String>,
    at: Instant,
}

/// Cache lookup keyed by (executable, version_flag).
fn check_installed_cached_full(
    executable: &'static str,
    version_flag: &'static str,
) -> (bool, Option<String>) {
    let mut guard = PROBE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(Default::default);
    if let Some(hit) = map.get(executable) {
        if hit.at.elapsed() < PROBE_TTL {
            return (hit.installed, hit.version.clone());
        }
    }
    let (installed, version) = check_installed_full(executable, version_flag);
    map.insert(
        executable,
        CachedProbe {
            installed,
            version: version.clone(),
            at: Instant::now(),
        },
    );
    (installed, version)
}

fn check_installed_full(executable: &str, version_flag: &str) -> (bool, Option<String>) {
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
