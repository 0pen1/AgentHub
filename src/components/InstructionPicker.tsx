import { useState } from "react";
import { Search, FileText, Lock } from "lucide-react";

export interface InstructionInfo {
  name: string;
  path: string;
  description: string;
  tags: string[];
  source: string;
}

interface Props {
  instructions: InstructionInfo[];
  selected: string[];
  onToggle: (path: string) => void;
  /** Expert preset locks the selection: checkboxes disabled, items shown read-only */
  locked?: boolean;
}

export default function InstructionPicker({ instructions, selected, onToggle, locked }: Props) {
  const [search, setSearch] = useState("");
  const filtered = instructions.filter(
    (i) =>
      i.name.toLowerCase().includes(search.toLowerCase()) ||
      i.description.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div>
      <h2 className="text-sm font-medium mb-3 flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
        <FileText size={15} style={{ color: 'var(--text-muted)' }} />
        提示词
        {instructions.length > 0 && (
          <span className="text-xs font-normal px-1.5 py-0.5 rounded-md"
            style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}>
            {instructions.length}
          </span>
        )}
      </h2>
      {instructions.length > 0 && (
        <div className="relative mb-3">
          <Search size={14} className="absolute left-3.5 top-1/2 -translate-y-1/2"
            style={{ color: 'var(--text-muted)' }} />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索提示词..."
            className="w-full pl-9 pr-3 py-2 rounded-xl text-sm border outline-none transition-all"
            style={{
              background: 'var(--bg-primary)',
              borderColor: 'var(--border)',
              color: 'var(--text-primary)',
            }}
          />
        </div>
      )}
      <div className="space-y-1.5 max-h-48 overflow-y-auto">
        {locked && (
          <p
            className="text-xs flex items-center gap-1.5 mb-2"
            style={{ color: "var(--text-muted)" }}
          >
            <Lock size={12} />
            已由所选专家固定，不可修改
          </p>
        )}
        {filtered.map((inst) => (
          <label
            key={inst.path}
            className="flex items-start gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer transition-colors"
            style={{
              background: selected.includes(inst.path) ? "rgba(79,110,247,0.06)" : "var(--bg-secondary)",
              opacity: locked && !selected.includes(inst.path) ? 0.5 : 1,
            }}
          >
            <input
              type="checkbox"
              checked={selected.includes(inst.path)}
              onChange={() => !locked && onToggle(inst.path)}
              disabled={locked}
              className="mt-0.5 w-4 h-4 rounded accent-[var(--accent-blue)]"
            />
            <div className="min-w-0 flex-1">
              <div className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
                {inst.name}
              </div>
              {inst.description && (
                <div className="text-xs mt-0.5 truncate" style={{ color: 'var(--text-secondary)' }}>
                  {inst.description}
                </div>
              )}
            </div>
          </label>
        ))}
        {instructions.length === 0 && (
          <p className="text-sm py-4 text-center" style={{ color: 'var(--text-muted)' }}>
            提示词库为空，可在配置管理中添加
          </p>
        )}
        {instructions.length > 0 && filtered.length === 0 && (
          <p className="text-sm py-3 text-center" style={{ color: 'var(--text-muted)' }}>
            无匹配结果
          </p>
        )}
      </div>
    </div>
  );
}
