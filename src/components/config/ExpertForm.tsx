import { useState } from "react";
import type { PresetInfo, AgentLite, SkillInfo, McpServerInfo } from "../../pages/ConfigLib";
import type { InstructionInfo } from "../InstructionPicker";

const EMOJI_CHOICES = ["🤖", "🧪", "🎨", "📊", "🔧", "📝", "🚀", "🧠", "💡", "🎯", "🛡️", "🌐"];

export interface ExpertFormValue {
  name: string;
  description: string;
  agent_id: string;
  tagsText: string; // comma-separated
  work_dir: string;
  icon: string;
  skills: string[];
  mcps: string[];
  instructions: string[];
}

export function emptyExpertForm(defaultAgent: string): ExpertFormValue {
  return {
    name: "",
    description: "",
    agent_id: defaultAgent,
    tagsText: "",
    work_dir: "",
    icon: "🤖",
    skills: [],
    mcps: [],
    instructions: [],
  };
}

export function formFromPreset(p: PresetInfo): ExpertFormValue {
  return {
    name: p.name,
    description: p.description,
    agent_id: p.agent_id,
    tagsText: p.tags.join(", "),
    work_dir: p.work_dir,
    icon: p.icon,
    skills: [...p.skills],
    mcps: [...p.mcps],
    instructions: [...p.instructions],
  };
}

interface Props {
  creating: boolean;
  form: ExpertFormValue;
  setForm: (f: ExpertFormValue) => void;
  agents: AgentLite[];
  skills: SkillInfo[];
  mcps: McpServerInfo[];
  instructions: InstructionInfo[];
  onCancel: () => void;
  onSave: () => void;
  onPickDir: () => void;
}

/** Create/edit form for an expert (launch-combo preset). */
export default function ExpertForm({
  creating,
  form,
  setForm,
  agents,
  skills,
  mcps,
  instructions,
  onCancel,
  onSave,
  onPickDir,
}: Props) {
  const [contentTab, setContentTab] = useState<"skills" | "mcps" | "instructions">("skills");

  const toggle = (list: string[], name: string) =>
    list.includes(name) ? list.filter((n) => n !== name) : [...list, name];

  const chipsRow = (
    label: string,
    list: string[],
    onToggle: (name: string) => void,
  ) => (
    <div>
      <label
        className="text-[12px] font-medium block mb-1.5"
        style={{ color: 'var(--text-secondary)' }}
      >
        {label}
        <span className="ml-2 font-normal text-[11px]" style={{ color: 'var(--text-muted)' }}>
          已选 {list.length} 个
        </span>
      </label>
      <div className="flex flex-wrap gap-1.5 max-h-28 overflow-y-auto p-2 rounded-xl border"
        style={{ borderColor: 'var(--border-light)' }}>
        {label === "Skills" &&
          (skills.length === 0 ? (
            <span className="text-[11px] px-1" style={{ color: 'var(--text-muted)' }}>
              还没有可用的 skill
            </span>
          ) : (
            skills.map((s) => (
              <Chip key={s.name} label={s.name} active={list.includes(s.name)} onClick={() => onToggle(s.name)} />
            ))
          ))}
        {label === "MCP" &&
          (mcps.length === 0 ? (
            <span className="text-[11px] px-1" style={{ color: 'var(--text-muted)' }}>
              还没有可用的 MCP
            </span>
          ) : (
            mcps.map((m) => (
              <Chip key={m.name} label={m.name} active={list.includes(m.name)} onClick={() => onToggle(m.name)} />
            ))
          ))}
        {label === "提示词" &&
          (instructions.length === 0 ? (
            <span className="text-[11px] px-1" style={{ color: 'var(--text-muted)' }}>
              还没有提示词
            </span>
          ) : (
            instructions.map((i) => (
              <Chip key={i.name} label={i.name} active={list.includes(i.name)} onClick={() => onToggle(i.name)} />
            ))
          ))}
      </div>
    </div>
  );

  return (
    <div className="max-w-2xl mx-auto px-8 py-6">
      <div className="flex items-center justify-between mb-5">
        <h2 className="text-base font-semibold" style={{ color: 'var(--text-primary)' }}>
          {creating ? "新建专家" : "编辑专家"}
        </h2>
        <div className="flex items-center gap-2">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 rounded-lg text-[12px] border transition-colors hover:bg-[var(--bg-tertiary)]"
            style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}
          >
            取消
          </button>
          <button
            onClick={onSave}
            className="px-4 py-1.5 rounded-lg text-[12px] font-medium text-white transition-opacity hover:opacity-90"
            style={{ background: 'var(--accent-blue)' }}
          >
            保存
          </button>
        </div>
      </div>

      <div className="space-y-4">
        <div className="flex gap-4">
          <div className="w-24 shrink-0">
            <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
              图标
            </label>
            <div className="grid grid-cols-4 gap-1">
              {EMOJI_CHOICES.map((e) => (
                <button
                  key={e}
                  onClick={() => setForm({ ...form, icon: e })}
                  className="w-9 h-9 rounded-lg text-[17px] flex items-center justify-center transition-colors"
                  style={{
                    background: form.icon === e ? 'rgba(79,110,247,0.12)' : 'var(--bg-tertiary)',
                    outline: form.icon === e ? '1.5px solid var(--accent-blue)' : 'none',
                  }}
                >
                  {e}
                </button>
              ))}
            </div>
          </div>
          <div className="flex-1 space-y-4">
            <div>
              <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                名称
              </label>
              <input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="TDD 开发专家"
                className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                style={{ background: 'var(--bg-primary)', borderColor: 'var(--border)', color: 'var(--text-primary)' }}
              />
            </div>
            <div>
              <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                描述
              </label>
              <input
                value={form.description}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
                placeholder="这个专家擅长什么"
                className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                style={{ background: 'var(--bg-primary)', borderColor: 'var(--border)', color: 'var(--text-primary)' }}
              />
            </div>
          </div>
        </div>

        <div className="flex gap-4">
          <div className="flex-1">
            <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
              Agent
            </label>
            <select
              value={form.agent_id}
              onChange={(e) => setForm({ ...form, agent_id: e.target.value })}
              className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none"
              style={{ background: 'var(--bg-primary)', borderColor: 'var(--border)', color: 'var(--text-primary)' }}
            >
              {agents.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                  {!a.installed && " (未安装)"}
                </option>
              ))}
            </select>
          </div>
          <div className="flex-1">
            <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
              标签 (逗号分隔)
            </label>
            <input
              value={form.tagsText}
              onChange={(e) => setForm({ ...form, tagsText: e.target.value })}
              placeholder="开发, 前端"
              className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
              style={{ background: 'var(--bg-primary)', borderColor: 'var(--border)', color: 'var(--text-primary)' }}
            />
          </div>
        </div>

        <div>
          <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
            默认工作目录 (可选 — 预存后卡片可一键启动)
          </label>
          <div className="flex gap-2">
            <input
              value={form.work_dir}
              onChange={(e) => setForm({ ...form, work_dir: e.target.value })}
              placeholder="/Users/you/projects/my-app"
              className="flex-1 px-3 py-2 rounded-xl text-[13px] font-mono border outline-none focus:border-[var(--accent-blue)] transition-colors"
              style={{ background: 'var(--bg-primary)', borderColor: 'var(--border)', color: 'var(--text-primary)' }}
            />
            <button
              onClick={onPickDir}
              className="px-3 py-2 rounded-xl border text-[12px] transition-colors hover:bg-[var(--bg-tertiary)]"
              style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}
            >
              浏览
            </button>
          </div>
        </div>

        {/* Content tabs: skills / mcps / instructions */}
        <div>
          <div className="flex items-center gap-1 p-1 rounded-xl w-fit mb-3"
            style={{ background: 'var(--bg-tertiary)' }}>
            {(["skills", "mcps", "instructions"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setContentTab(t)}
                className="px-3 py-1 rounded-lg text-[12px] font-medium transition-all"
                style={{
                  background: contentTab === t ? 'var(--bg-primary)' : 'transparent',
                  color: contentTab === t ? 'var(--text-primary)' : 'var(--text-muted)',
                }}
              >
                {t === "skills" ? "Skills" : t === "mcps" ? "MCP" : "提示词"}
              </button>
            ))}
          </div>
          {contentTab === "skills" &&
            chipsRow("Skills", form.skills, (n) => setForm({ ...form, skills: toggle(form.skills, n) }))}
          {contentTab === "mcps" &&
            chipsRow("MCP", form.mcps, (n) => setForm({ ...form, mcps: toggle(form.mcps, n) }))}
          {contentTab === "instructions" &&
            chipsRow("提示词", form.instructions, (n) =>
              setForm({ ...form, instructions: toggle(form.instructions, n) }))}
        </div>
      </div>
    </div>
  );
}

function Chip({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="px-2 py-1 rounded-lg text-[11px] transition-colors max-w-[200px] truncate"
      style={{
        background: active ? 'var(--accent-blue)' : 'var(--bg-secondary)',
        color: active ? 'white' : 'var(--text-secondary)',
        border: active ? 'none' : '1px solid var(--border-light)',
      }}
      title={label}
    >
      {label}
    </button>
  );
}
