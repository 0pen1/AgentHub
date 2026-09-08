// Shared types between frontend and Tauri backend

export interface AgentInfo {
  id: string;
  name: string;
  executable: string;
  version: string | null;
  installed: boolean;
  config_format: "json" | "toml" | "jsonc";
  instruction_file: string | null;
}

export interface SkillInfo {
  name: string;
  description: string;
  path: string;
  source: string;
}

export interface McpServerInfo {
  name: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  source: string;
}

export interface SessionInfo {
  id: string;
  agent_id: string;
  agent_name: string;
  name: string;
  work_dir: string;
  status: SessionStatus;
  created_at: string;
  skills: string[];
  mcps: string[];
  instructions: string[];
}

export type SessionStatus = "running" | "waiting" | "idle" | "error" | "stopped";

export interface LaunchConfig {
  agent_id: string;
  work_dir: string;
  skills: string[];
  mcps: string[];
  instructions: string[];
  preset_name?: string;
}

export interface ConfigPreset {
  name: string;
  skills: string[];
  mcps: string[];
  instructions: string[];
}
