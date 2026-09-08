import type { SkillInfo } from "../../pages/ConfigLib";

interface Props {
  skills: SkillInfo[];
  activeName: string | null;
  onSelect: (name: string) => void;
  onCreate: () => void;
  onImport: () => void;
}

export default function SkillList({ skills, activeName, onSelect, onCreate, onImport }: Props) {
  return (
    <>
      <div className="px-3 pt-3 pb-2 flex items-center justify-between shrink-0">
        <span className="text-[11px] font-medium uppercase tracking-[0.05em]"
          style={{ color: 'var(--text-muted)' }}>
          Skills · {skills.length}
        </span>
        <div className="flex items-center gap-1">
          <button onClick={onImport}
            className="px-2 py-1 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-tertiary)]"
            style={{ color: 'var(--text-secondary)' }}
            title="从磁盘导入 skill 目录">
            导入
          </button>
          <button onClick={onCreate}
            className="px-2 py-1 rounded-md text-[11px] font-medium transition-opacity hover:opacity-80"
            style={{ background: 'var(--accent-blue)', color: 'white' }}>
            + 新建
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto px-2 pb-3 space-y-0.5">
        {skills.map((skill) => (
          <button key={skill.name} onClick={() => onSelect(skill.name)}
            className="w-full text-left px-3 py-2.5 rounded-lg transition-all"
            style={{ background: activeName === skill.name ? 'var(--bg-tertiary)' : 'transparent' }}>
            <div className="text-[13px] font-medium truncate" style={{ color: 'var(--text-primary)' }}>
              {skill.name}
            </div>
            {skill.description && (
              <div className="text-xs truncate mt-0.5" style={{ color: 'var(--text-muted)' }}>
                {skill.description}
              </div>
            )}
          </button>
        ))}
        {skills.length === 0 && (
          <p className="text-xs px-3 py-6 text-center" style={{ color: 'var(--text-muted)' }}>
            还没有托管的 skill
          </p>
        )}
      </div>
    </>
  );
}
