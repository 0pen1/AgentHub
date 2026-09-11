import { Play, Edit3, Trash2 } from "lucide-react";
import type { PresetInfo, AgentLite } from "../../pages/ConfigLib";
import { TagChips } from "./LibraryCard";

interface Props {
  preset: PresetInfo;
  agents: AgentLite[];
  activeTag?: string | null;
  onTagClick?: (tag: string) => void;
  onLaunch: (preset: PresetInfo) => void;
  onEdit: (preset: PresetInfo) => void;
  onDelete: (preset: PresetInfo) => void;
}

/**
 * Expert card: fixed agent + skills/MCP/prompts combo, one-click launch.
 *
 * Layout is height-normalized so every card reads the same regardless of
 * content: title/badge row and summary row are single-line (truncate),
 * description is exactly two lines (line-clamp + fixed height), the tag slot
 * reserves space even when empty, and the action row is pinned to the bottom
 * (mt-auto) — grid rows stretch, so buttons align across the wall.
 */
export default function ExpertCard({
  preset,
  agents,
  activeTag,
  onTagClick,
  onLaunch,
  onEdit,
  onDelete,
}: Props) {
  const agent = agents.find((a) => a.id === preset.agent_id);
  const summary = [
    preset.skills.length ? `${preset.skills.length} skill` : null,
    preset.mcps.length ? `${preset.mcps.length} MCP` : null,
    preset.instructions.length ? `${preset.instructions.length} 系统指令` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <div
      className="group relative rounded-2xl border p-4 transition-all hover:shadow-sm flex flex-col h-full"
      style={{ background: 'var(--bg-secondary)', borderColor: 'var(--border-light)' }}
    >
      {/* Hover actions */}
      <div
        className="absolute top-2.5 right-2.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
      >
        <button
          onClick={() => onEdit(preset)}
          title="编辑"
          className="p-1.5 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
          style={{ color: 'var(--text-muted)' }}
        >
          <Edit3 size={13} />
        </button>
        <button
          onClick={() => onDelete(preset)}
          title="删除"
          className="p-1.5 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
          style={{ color: 'var(--accent-red, #ef4444)' }}
        >
          <Trash2 size={13} />
        </button>
      </div>

      {/* Header: icon + single-line title/badge + single-line summary */}
      <div className="flex items-start gap-3">
        <div
          className="w-10 h-10 rounded-xl flex items-center justify-center text-[19px] shrink-0"
          style={{ background: 'var(--bg-tertiary)' }}
        >
          {preset.icon || "🤖"}
        </div>
        <div className="min-w-0 flex-1 pr-12">
          <div className="flex items-center gap-1.5 min-w-0">
            <h3
              className="text-[13px] font-medium truncate min-w-0"
              style={{ color: 'var(--text-primary)' }}
              title={preset.name}
            >
              {preset.name}
            </h3>
            {agent && (
              <span
                className="text-[10px] px-1.5 py-0.5 rounded-md shrink-0"
                style={{ background: 'rgba(139,92,246,0.12)', color: '#8b5cf6' }}
                title={`Agent: ${agent.name}`}
              >
                {agent.name}
              </span>
            )}
          </div>
          <p className="text-[11px] mt-0.5 truncate" style={{ color: 'var(--text-muted)' }} title={summary}>
            {summary || "无固定配置"}
          </p>
        </div>
      </div>

      {/* Description: always exactly two lines tall */}
      <p
        className="text-[12px] leading-[1.5] mt-2 line-clamp-2 h-[3em] overflow-hidden"
        style={{ color: 'var(--text-secondary)' }}
        title={preset.description}
      >
        {preset.description || <span style={{ color: 'var(--text-muted)' }}>暂无描述</span>}
      </p>

      {/* Tag slot: fixed height so cards with no tags align with tagged ones */}
      <div className="min-h-[28px] overflow-hidden">
        <TagChips tags={preset.tags} activeTag={activeTag} onTagClick={onTagClick} />
      </div>

      {/* Actions: pinned to the card bottom */}
      <div className="mt-auto pt-3 flex items-center gap-2">
        <button
          onClick={() => onLaunch(preset)}
          className="flex-1 inline-flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-[12px] font-medium text-white transition-opacity hover:opacity-90"
          style={{ background: 'var(--accent-blue)' }}
        >
          <Play size={12} />
          {preset.work_dir ? "启动" : "选择目录启动"}
        </button>
        {preset.work_dir && (
          <span
            className="text-[10px] font-mono truncate max-w-[110px]"
            style={{ color: 'var(--text-muted)' }}
            title={preset.work_dir}
          >
            {preset.work_dir}
          </span>
        )}
      </div>
    </div>
  );
}
