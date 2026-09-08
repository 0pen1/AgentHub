/// Detect agent session status from PTY output patterns
#[allow(dead_code)]
pub enum AgentStatus {
    Running,
    Waiting,
    Idle,
    Error,
    Stopped,
}

#[allow(dead_code)]
impl AgentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            AgentStatus::Running => "running",
            AgentStatus::Waiting => "waiting",
            AgentStatus::Idle => "idle",
            AgentStatus::Error => "error",
            AgentStatus::Stopped => "stopped",
        }
    }
}

/// Pattern-based status detector
#[allow(dead_code)]
pub struct StatusDetector {
    agent_id: String,
}

#[allow(dead_code)]
impl StatusDetector {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
        }
    }

    pub fn detect(&self, recent_output: &str) -> AgentStatus {
        if recent_output.contains("error")
            || recent_output.contains("Error")
            || recent_output.contains("ERROR")
        {
            return AgentStatus::Error;
        }

        match self.agent_id.as_str() {
            "claude" => self.detect_claude(recent_output),
            "codex" => self.detect_codex(recent_output),
            _ => AgentStatus::Running,
        }
    }

    fn detect_claude(&self, output: &str) -> AgentStatus {
        if output.contains("❯") || output.contains(">") {
            AgentStatus::Waiting
        } else if output.contains("Thinking") || output.contains("⠋") || output.contains("⠙") {
            AgentStatus::Running
        } else {
            AgentStatus::Running
        }
    }

    fn detect_codex(&self, output: &str) -> AgentStatus {
        if output.contains(">") || output.contains("$") {
            AgentStatus::Waiting
        } else {
            AgentStatus::Running
        }
    }
}
