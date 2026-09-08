import { useState } from "react";
import { Search, Puzzle } from "lucide-react";
import type { SkillInfo } from "../pages/Launcher";

interface Props {
  skills: SkillInfo[];
  selected: string[];
  onToggle: (name: string) => void;
}

export default function SkillPicker({ skills: allSkills, selected, onToggle }: Props) {
  const [search, setSearch] = useState("");
  const skills = allSkills.filter(
    (s) =>
      s.name.toLowerCase().includes(search.toLowerCase()) ||
      s.description.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div>
      <h2 className="text-sm font-medium mb-3 flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
        <Puzzle size={15} style={{ color: 'var(--text-muted)' }} />
        Skills
        {allSkills.length > 0 && (
          <span className="text-xs font-normal px-1.5 py-0.5 rounded-md"
            style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}>
            {allSkills.length}
          </span>
        )}
      </h2>
      {allSkills.length > 0 && (
        <div className="relative mb-3">
          <Search size={14} className="absolute left-3.5 top-1/2 -translate-y-1/2"
            style={{ color: 'var(--text-muted)' }} />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索 skill..."
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
        {skills.map((skill) => (
          <label
            key={skill.path}
            className="flex items-start gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer transition-colors"
            style={{ background: selected.includes(skill.name) ? 'rgba(79,110,247,0.06)' : 'var(--bg-secondary)' }}
          >
            <input
              type="checkbox"
              checked={selected.includes(skill.name)}
              onChange={() => onToggle(skill.name)}
              className="mt-0.5 w-4 h-4 rounded accent-[var(--accent-blue)]"
            />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
                  {skill.name}
                </span>
                <span className="text-xs px-1.5 py-0.5 rounded-md"
                  style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}>
                  {skill.source}
                </span>
              </div>
              {skill.description && (
                <div className="text-xs mt-0.5 truncate" style={{ color: 'var(--text-secondary)' }}>
                  {skill.description}
                </div>
              )}
            </div>
          </label>
        ))}
        {allSkills.length === 0 && (
          <p className="text-sm py-4 text-center" style={{ color: 'var(--text-muted)' }}>
            未发现 skill
          </p>
        )}
        {allSkills.length > 0 && skills.length === 0 && (
          <p className="text-sm py-3 text-center" style={{ color: 'var(--text-muted)' }}>
            无匹配结果
          </p>
        )}
      </div>
    </div>
  );
}
