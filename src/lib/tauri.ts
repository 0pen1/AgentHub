import { invoke } from "@tauri-apps/api/core";
import type { AgentInfo, SessionInfo, SkillInfo, McpServerInfo, LaunchConfig } from "./types";

// Agent Registry
export async function listAgents(): Promise<AgentInfo[]> {
  return invoke("list_agents");
}

// Skills
export async function listSkills(): Promise<SkillInfo[]> {
  return invoke("list_skills");
}

// MCP Servers
export async function listMcpServers(): Promise<McpServerInfo[]> {
  return invoke("list_mcp_servers");
}

// Sessions
export async function launchSession(config: LaunchConfig): Promise<SessionInfo> {
  return invoke("launch_session", { config });
}

export async function listSessions(): Promise<SessionInfo[]> {
  return invoke("list_sessions");
}

export async function killSession(sessionId: string): Promise<void> {
  return invoke("kill_session", { sessionId });
}

// PTY
export async function ptyWrite(sessionId: string, data: string): Promise<void> {
  return invoke("pty_write", { sessionId, data });
}

export async function ptyResize(sessionId: string, cols: number, rows: number): Promise<void> {
  return invoke("pty_resize", { sessionId, cols, rows });
}
