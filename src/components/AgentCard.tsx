import type { AgentInfo } from "../pages/Launcher";

const AGENT_STYLE: Record<string, { bg: string; color: string }> = {
  claude: { bg: "#FFF3ED", color: "#e97627" },
  codex: { bg: "#ECFDF5", color: "#10a37f" },
  gemini: { bg: "#EFF6FF", color: "#4285f4" },
  pi: { bg: "#F5F3FF", color: "#8b5cf6" },
  opencode: { bg: "#F3F4F6", color: "#6b7280" },
};

interface Props {
  agent: AgentInfo;
  selected: boolean;
  onSelect: () => void;
}

export default function AgentCard({ agent, selected, onSelect }: Props) {
  const style = AGENT_STYLE[agent.id] || { bg: "#F3F4F6", color: "#6b7280" };

  return (
    <button
      onClick={onSelect}
      disabled={!agent.installed}
      className="w-full text-left rounded-xl transition-all disabled:cursor-not-allowed group"
      style={{
        padding: '14px 16px',
        background: selected ? 'var(--bg-tertiary)' : 'var(--bg-primary)',
        border: selected ? '1.5px solid var(--accent-blue)' : '1px solid var(--border)',
        opacity: agent.installed ? 1 : 0.35,
        boxShadow: selected ? '0 0 0 3px rgba(79,110,247,0.06)' : 'var(--shadow-xs)',
      }}
    >
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-[10px] flex items-center justify-center text-sm font-semibold shrink-0"
          style={{ background: style.bg, color: style.color }}>
          {agent.name.charAt(0)}
        </div>
        <div className="flex-1 min-w-0">
          <div className="text-[13px] font-medium truncate" style={{ color: 'var(--text-primary)' }}>
            {agent.name}
          </div>
          <div className="text-[11px] truncate mt-0.5" style={{ color: 'var(--text-muted)' }}>
            {agent.installed ? `v${agent.version || "—"}` : "未安装"}
          </div>
        </div>
        {selected && (
          <div className="w-5 h-5 rounded-full flex items-center justify-center shrink-0"
            style={{ background: 'var(--accent-blue)' }}>
            <svg width="10" height="8" viewBox="0 0 10 8" fill="none">
              <path d="M1 4L3.5 6.5L9 1" stroke="white" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </div>
        )}
      </div>
    </button>
  );
}
