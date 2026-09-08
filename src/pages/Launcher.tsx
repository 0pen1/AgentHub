import { useState, useEffect, useRef, Fragment } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useNavigate, useLocation } from "react-router-dom";
import {
  FolderOpen,
  ArrowRight,
  ArrowLeft,
  Play,
  Bot,
  Settings2,
  Check,
} from "lucide-react";
import AgentCard from "../components/AgentCard";
import SkillPicker from "../components/SkillPicker";
import McpPicker from "../components/McpPicker";
import InstructionPicker, { type InstructionInfo } from "../components/InstructionPicker";

export interface AgentInfo {
  id: string;
  name: string;
  executable: string;
  version: string | null;
  installed: boolean;
  config_format: string;
  instruction_file: string | null;
}

export interface SkillInfo {
  name: string;
  description: string;
  tags?: string[];
  path: string;
  source: string;
}

export interface McpServerInfo {
  name: string;
  command: string;
  args: string[];
  source: string;
}

export interface PresetInfo {
  name: string;
  description: string;
  agent_id: string;
  tags: string[];
  work_dir: string;
  icon: string;
  skills: string[];
  mcps: string[];
  instructions: string[];
}

const STEPS = [
  { n: 1, label: "专家 / Agent" },
  { n: 2, label: "工作目录" },
  { n: 3, label: "能力配置" },
  { n: 4, label: "确认启动" },
];

export default function Launcher() {
  const navigate = useNavigate();
  const location = useLocation();
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [mcpServers, setMcpServers] = useState<McpServerInfo[]>([]);
  const [instructions, setInstructions] = useState<InstructionInfo[]>([]);
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const [workDir, setWorkDir] = useState<string>("");
  const [selectedSkills, setSelectedSkills] = useState<string[]>([]);
  const [selectedMcps, setSelectedMcps] = useState<string[]>([]);
  const [selectedInstructions, setSelectedInstructions] = useState<string[]>([]);
  const [selectedExpert, setSelectedExpert] = useState<string | null>(null);
  const [launching, setLaunching] = useState(false);
  // Wizard state: current step + furthest reachable step (unlocks indicator jumps)
  const [step, setStep] = useState(1);
  const [maxReached, setMaxReached] = useState(1);
  const [contentTab, setContentTab] = useState<"skills" | "mcps" | "instructions">("skills");

  const loadData = async () => {
    try {
      const [agentList, skillList, mcpList, instructionList, presetList] = await Promise.all([
        invoke<AgentInfo[]>("list_agents"),
        // Launch pickers only offer managed ("托管") entries — full scans of
        // every agent's own config live in the ConfigLib discovery sections.
        invoke<SkillInfo[]>("list_library_skills").catch(() => [] as SkillInfo[]),
        invoke<McpServerInfo[]>("list_library_mcps").catch(() => [] as McpServerInfo[]),
        invoke<InstructionInfo[]>("list_library_instructions"),
        invoke<PresetInfo[]>("list_presets"),
      ]);
      setAgents(agentList);
      setSkills(skillList);
      setMcpServers(mcpList);
      setInstructions(instructionList);
      setPresets(presetList);
    } catch (err) {
      console.error("Failed to load data:", err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  // Fill the pickers from a preset (user can still adjust afterwards).
  // Presets store instruction NAMES (file stems); the picker and injector both
  // key off full paths — resolve name → path here so the checkboxes light up
  // and the backend merges the right files.
  const applyPreset = (p: PresetInfo) => {
    setSelectedExpert(p.name);
    setSelectedAgent(p.agent_id);
    setWorkDir(p.work_dir || "");
    setSelectedSkills(p.skills);
    setSelectedMcps(p.mcps);
    setSelectedInstructions(
      p.instructions
        .map((name) => instructions.find((i) => i.name === name || i.path === name)?.path || name)
    );
  };

  const clearPreset = () => {
    setSelectedExpert(null);
    setSelectedSkills([]);
    setSelectedMcps([]);
    setSelectedInstructions([]);
    setSelectedAgent(null);
    setWorkDir("");
  };

  const handleLaunch = async (overrideAgent?: string, overrideDir?: string) => {
    const agent = overrideAgent || selectedAgent;
    const dir = overrideDir || workDir;
    if (!agent || !dir) return;
    setLaunching(true);
    try {
      const session = await invoke<{
        id: string;
        agent_id: string;
        agent_name: string;
        name: string;
        work_dir: string;
        status: string;
        created_at: string;
      }>("launch_session", {
        config: {
          agent_id: agent,
          work_dir: dir,
          skills: selectedSkills,
          mcps: selectedMcps,
          instructions: selectedInstructions,
        },
      });
      // Navigate to sessions page; pass the new session via router state
      navigate("/sessions", { state: { launched: session } });
    } catch (err) {
      console.error("Launch failed:", err);
      alert(`启动失败: ${err}`);
    } finally {
      setLaunching(false);
    }
  };

  const launchExpertDirect = async (p: PresetInfo) => {
    if (!p.work_dir || launching) return;
    setLaunching(true);
    try {
      const session = await invoke<SessionSummary>("launch_session", {
        config: {
          agent_id: p.agent_id,
          work_dir: p.work_dir,
          skills: p.skills,
          mcps: p.mcps,
          instructions: p.instructions,
        },
      });
      navigate("/sessions", { state: { launched: session } });
    } catch (err) {
      console.error("Expert launch failed:", err);
      alert(`启动失败: ${err}`);
    } finally {
      setLaunching(false);
    }
  };

  // One-shot direct launch requested from the ConfigLib expert card
  // (route state carries the preset name).
  const expertLaunchHandledRef = useRef(false);
  useEffect(() => {
    if (expertLaunchHandledRef.current) return;
    const state = location.state as { expertLaunch?: string } | null;
    if (!state?.expertLaunch || presets.length === 0) return;
    const p = presets.find((x) => x.name === state.expertLaunch);
    if (p) {
      expertLaunchHandledRef.current = true;
      navigate("/", { replace: true, state: {} });
      launchExpertDirect(p);
    }
  }, [location.state, presets, navigate]);

  const handleSelectDir = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        setWorkDir(selected as string);
      }
    } catch (err) {
      console.error("Directory selection failed:", err);
    }
  };

  const selectedAgentInfo = agents.find((a) => a.id === selectedAgent);
  const selectedExpertInfo = presets.find((p) => p.name === selectedExpert);

  // --- Wizard navigation ---
  const stepValid = (s: number): boolean => {
    if (s === 1) return !!selectedAgent;
    if (s === 2) return !!workDir.trim();
    return true;
  };

  const goNext = () => {
    if (!stepValid(step) || step >= STEPS.length) return;
    const next = step + 1;
    setStep(next);
    setMaxReached((m) => Math.max(m, next));
  };

  const goBack = () => setStep((s) => Math.max(1, s - 1));

  const jumpTo = (n: number) => {
    if (n > maxReached || n === step) return;
    setStep(n);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Fixed header: title + step indicator */}
      <div
        className="shrink-0"
        style={{ borderBottom: "1px solid var(--border-light)", background: "var(--bg-secondary)" }}
      >
        <div className="max-w-2xl mx-auto px-8 pt-4 pb-4">
          <h1 className="text-[20px] font-semibold tracking-[-0.02em]" style={{ color: "var(--text-primary)" }}>
            开始新会话
          </h1>
          <p className="text-[12px] mt-0.5" style={{ color: "var(--text-secondary)" }}>
            选择专家快速开始，或按步骤完成手动配置
          </p>
          <div className="mt-4">
            <StepIndicator current={step} maxReached={maxReached} onJump={jumpTo} />
          </div>
        </div>
      </div>

      {/* Step content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-2xl mx-auto px-8 py-6">
          {/* Step 1: expert or agent */}
          {step === 1 && (
            <div className="space-y-7">
              {presets.length > 0 && (
                <section>
                  <h2
                    className="text-[13px] font-medium mb-3 flex items-center gap-2"
                    style={{ color: "var(--text-primary)" }}
                  >
                    <Bot size={15} style={{ color: "var(--text-muted)" }} />
                    专家
                    <span className="text-[11px] font-normal" style={{ color: "var(--text-muted)" }}>
                      · 选中后自动填充配置，可再微调
                    </span>
                  </h2>
                  <div className="grid grid-cols-2 gap-2.5">
                    {presets.map((p) => {
                      const active = selectedExpert === p.name;
                      return (
                        <div
                          key={p.name}
                          onClick={() => (active ? clearPreset() : applyPreset(p))}
                          className="group relative rounded-xl border p-3 cursor-pointer transition-all"
                          style={{
                            background: active ? "rgba(79,110,247,0.06)" : "var(--bg-secondary)",
                            borderColor: active ? "var(--accent-blue)" : "var(--border-light)",
                          }}
                        >
                          <div className="flex items-start gap-2.5">
                            <div
                              className="w-8 h-8 rounded-lg flex items-center justify-center text-[15px] shrink-0"
                              style={{ background: "var(--bg-tertiary)" }}
                            >
                              {p.icon || "🤖"}
                            </div>
                            <div className="min-w-0 flex-1">
                              <div className="text-[13px] font-medium truncate" style={{ color: "var(--text-primary)" }}>
                                {p.name}
                              </div>
                              <div className="text-[11px] truncate mt-0.5" style={{ color: "var(--text-muted)" }}>
                                {p.description ||
                                  `${p.skills.length} skill · ${p.mcps.length} MCP · ${p.instructions.length} 提示词`}
                              </div>
                            </div>
                            {p.work_dir && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  launchExpertDirect(p);
                                }}
                                disabled={launching}
                                title={`直接启动 (${p.work_dir})`}
                                className="p-1.5 rounded-lg text-white transition-opacity hover:opacity-80 disabled:opacity-30 opacity-0 group-hover:opacity-100 shrink-0"
                                style={{ background: "var(--accent-blue)" }}
                              >
                                <Play size={12} />
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                  <div className="flex items-center gap-3 mt-6">
                    <div className="flex-1 h-px" style={{ background: "var(--border-light)" }} />
                    <span className="text-[11px]" style={{ color: "var(--text-muted)" }}>
                      或手动配置
                    </span>
                    <div className="flex-1 h-px" style={{ background: "var(--border-light)" }} />
                  </div>
                </section>
              )}

              <section>
                <h2 className="text-[13px] font-medium mb-3" style={{ color: "var(--text-primary)" }}>
                  选择 Agent
                </h2>
                <div className="grid grid-cols-3 gap-2.5">
                  {agents.map((agent) => (
                    <AgentCard
                      key={agent.id}
                      agent={agent}
                      selected={selectedAgent === agent.id}
                      onSelect={() => {
                        // Manual agent pick deselects the expert
                        setSelectedExpert(null);
                        setSelectedAgent(agent.id);
                      }}
                    />
                  ))}
                </div>
              </section>

              {presets.length === 0 && (
                <button
                  onClick={() => navigate("/config")}
                  className="w-full py-3 rounded-xl border border-dashed text-[12px] flex items-center justify-center gap-1.5 transition-colors hover:bg-[var(--bg-tertiary)]"
                  style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
                >
                  <Settings2 size={13} />
                  还没有专家 — 去配置管理创建专家
                  <ArrowRight size={12} />
                </button>
              )}
            </div>
          )}

          {/* Step 2: work directory */}
          {step === 2 && (
            <section>
              <h2 className="text-[13px] font-medium mb-3" style={{ color: "var(--text-primary)" }}>
                工作目录
              </h2>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={workDir}
                  onChange={(e) => setWorkDir(e.target.value)}
                  placeholder="选择项目目录..."
                  className="flex-1 px-4 py-2.5 rounded-xl text-[13px] border outline-none transition-all"
                  style={{
                    background: "var(--bg-primary)",
                    borderColor: workDir ? "var(--border)" : "var(--text-muted)",
                    color: "var(--text-primary)",
                  }}
                />
                <button
                  className="px-4 py-2.5 rounded-xl border text-[13px] transition-colors hover:bg-[var(--bg-tertiary)] flex items-center gap-1.5"
                  style={{ borderColor: "var(--border)", color: "var(--text-secondary)" }}
                  onClick={handleSelectDir}
                >
                  <FolderOpen size={15} />
                  浏览
                </button>
              </div>
              <p className="text-[12px] mt-2" style={{ color: "var(--text-muted)" }}>
                会话将在此目录下启动，Agent 将读取该目录中的项目文件
              </p>
            </section>
          )}

          {/* Step 3: skills / mcp / instructions (tabbed) */}
          {step === 3 && (
            <section>
              <h2
                className="text-[13px] font-medium mb-3 flex items-center gap-2"
                style={{ color: "var(--text-primary)" }}
              >
                能力配置
                <span className="text-[11px] font-normal" style={{ color: "var(--text-muted)" }}>
                  可选 — 为会话注入 Skills、MCP 与提示词
                </span>
              </h2>
              <div className="flex items-center gap-1 p-1 rounded-xl w-fit mb-4" style={{ background: "var(--bg-tertiary)" }}>
                {(["skills", "mcps", "instructions"] as const).map((t) => {
                  const count =
                    t === "skills"
                      ? selectedSkills.length
                      : t === "mcps"
                      ? selectedMcps.length
                      : selectedInstructions.length;
                  const label = t === "skills" ? "Skills" : t === "mcps" ? "MCP" : "提示词";
                  return (
                    <button
                      key={t}
                      onClick={() => setContentTab(t)}
                      className="px-3.5 py-1.5 rounded-lg text-[12px] font-medium transition-all"
                      style={{
                        background: contentTab === t ? "var(--bg-primary)" : "transparent",
                        color: contentTab === t ? "var(--text-primary)" : "var(--text-muted)",
                      }}
                    >
                      {label}
                      {count > 0 && (
                        <span
                          className="ml-1.5 inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full text-[10px] text-white"
                          style={{ background: "var(--accent-blue)" }}
                        >
                          {count}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
              {contentTab === "skills" && (
                <SkillPicker
                  skills={skills}
                  selected={selectedSkills}
                  onToggle={(name) =>
                    setSelectedSkills((prev) =>
                      prev.includes(name) ? prev.filter((s) => s !== name) : [...prev, name]
                    )
                  }
                />
              )}
              {contentTab === "mcps" && (
                <McpPicker
                  servers={mcpServers}
                  selected={selectedMcps}
                  onToggle={(name) =>
                    setSelectedMcps((prev) =>
                      prev.includes(name) ? prev.filter((s) => s !== name) : [...prev, name]
                    )
                  }
                />
              )}
              {contentTab === "instructions" && (
                <InstructionPicker
                  instructions={instructions}
                  selected={selectedInstructions}
                  locked={!!selectedExpert}
                  onToggle={(path) =>
                    setSelectedInstructions((prev) =>
                      prev.includes(path) ? prev.filter((s) => s !== path) : [...prev, path]
                    )
                  }
                />
              )}
            </section>
          )}

          {/* Step 4: review + confirm */}
          {step === 4 && (
            <section>
              <h2 className="text-[13px] font-medium mb-3" style={{ color: "var(--text-primary)" }}>
                确认启动
              </h2>
              <div
                className="rounded-xl border p-4 space-y-3.5"
                style={{ background: "var(--bg-secondary)", borderColor: "var(--border-light)" }}
              >
                <SummaryRow label="Agent">
                  <div className="flex items-center gap-2 flex-wrap">
                    {selectedExpertInfo && (
                      <span
                        className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md text-[11px]"
                        style={{ background: "rgba(79,110,247,0.12)", color: "var(--accent-blue)" }}
                      >
                        <Bot size={11} />
                        {selectedExpertInfo.name}
                      </span>
                    )}
                    {selectedAgentInfo?.name}
                  </div>
                </SummaryRow>
                <SummaryRow label="工作目录">
                  <span className="font-mono text-[12px] break-all">
                    {workDir || <span style={{ color: "var(--text-muted)" }}>未选择</span>}
                  </span>
                </SummaryRow>
                <SummaryRow label="Skills">
                  <ChipList items={selectedSkills} />
                </SummaryRow>
                <SummaryRow label="MCP">
                  <ChipList items={selectedMcps} />
                </SummaryRow>
                <SummaryRow label="提示词">
                  <ChipList items={selectedInstructions} />
                </SummaryRow>
              </div>
            </section>
          )}
        </div>
      </div>

      {/* Bottom nav bar */}
      <div
        className="px-8 py-4 shrink-0"
        style={{ borderTop: "1px solid var(--border-light)", background: "var(--bg-secondary)" }}
      >
        <div className="max-w-2xl mx-auto flex items-center justify-between">
          {step > 1 ? (
            <button
              onClick={goBack}
              className="inline-flex items-center gap-1.5 px-4 py-2.5 rounded-xl border text-[13px] transition-colors hover:bg-[var(--bg-tertiary)]"
              style={{ borderColor: "var(--border)", color: "var(--text-secondary)" }}
            >
              <ArrowLeft size={15} />
              上一步
            </button>
          ) : (
            <div className="text-[13px]" style={{ color: "var(--text-muted)" }}>
              {selectedAgent ? "已选择配置，点击下一步继续" : "先选择专家或 Agent"}
            </div>
          )}

          <div className="flex items-center gap-3">
            {selectedAgent && (
              <div className="flex items-center text-[13px]" style={{ color: "var(--text-secondary)" }}>
                {selectedExpertInfo && (
                  <span
                    className="inline-flex items-center gap-1 mr-2 px-1.5 py-0.5 rounded-md"
                    style={{ background: "rgba(79,110,247,0.12)", color: "var(--accent-blue)" }}
                  >
                    <Bot size={11} />
                    {selectedExpertInfo.name}
                  </span>
                )}
                {selectedAgentInfo?.name}
                {workDir && (
                  <span className="ml-2 font-mono text-[11px]" style={{ color: "var(--text-muted)" }}>
                    {workDir.split("/").pop()}
                  </span>
                )}
                {!workDir && (
                  <span className="ml-2 text-[12px]" style={{ color: "var(--accent-yellow)" }}>
                    · 请选择工作目录
                  </span>
                )}
              </div>
            )}
            {step < 4 ? (
              <button
                onClick={goNext}
                disabled={!stepValid(step)}
                className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-[13px] font-medium text-white transition-all disabled:opacity-30 hover:opacity-90"
                style={{ background: "var(--accent-blue)" }}
              >
                下一步
                <ArrowRight size={15} />
              </button>
            ) : (
              <button
                onClick={() => handleLaunch()}
                disabled={!workDir || launching}
                className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-[13px] font-medium text-white transition-all disabled:opacity-30 hover:opacity-90"
                style={{ background: "var(--accent-blue)" }}
              >
                {launching ? "启动中..." : "启动会话"}
                {!launching && <ArrowRight size={15} />}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/** Circular step indicator with connecting lines; done/active/reachable states. */
function StepIndicator({
  current,
  maxReached,
  onJump,
}: {
  current: number;
  maxReached: number;
  onJump: (n: number) => void;
}) {
  return (
    <div className="flex items-center">
      {STEPS.map((s, i) => {
        const done = s.n < current;
        const active = s.n === current;
        const clickable = s.n <= maxReached && !active;
        return (
          <Fragment key={s.n}>
            {i > 0 && (
              <div
                className="flex-1 h-[2px] mx-2.5 rounded-full transition-colors"
                style={{ background: s.n <= current ? "var(--accent-blue)" : "var(--border-light)" }}
              />
            )}
            <button
              onClick={() => clickable && onJump(s.n)}
              disabled={!clickable}
              className="flex items-center gap-1.5"
              style={{ cursor: clickable ? "pointer" : "default" }}
            >
              <span
                className="w-[22px] h-[22px] rounded-full flex items-center justify-center text-[11px] font-medium transition-colors shrink-0"
                style={
                  active
                    ? { background: "var(--accent-blue)", color: "white" }
                    : done
                    ? { background: "rgba(79,110,247,0.12)", color: "var(--accent-blue)" }
                    : { background: "var(--bg-tertiary)", color: "var(--text-muted)" }
                }
              >
                {done ? <Check size={12} strokeWidth={2.5} /> : s.n}
              </span>
              <span
                className="text-[12px] whitespace-nowrap"
                style={{
                  color: active ? "var(--text-primary)" : done ? "var(--text-secondary)" : "var(--text-muted)",
                  fontWeight: active ? 500 : 400,
                }}
              >
                {s.label}
              </span>
            </button>
          </Fragment>
        );
      })}
    </div>
  );
}

function SummaryRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-3">
      <span className="text-[12px] w-14 shrink-0 pt-0.5" style={{ color: "var(--text-muted)" }}>
        {label}
      </span>
      <div className="flex-1 min-w-0 text-[13px]" style={{ color: "var(--text-primary)" }}>
        {children}
      </div>
    </div>
  );
}

function ChipList({ items, empty }: { items: string[]; empty?: string }) {
  if (items.length === 0) {
    return (
      <span className="text-[12px]" style={{ color: "var(--text-muted)" }}>
        {empty || "未选择"}
      </span>
    );
  }
  return (
    <div className="flex flex-wrap gap-1.5">
      {items.map((x) => (
        <span
          key={x}
          className="px-2 py-0.5 rounded-lg text-[11px] max-w-[220px] truncate"
          style={{ background: "rgba(79,110,247,0.12)", color: "var(--accent-blue)" }}
          title={x}
        >
          {x}
        </span>
      ))}
    </div>
  );
}

interface SessionSummary {
  id: string;
}
