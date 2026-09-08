import type { SessionInfo } from "../pages/Sessions";
import { Trash2 } from "lucide-react";

const STATUS_COLORS: Record<string, string> = {
  running: "#22c55e",
  waiting: "#eab308",
  idle: "#9e9eb0",
  error: "#ef4444",
  stopped: "#9e9eb0",
};

const AGENT_COLORS: Record<string, string> = {
  claude: "#e97627",
  codex: "#10a37f",
  gemini: "#4285f4",
  pi: "#8b5cf6",
  opencode: "#6b7280",
};

interface Props {
  sessions: SessionInfo[];
  activeId: string | null;
  /** Selecting an exited session auto-resumes it — onSelect handles both */
  onSelect: (id: string) => void;
  onKill?: (id: string) => void;
  onDelete?: (id: string) => void;
}

export default function SessionTree({ sessions, activeId, onSelect, onDelete }: Props) {
  if (sessions.length === 0) {
    return (
      <div className="p-6 text-center">
        <p className="text-sm" style={{ color: 'var(--text-muted)' }}>
          暂无会话
        </p>
      </div>
    );
  }

  return (
    <div className="p-2 space-y-0.5">
      {sessions.map((session) => {
        const isActive = activeId === session.id;
        const agentColor = AGENT_COLORS[session.agent_id] || "#6b7280";
        const isRunning = session.status === "running";
        return (
          <button
            key={session.id}
            onClick={() => onSelect(session.id)}
            className="w-full text-left px-3 py-2.5 rounded-lg transition-all relative group"
            style={{
              background: isActive ? 'var(--bg-tertiary)' : 'transparent',
            }}
            title={isRunning ? undefined : "已退出 · 点击恢复会话"}
          >
            {isActive && (
              <div className="absolute left-0 top-2 bottom-2 w-[3px] rounded-r"
                style={{ background: 'var(--accent-blue)' }} />
            )}
            <div className="flex items-center gap-2.5">
              <div className="w-6 h-6 rounded-md flex items-center justify-center text-white text-[10px] font-bold shrink-0"
                style={{ background: agentColor }}>
                {session.agent_name.charAt(0)}
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium truncate" style={{ color: 'var(--text-primary)' }}>
                  {session.name}
                </div>
                <div className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>
                  {session.work_dir.split("/").pop()}
                </div>
              </div>
              <span className="flex items-center gap-1.5 shrink-0">
                <span
                  className="w-2 h-2 rounded-full shrink-0"
                  style={{ background: STATUS_COLORS[session.status] || '#9e9eb0' }}
                />
                {!isRunning && (
                  <span
                    className="opacity-0 group-hover:opacity-100 transition-opacity p-0.5 rounded hover:bg-[var(--bg-tertiary)]"
                    style={{ color: 'var(--accent-red, #ef4444)' }}
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete?.(session.id);
                    }}
                    title="删除会话"
                  >
                    <Trash2 size={13} />
                  </span>
                )}
              </span>
            </div>
          </button>
        );
      })}
    </div>
  );
}
