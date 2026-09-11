import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useNavigate } from "react-router-dom";
import { Puzzle, Plug, FileText, Bot, Plus } from "lucide-react";
import LibraryCard from "../components/config/LibraryCard";
import CardWallToolbar, {
  collectTags,
  matchesSearch,
} from "../components/config/CardWallToolbar";
import ExpertCard from "../components/config/ExpertCard";
import ExpertForm, {
  emptyExpertForm,
  formFromPreset,
  type ExpertFormValue,
} from "../components/config/ExpertForm";
import type { InstructionInfo } from "../components/InstructionPicker";
import { copyText } from "../lib/clipboard";
import { stripFrontmatter } from "../components/PromptPanel";

export interface SkillInfo {
  name: string;
  description: string;
  tags: string[];
  path: string;
  source: string;
}

export interface McpServerInfo {
  name: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  server_type?: string | null;
  url?: string | null;
  source: string;
}

export interface McpImportCandidate {
  name: string;
  command: string;
  args: string[];
  env_keys: string[];
  exists_in_library: boolean;
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

export interface AgentLite {
  id: string;
  name: string;
  installed: boolean;
}

type Tab = "skills" | "mcps" | "instructions" | "experts";

interface SkillDetail {
  name: string;
  description: string;
  tags: string[];
  path: string;
  content: string;
  files: string[];
}

const TABS: { id: Tab; label: string; icon: typeof Puzzle }[] = [
  { id: "experts", label: "专家", icon: Bot },
  { id: "skills", label: "Skills", icon: Puzzle },
  { id: "mcps", label: "MCP 服务器", icon: Plug },
  { id: "instructions", label: "系统指令", icon: FileText },
];

const inputStyle = {
  background: 'var(--bg-primary)',
  borderColor: 'var(--border)',
  color: 'var(--text-primary)',
} as const;

export default function ConfigLib() {
  const navigate = useNavigate();
  const [tab, setTab] = useState<Tab>("experts");

  // Shared search / tag filter state (reset on tab switch)
  const [search, setSearch] = useState("");
  const [activeTag, setActiveTag] = useState<string | null>(null);

  // Skills
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [scannedSkills, setScannedSkills] = useState<SkillInfo[]>([]);
  const [skillDetail, setSkillDetail] = useState<SkillDetail | null>(null);
  const [skillEditing, setSkillEditing] = useState(false);
  const [skillCreating, setSkillCreating] = useState(false);
  const [skillForm, setSkillForm] = useState({
    name: "",
    description: "",
    tagsText: "",
    content: "",
  });

  // MCPs
  const [mcps, setMcps] = useState<McpServerInfo[]>([]);
  const [scannedMcps, setScannedMcps] = useState<McpServerInfo[]>([]);
  const [mcpEditing, setMcpEditing] = useState(false);
  const [mcpCreating, setMcpCreating] = useState(false);
  const [mcpForm, setMcpForm] = useState<{
    name: string;
    serverType: "stdio" | "sse" | "http";
    command: string;
    argsText: string;
    envPairs: { key: string; value: string }[];
    url: string;
  }>({
    name: "",
    serverType: "stdio",
    command: "",
    argsText: "",
    envPairs: [],
    url: "",
  });
  const [importCandidates, setImportCandidates] = useState<McpImportCandidate[] | null>(null);
  const [importChecked, setImportChecked] = useState<string[]>([]);
  const [importSourcePath, setImportSourcePath] = useState("");

  // Instructions
  const [instructions, setInstructions] = useState<InstructionInfo[]>([]);
  const [activeInst, setActiveInst] = useState<string | null>(null);
  const [instEditing, setInstEditing] = useState(false);
  const [instCreating, setInstCreating] = useState(false);
  const [instForm, setInstForm] = useState({ name: "", content: "" });
  // Name of the instruction whose content was just copied (feedback icon).
  const [copiedInst, setCopiedInst] = useState<string | null>(null);

  // Experts
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [agents, setAgents] = useState<AgentLite[]>([]);
  const [expertEditing, setExpertEditing] = useState(false);
  const [expertCreating, setExpertCreating] = useState(false);
  const [expertForm, setExpertForm] = useState<ExpertFormValue>(emptyExpertForm("claude"));
  const [expertEditingName, setExpertEditingName] = useState<string | null>(null);

  // Full data loaders — skills/instructions re-parsed from disk, agents for expert form
  const loadSkills = useCallback(async () => {
    try {
      const [library, scanned] = await Promise.all([
        invoke<SkillInfo[]>("list_library_skills"),
        invoke<SkillInfo[]>("list_skills"),
      ]);
      setSkills(library);
      setScannedSkills(scanned.filter((s) => s.source !== "托管"));
    } catch (err) {
      console.error(err);
    }
  }, []);

  const loadMcps = useCallback(async () => {
    try {
      const [library, scanned] = await Promise.all([
        invoke<McpServerInfo[]>("list_library_mcps"),
        invoke<McpServerInfo[]>("list_mcp_servers"),
      ]);
      setMcps(library);
      setScannedMcps(scanned.filter((m) => m.source !== "托管"));
    } catch (err) {
      console.error(err);
    }
  }, []);

  const loadInstructions = useCallback(async () => {
    try {
      setInstructions(await invoke<InstructionInfo[]>("list_library_instructions"));
    } catch (err) {
      console.error(err);
    }
  }, []);

  const loadPresets = useCallback(async () => {
    try {
      setPresets(await invoke<PresetInfo[]>("list_presets"));
    } catch (err) {
      console.error(err);
    }
  }, []);

  useEffect(() => {
    loadSkills();
    loadMcps();
    loadInstructions();
    loadPresets();
    invoke<AgentLite[]>("list_agents")
      .then(setAgents)
      .catch(console.error);
  }, [loadSkills, loadMcps, loadInstructions, loadPresets]);

  const switchTab = (t: Tab) => {
    setTab(t);
    setSearch("");
    setActiveTag(null);
  };

  // ---------------- Skills handlers ----------------

  const openSkill = async (name: string) => {
    try {
      setSkillDetail(await invoke<SkillDetail>("read_library_skill", { name }));
      setSkillEditing(false);
      setSkillCreating(false);
    } catch (err) {
      alert(err);
    }
  };

  const startCreateSkill = () => {
    setSkillDetail(null);
    setSkillCreating(true);
    setSkillEditing(true);
    setSkillForm({ name: "", description: "", tagsText: "", content: "# 描述这个 skill 的用法\n" });
  };

  const startEditSkill = (name?: string) => {
    const target = name || skillDetail?.name;
    if (!target) return;
    invoke<SkillDetail>("read_library_skill", { name: target }).then((d) => {
      setSkillDetail(d);
      setSkillCreating(false);
      setSkillEditing(true);
      setSkillForm({
        name: d.name,
        description: d.description,
        tagsText: d.tags.join(", "),
        content: d.content,
      });
    });
  };

  const saveSkill = async () => {
    const tags = skillForm.tagsText
      .split(/[,，]/)
      .map((t) => t.trim())
      .filter(Boolean);
    try {
      if (skillCreating) {
        await invoke("create_library_skill", {
          name: skillForm.name,
          description: skillForm.description,
          tags,
          content: skillForm.content,
        });
      } else if (skillDetail) {
        await invoke("update_library_skill", {
          name: skillDetail.name,
          newName: skillForm.name,
          description: skillForm.description,
          tags,
          content: skillForm.content,
        });
      }
      await loadSkills();
      await openSkill(skillForm.name.trim());
    } catch (err) {
      alert(err);
    }
  };

  const deleteSkill = async (name: string) => {
    if (!confirm(`删除 skill「${name}」？\n\n此操作会删除库目录,不可恢复。`)) return;
    try {
      await invoke("delete_library_skill", { name });
      if (skillDetail?.name === name) setSkillDetail(null);
      await loadSkills();
    } catch (err) {
      alert(err);
    }
  };

  const importSkill = async (sourceDir?: string) => {
    let dir = sourceDir;
    if (!dir) {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || typeof selected !== "string") return;
      dir = selected;
    }
    try {
      await invoke("import_library_skill", { sourceDir: dir, name: null });
      await loadSkills();
    } catch (err) {
      alert(err);
    }
  };

  const exportSkill = async (name: string) => {
    const dest = await open({ directory: true, multiple: false });
    if (!dest || typeof dest !== "string") return;
    try {
      const out = await invoke<string>("export_library_skill", { name, destDir: dest });
      alert(`已导出到: ${out}`);
    } catch (err) {
      alert(err);
    }
  };

  // ---------------- MCP handlers ----------------

  const startCreateMcp = () => {
    setMcpCreating(true);
    setMcpEditing(true);
    setMcpEditingName(null);
    setImportCandidates(null);
    setMcpForm({ name: "", serverType: "stdio", command: "", argsText: "", envPairs: [], url: "" });
  };

  const startEditMcp = async (name: string) => {
    const list = await invoke<McpServerInfo[]>("list_library_mcps");
    const found = list.find((m) => m.name === name);
    if (!found) return;
    const isRemote = found.server_type === "sse" || found.server_type === "http";
    setMcpCreating(false);
    setMcpEditing(true);
    setMcpEditingName(name);
    setImportCandidates(null);
    setMcpForm({
      name: found.name,
      serverType: isRemote ? (found.server_type as "sse" | "http") : "stdio",
      command: found.command === "(http)" ? "" : found.command,
      argsText: isRemote ? "" : found.args.join("\n"),
      envPairs: Object.entries(found.env || {}).map(([key, value]) => ({ key, value })),
      url: found.url || "",
    });
  };

  // MCP being edited (its original name — the form's name field may rename it)
  const [mcpEditingName, setMcpEditingName] = useState<string | null>(null);

  const saveMcp = async () => {
    const entry = {
      name: mcpForm.name,
      command: mcpForm.command,
      args: mcpForm.argsText.split("\n").map((s) => s.trim()).filter(Boolean),
      env: Object.fromEntries(
        mcpForm.envPairs.filter((p) => p.key.trim()).map((p) => [p.key.trim(), p.value]),
      ),
      type: mcpForm.serverType,
      url: mcpForm.url || null,
    };
    try {
      if (mcpCreating) {
        await invoke("create_library_mcp", { entry });
      } else if (mcpEditingName) {
        await invoke("update_library_mcp", { oldName: mcpEditingName, entry });
      }
      await loadMcps();
      setMcpEditing(false);
      setMcpCreating(false);
      setMcpEditingName(null);
    } catch (err) {
      alert(err);
    }
  };

  const deleteMcp = async (name: string) => {
    if (!confirm(`删除 MCP「${name}」？`)) return;
    try {
      await invoke("delete_library_mcp", { name });
      await loadMcps();
    } catch (err) {
      alert(err);
    }
  };

  const adoptMcp = async (name: string) => {
    try {
      await invoke("adopt_scanned_mcp", { name });
      await loadMcps();
    } catch (err) {
      alert(err);
    }
  };

  const startMcpImport = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "MCP 配置", extensions: ["json", "toml"] }],
    });
    if (!selected || typeof selected !== "string") return;
    try {
      const candidates = await invoke<McpImportCandidate[]>("parse_mcp_import_file", {
        path: selected,
      });
      if (candidates.length === 0) {
        alert("文件中没有可导入的 MCP 配置");
        return;
      }
      setImportSourcePath(selected);
      setImportCandidates(candidates);
      setImportChecked(candidates.filter((c) => !c.exists_in_library).map((c) => c.name));
      setMcpEditing(false);
    } catch (err) {
      alert(err);
    }
  };

  const commitMcpImport = async () => {
    try {
      const result = await invoke<{ imported: string[]; skipped: string[] }>(
        "import_library_mcps",
        { path: importSourcePath, names: importChecked },
      );
      setImportCandidates(null);
      await loadMcps();
      const parts: string[] = [];
      if (result.imported.length) parts.push(`已导入 ${result.imported.length} 个`);
      if (result.skipped.length) parts.push(`跳过 ${result.skipped.length} 个(已存在)`);
      alert(parts.join(", ") || "没有导入任何配置");
    } catch (err) {
      alert(err);
    }
  };

  const exportMcp = async (name: string) => {
    const dest = await open({
      save: true,
      defaultPath: `mcp-${name}.json`,
      filters: [
        { name: "Claude JSON", extensions: ["json"] },
        { name: "Codex TOML", extensions: ["toml"] },
      ],
    });
    if (!dest || typeof dest !== "string") return;
    const format = dest.endsWith(".toml") ? "codex-toml" : "claude-json";
    try {
      const out = await invoke<string>("export_library_mcps", {
        names: [name],
        destPath: dest,
        format,
      });
      alert(`已导出到: ${out}`);
    } catch (err) {
      alert(err);
    }
  };

  // ---------------- Instruction handlers ----------------

  const copyInst = async (name: string) => {
    try {
      const detail = await invoke<{ name: string; path: string; content: string }>(
        "read_library_instruction",
        { name },
      );
      await copyText(stripFrontmatter(detail.content));
      setCopiedInst(name);
      setTimeout(() => setCopiedInst((c) => (c === name ? null : c)), 1500);
    } catch (err) {
      alert(err);
    }
  };

  const startCreateInst = () => {
    setActiveInst(null);
    setInstCreating(true);
    setInstEditing(true);
    setInstForm({ name: "", content: "" });
  };

  const startEditInst = async (name: string) => {
    try {
      const detail = await invoke<{ name: string; path: string; content: string }>(
        "read_library_instruction",
        { name },
      );
      setActiveInst(name);
      setInstCreating(false);
      setInstEditing(true);
      setInstForm({ name: detail.name, content: detail.content });
    } catch (err) {
      alert(err);
    }
  };

  const saveInst = async () => {
    try {
      if (instCreating) {
        await invoke("create_library_instruction", {
          name: instForm.name,
          content: instForm.content,
        });
      } else if (activeInst) {
        await invoke("update_library_instruction", {
          name: activeInst,
          newName: instForm.name,
          content: instForm.content,
        });
      }
      await loadInstructions();
      setInstEditing(false);
      setInstCreating(false);
    } catch (err) {
      alert(err);
    }
  };

  const deleteInst = async (name: string) => {
    if (!confirm(`删除系统指令「${name}」？`)) return;
    try {
      await invoke("delete_library_instruction", { name });
      if (activeInst === name) setActiveInst(null);
      await loadInstructions();
    } catch (err) {
      alert(err);
    }
  };

  const importInst = async () => {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    try {
      const imported = await invoke<string[]>("import_library_instructions", { paths });
      await loadInstructions();
      alert(imported.length ? `已导入 ${imported.length} 个系统指令` : "没有新导入(重名跳过)");
    } catch (err) {
      alert(err);
    }
  };

  // ---------------- Expert handlers ----------------

  const startCreateExpert = () => {
    setExpertCreating(true);
    setExpertEditing(false);
    setExpertEditingName(null);
    setExpertForm(emptyExpertForm(agents.find((a) => a.installed)?.id || "claude"));
  };

  const startEditExpert = (p: PresetInfo) => {
    setExpertCreating(false);
    setExpertEditing(true);
    setExpertEditingName(p.name);
    setExpertForm(formFromPreset(p));
  };

  const saveExpert = async () => {
    const name = expertForm.name.trim();
    if (!name) {
      alert("请填写专家名称");
      return;
    }
    try {
      await invoke("save_preset", {
        input: {
          name: expertEditingName || name,
          // rename support: save under the new name; old row removed below
          ...(expertEditingName && expertEditingName !== name ? {} : {}),
          description: expertForm.description,
          agent_id: expertForm.agent_id,
          tags: expertForm.tagsText
            .split(/[,，]/)
            .map((t) => t.trim())
            .filter(Boolean),
          work_dir: expertForm.work_dir,
          icon: expertForm.icon,
          skills: expertForm.skills,
          mcps: expertForm.mcps,
          instructions: expertForm.instructions,
        },
      });
      if (expertEditingName && expertEditingName !== name) {
        await invoke("delete_preset", { name: expertEditingName });
      }
      setExpertEditing(false);
      setExpertCreating(false);
      setExpertEditingName(null);
      await loadPresets();
    } catch (err) {
      alert(err);
    }
  };

  const deleteExpert = async (p: PresetInfo) => {
    if (!confirm(`删除专家「${p.name}」？`)) return;
    try {
      await invoke("delete_preset", { name: p.name });
      await loadPresets();
    } catch (err) {
      alert(err);
    }
  };

  const pickExpertDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === "string") {
      setExpertForm({ ...expertForm, work_dir: selected });
    }
  };

  const launchExpert = async (p: PresetInfo) => {
    if (p.work_dir) {
      // Pre-stored dir: go to Launcher with the expert; Launcher auto-launches.
      navigate("/", { state: { expertLaunch: p.name } });
      return;
    }
    // No pre-stored dir: open the expert form so the user can store one,
    // or they can launch it from the Launcher after picking a directory.
    startEditExpert(p);
  };

  // ---------------- Filtered data ----------------

  const skillTags = useMemo(() => collectTags(skills), [skills]);
  const mcpTags = useMemo(() => collectTags([]), []); // MCPs have no tags yet
  const instTags = useMemo(() => collectTags(instructions), [instructions]);
  const expertTags = useMemo(() => collectTags(presets), [presets]);

  const filteredSkills = useMemo(
    () =>
      skills.filter(
        (s) =>
          matchesSearch(s, search) &&
          (!activeTag || (s.tags || []).includes(activeTag)),
      ),
    [skills, search, activeTag],
  );
  const filteredScannedSkills = useMemo(
    () =>
      scannedSkills.filter(
        (s) =>
          matchesSearch(s, search) &&
          (!activeTag || (s.tags || []).includes(activeTag)),
      ),
    [scannedSkills, search, activeTag],
  );
  const filteredMcps = useMemo(
    () => mcps.filter((m) => matchesSearch(m, search)),
    [mcps, search],
  );
  const filteredScannedMcps = useMemo(
    () => scannedMcps.filter((m) => matchesSearch(m, search)),
    [scannedMcps, search],
  );
  const filteredInsts = useMemo(
    () =>
      instructions.filter(
        (i) =>
          matchesSearch(i, search) &&
          (!activeTag || (i.tags || []).includes(activeTag)),
      ),
    [instructions, search, activeTag],
  );
  const filteredPresets = useMemo(
    () =>
      presets.filter(
        (p) =>
          matchesSearch(p, search) &&
          (!activeTag || (p.tags || []).includes(activeTag)),
      ),
    [presets, search, activeTag],
  );

  // ---------------- Render helpers ----------------

  const btnSecondary =
    "px-3 py-1.5 rounded-lg text-[12px] border transition-colors hover:bg-[var(--bg-tertiary)]";
  const btnDanger =
    "px-3 py-1.5 rounded-lg text-[12px] border transition-colors hover:opacity-80";
  const btnPrimary =
    "px-4 py-1.5 rounded-lg text-[12px] font-medium text-white transition-opacity hover:opacity-90";

  const renderEditorHeader = (title: string, actions: React.ReactNode) => (
    <div className="flex items-center justify-between mb-5">
      <h2 className="text-base font-semibold" style={{ color: 'var(--text-primary)' }}>
        {title}
      </h2>
      <div className="flex items-center gap-2">{actions}</div>
    </div>
  );

  const cardGrid = (children: React.ReactNode) => (
    <div className="grid grid-cols-3 gap-3">{children}</div>
  );

  const newCard = (label: string, onClick: () => void) => (
    <button
      key="__new__"
      onClick={onClick}
      className="rounded-2xl border border-dashed p-4 flex flex-col items-center justify-center gap-1.5 min-h-[120px] h-full transition-colors hover:bg-[var(--bg-tertiary)]"
      style={{ borderColor: 'var(--border)', color: 'var(--text-muted)' }}
    >
      <Plus size={18} />
      <span className="text-[12px]">{label}</span>
    </button>
  );

  const sectionHeader = (title: string, count: number) => (
    <div className="flex items-center gap-2 mt-1">
      <h3 className="text-[12px] font-semibold" style={{ color: 'var(--text-primary)' }}>
        {title}
      </h3>
      <span
        className="text-[10px] px-1.5 py-0.5 rounded-md"
        style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}
      >
        {count}
      </span>
    </div>
  );

  return (
    <div className="h-full flex flex-col">
      <header
        className="h-14 flex items-center px-6 border-b shrink-0 gap-6"
        style={{ borderColor: 'var(--border)' }}
      >
        <h1 className="text-base font-semibold" style={{ color: 'var(--text-primary)' }}>
          配置管理
        </h1>
        <div className="flex items-center gap-1 p-1 rounded-xl" style={{ background: 'var(--bg-tertiary)' }}>
          {TABS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => switchTab(id)}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[12px] font-medium transition-all"
              style={{
                background: tab === id ? 'var(--bg-primary)' : 'transparent',
                color: tab === id ? 'var(--text-primary)' : 'var(--text-muted)',
              }}
            >
              <Icon size={13} />
              {label}
            </button>
          ))}
        </div>
        <span
          className="ml-auto text-[11px] px-2 py-1 rounded-md"
          style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}
        >
          ~/.agenthub/library · 不改动 agent 原生配置
        </span>
      </header>

      <div className="flex-1 overflow-y-auto px-8 py-5">
        <div className="max-w-5xl mx-auto">
          {/* ============ EXPERTS ============ */}
          {tab === "experts" && (
            <>
              {expertEditing || expertCreating ? (
                <ExpertForm
                  creating={expertCreating}
                  form={expertForm}
                  setForm={setExpertForm}
                  agents={agents}
                  skills={[...skills, ...scannedSkills]}
                  mcps={[...mcps, ...scannedMcps]}
                  instructions={instructions}
                  onCancel={() => {
                    setExpertEditing(false);
                    setExpertCreating(false);
                    setExpertEditingName(null);
                  }}
                  onSave={saveExpert}
                  onPickDir={pickExpertDir}
                />
              ) : (
                <>
                  <div className="mb-4">
                    <CardWallToolbar
                      search={search}
                      onSearch={setSearch}
                      placeholder="搜索专家…"
                      allTags={expertTags}
                      activeTag={activeTag}
                      onTagClick={setActiveTag}
                      actions={
                        <button
                          onClick={startCreateExpert}
                          className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-[12px] font-medium text-white transition-opacity hover:opacity-90"
                          style={{ background: 'var(--accent-blue)' }}
                        >
                          <Plus size={14} />
                          新建专家
                        </button>
                      }
                    />
                  </div>
                  {presets.length === 0 ? (
                    <EmptyHint text="还没有专家 — 创建一个,把常用的 agent + skills + MCP + 系统指令组合固定下来" />
                  ) : (
                    cardGrid(
                      <>
                        {filteredPresets.map((p) => (
                          <ExpertCard
                            key={p.name}
                            preset={p}
                            agents={agents}
                            activeTag={activeTag}
                            onTagClick={setActiveTag}
                            onLaunch={launchExpert}
                            onEdit={startEditExpert}
                            onDelete={deleteExpert}
                          />
                        ))}
                        {newCard("新建专家", startCreateExpert)}
                      </>,
                    )
                  )}
                </>
              )}
            </>
          )}

          {/* ============ SKILLS ============ */}
          {tab === "skills" && (
            <>
              {skillEditing ? (
                <div className="max-w-2xl mx-auto">
                  {renderEditorHeader(
                    skillCreating ? "新建 Skill" : "编辑 Skill",
                    <>
                      <button
                        onClick={() => {
                          setSkillEditing(false);
                          setSkillCreating(false);
                        }}
                        className={btnSecondary}
                        style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}
                      >
                        取消
                      </button>
                      <button onClick={saveSkill} className={btnPrimary} style={{ background: 'var(--accent-blue)' }}>
                        保存
                      </button>
                    </>,
                  )}
                  <div className="space-y-4">
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        名称
                      </label>
                      <input
                        value={skillForm.name}
                        onChange={(e) => setSkillForm({ ...skillForm, name: e.target.value })}
                        placeholder="my-skill"
                        className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                        style={inputStyle}
                      />
                    </div>
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        描述
                      </label>
                      <input
                        value={skillForm.description}
                        onChange={(e) => setSkillForm({ ...skillForm, description: e.target.value })}
                        placeholder="这个 skill 做什么、什么时候用"
                        className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                        style={inputStyle}
                      />
                    </div>
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        标签 (逗号分隔)
                      </label>
                      <input
                        value={skillForm.tagsText}
                        onChange={(e) => setSkillForm({ ...skillForm, tagsText: e.target.value })}
                        placeholder="开发, 测试"
                        className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                        style={inputStyle}
                      />
                    </div>
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        内容 (Markdown 正文)
                      </label>
                      <textarea
                        value={skillForm.content}
                        onChange={(e) => setSkillForm({ ...skillForm, content: e.target.value })}
                        rows={18}
                        className="w-full px-3 py-2 rounded-xl text-[13px] font-mono border outline-none focus:border-[var(--accent-blue)] transition-colors resize-y"
                        style={inputStyle}
                      />
                    </div>
                  </div>
                </div>
              ) : (
                <>
                  <div className="mb-4">
                    <CardWallToolbar
                      search={search}
                      onSearch={setSearch}
                      placeholder="搜索 skill…"
                      allTags={skillTags}
                      activeTag={activeTag}
                      onTagClick={setActiveTag}
                      actions={
                        <>
                          <button
                            onClick={() => importSkill()}
                            className={btnSecondary}
                            style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}
                          >
                            导入目录
                          </button>
                          <button
                            onClick={startCreateSkill}
                            className={btnPrimary}
                            style={{ background: 'var(--accent-blue)' }}
                          >
                            新建 Skill
                          </button>
                        </>
                      }
                    />
                  </div>

                  {skillDetail && (
                    <div className="mb-5 p-5 rounded-2xl border" style={{ borderColor: 'var(--border-light)', background: 'var(--bg-secondary)' }}>
                      {renderEditorHeader(
                        skillDetail.name,
                        <>
                          <button onClick={() => startEditSkill(skillDetail.name)} className={btnSecondary}
                            style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                            编辑
                          </button>
                          <button onClick={() => exportSkill(skillDetail.name)} className={btnSecondary}
                            style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                            导出
                          </button>
                          <button onClick={() => deleteSkill(skillDetail.name)} className={btnDanger}
                            style={{ borderColor: 'rgba(239,68,68,0.35)', color: 'var(--accent-red, #ef4444)' }}>
                            删除
                          </button>
                        </>,
                      )}
                      <p className="text-[13px] mb-3" style={{ color: 'var(--text-secondary)' }}>
                        {skillDetail.description || <span style={{ color: 'var(--text-muted)' }}>无描述</span>}
                      </p>
                      <p className="text-[11px] font-mono mb-1" style={{ color: 'var(--text-muted)' }}>
                        {skillDetail.path}
                      </p>
                      {skillDetail.files.length > 1 && (
                        <p className="text-[11px] mb-3" style={{ color: 'var(--text-muted)' }}>
                          包含文件: {skillDetail.files.join(", ")}
                        </p>
                      )}
                      <pre
                        className="text-[12px] font-mono whitespace-pre-wrap p-4 rounded-xl border"
                        style={{ ...inputStyle, borderColor: 'var(--border-light)' }}
                      >
                        {skillDetail.content}
                      </pre>
                    </div>
                  )}

                  <div className="space-y-4">
                    <section>
                      {sectionHeader("我的托管库", skills.length)}
                      <div className="mt-2.5">
                        {skills.length === 0 ? (
                          <p className="text-[12px] py-3" style={{ color: 'var(--text-muted)' }}>
                            托管库为空 — 新建或从「本机发现」导入
                          </p>
                        ) : (
                          cardGrid(
                            <>
                              {filteredSkills.map((s) => (
                                <LibraryCard
                                  key={s.name}
                                  icon="🧩"
                                  title={s.name}
                                  description={s.description}
                                  tags={s.tags}
                                  activeTag={activeTag}
                                  onTagClick={setActiveTag}
                                  source={s.source}
                                  onOpen={() => openSkill(s.name)}
                                  onEdit={() => startEditSkill(s.name)}
                                  onDelete={() => deleteSkill(s.name)}
                                  onExport={() => exportSkill(s.name)}
                                />
                              ))}
                              {newCard("新建 Skill", startCreateSkill)}
                            </>,
                          )
                        )}
                      </div>
                    </section>

                    {scannedSkills.length > 0 && (
                      <section>
                        {sectionHeader("本机发现", scannedSkills.length)}
                        <div className="mt-2.5">
                          {cardGrid(
                            filteredScannedSkills.map((s) => (
                              <LibraryCard
                                key={s.name}
                                icon="🧩"
                                title={s.name}
                                description={s.description}
                                tags={s.tags}
                                activeTag={activeTag}
                                onTagClick={setActiveTag}
                                source={s.source}
                                onImport={() => importSkill(s.path.replace("/SKILL.md", ""))}
                              />
                            )),
                          )}
                        </div>
                      </section>
                    )}
                  </div>
                </>
              )}
            </>
          )}

          {/* ============ MCPS ============ */}
          {tab === "mcps" && (
            <>
              {importCandidates ? (
                <div className="max-w-2xl mx-auto">
                  {renderEditorHeader(
                    "导入 MCP 配置",
                    <>
                      <button onClick={() => setImportCandidates(null)} className={btnSecondary}
                        style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                        取消
                      </button>
                      <button onClick={commitMcpImport} disabled={importChecked.length === 0}
                        className={btnPrimary} style={{ background: 'var(--accent-blue)' }}>
                        导入所选 ({importChecked.length})
                      </button>
                    </>,
                  )}
                  <p className="text-[12px] mb-4" style={{ color: 'var(--text-muted)' }}>
                    来源: <span className="font-mono">{importSourcePath}</span>
                  </p>
                  <div className="space-y-2">
                    {importCandidates.map((c) => (
                      <label
                        key={c.name}
                        className="flex items-start gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer"
                        style={{
                          background: importChecked.includes(c.name)
                            ? 'rgba(79,110,247,0.06)'
                            : 'var(--bg-secondary)',
                          opacity: c.exists_in_library ? 0.65 : 1,
                        }}
                      >
                        <input
                          type="checkbox"
                          className="mt-0.5 w-4 h-4 rounded accent-[var(--accent-blue)]"
                          checked={importChecked.includes(c.name)}
                          onChange={() =>
                            setImportChecked((prev) =>
                              prev.includes(c.name)
                                ? prev.filter((n) => n !== c.name)
                                : [...prev, c.name],
                            )
                          }
                        />
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span className="text-[13px] font-medium" style={{ color: 'var(--text-primary)' }}>
                              {c.name}
                            </span>
                            {c.exists_in_library && (
                              <span className="text-[10px] px-1.5 py-0.5 rounded-md"
                                style={{ background: 'var(--bg-tertiary)', color: 'var(--accent-yellow, #ca8a04)' }}>
                                库中已存在 · 将跳过
                              </span>
                            )}
                          </div>
                          <div className="text-[11px] font-mono mt-0.5" style={{ color: 'var(--text-muted)' }}>
                            {c.command} {c.args.join(" ")}
                            {c.env_keys.length > 0 && ` · env: ${c.env_keys.join(", ")}`}
                          </div>
                        </div>
                      </label>
                    ))}
                  </div>
                </div>
              ) : mcpEditing ? (
                <div className="max-w-2xl mx-auto">
                  {renderEditorHeader(
                    mcpCreating ? "新建 MCP 服务器" : "编辑 MCP 服务器",
                    <>
                      <button onClick={() => { setMcpEditing(false); setMcpCreating(false); }}
                        className={btnSecondary}
                        style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                        取消
                      </button>
                      <button onClick={saveMcp} className={btnPrimary} style={{ background: 'var(--accent-blue)' }}>
                        保存
                      </button>
                    </>,
                  )}
                  <div className="space-y-4">
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        名称
                      </label>
                      <input value={mcpForm.name}
                        onChange={(e) => setMcpForm({ ...mcpForm, name: e.target.value })}
                        placeholder="my-mcp-server"
                        className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                        style={inputStyle} />
                    </div>
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        类型
                      </label>
                      <select value={mcpForm.serverType}
                        onChange={(e) =>
                          setMcpForm({ ...mcpForm, serverType: e.target.value as typeof mcpForm.serverType })
                        }
                        className="px-3 py-2 rounded-xl text-[13px] border outline-none"
                        style={inputStyle}>
                        <option value="stdio">stdio (本地进程)</option>
                        <option value="sse">sse (远程)</option>
                        <option value="http">http (远程)</option>
                      </select>
                    </div>
                    {mcpForm.serverType === "stdio" ? (
                      <>
                        <div>
                          <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                            命令
                          </label>
                          <input value={mcpForm.command}
                            onChange={(e) => setMcpForm({ ...mcpForm, command: e.target.value })}
                            placeholder="npx"
                            className="w-full px-3 py-2 rounded-xl text-[13px] font-mono border outline-none focus:border-[var(--accent-blue)] transition-colors"
                            style={inputStyle} />
                        </div>
                        <div>
                          <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                            参数 (每行一个)
                          </label>
                          <textarea value={mcpForm.argsText}
                            onChange={(e) => setMcpForm({ ...mcpForm, argsText: e.target.value })}
                            rows={3}
                            placeholder={"-y\n@some/mcp-server"}
                            className="w-full px-3 py-2 rounded-xl text-[13px] font-mono border outline-none focus:border-[var(--accent-blue)] transition-colors resize-y"
                            style={inputStyle} />
                        </div>
                      </>
                    ) : (
                      <div>
                        <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                          URL
                        </label>
                        <input value={mcpForm.url}
                          onChange={(e) => setMcpForm({ ...mcpForm, url: e.target.value })}
                          placeholder="https://example.com/mcp"
                          className="w-full px-3 py-2 rounded-xl text-[13px] font-mono border outline-none focus:border-[var(--accent-blue)] transition-colors"
                          style={inputStyle} />
                      </div>
                    )}
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        环境变量
                      </label>
                      <div className="space-y-2">
                        {mcpForm.envPairs.map((pair, idx) => (
                          <div key={idx} className="flex items-center gap-2">
                            <input value={pair.key}
                              onChange={(e) =>
                                setMcpForm({
                                  ...mcpForm,
                                  envPairs: mcpForm.envPairs.map((p, i) =>
                                    i === idx ? { ...p, key: e.target.value } : p,
                                  ),
                                })
                              }
                              placeholder="KEY"
                              className="flex-1 px-3 py-1.5 rounded-lg text-[12px] font-mono border outline-none"
                              style={inputStyle} />
                            <input value={pair.value}
                              onChange={(e) =>
                                setMcpForm({
                                  ...mcpForm,
                                  envPairs: mcpForm.envPairs.map((p, i) =>
                                    i === idx ? { ...p, value: e.target.value } : p,
                                  ),
                                })
                              }
                              placeholder="value"
                              className="flex-1 px-3 py-1.5 rounded-lg text-[12px] font-mono border outline-none"
                              style={inputStyle} />
                            <button onClick={() =>
                              setMcpForm({
                                ...mcpForm,
                                envPairs: mcpForm.envPairs.filter((_, i) => i !== idx),
                              })
                            }
                              className="px-2 py-1 rounded-md text-[12px] hover:bg-[var(--bg-tertiary)]"
                              style={{ color: 'var(--text-muted)' }}>
                              ✕
                            </button>
                          </div>
                        ))}
                        <button onClick={() =>
                          setMcpForm({ ...mcpForm, envPairs: [...mcpForm.envPairs, { key: "", value: "" }] })
                        }
                          className="text-[12px] px-2 py-1 rounded-md hover:bg-[var(--bg-tertiary)]"
                          style={{ color: 'var(--accent-blue)' }}>
                          + 添加变量
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              ) : (
                <>
                  <div className="mb-4">
                    <CardWallToolbar
                      search={search}
                      onSearch={setSearch}
                      placeholder="搜索 MCP…"
                      allTags={mcpTags}
                      activeTag={activeTag}
                      onTagClick={setActiveTag}
                      actions={
                        <>
                          <button onClick={startMcpImport} className={btnSecondary}
                            style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                            导入配置文件
                          </button>
                          <button onClick={startCreateMcp} className={btnPrimary}
                            style={{ background: 'var(--accent-blue)' }}>
                            新建 MCP
                          </button>
                        </>
                      }
                    />
                  </div>

                  <div className="space-y-4">
                    <section>
                      {sectionHeader("我的托管库", mcps.length)}
                      <div className="mt-2.5">
                        {mcps.length === 0 ? (
                          <p className="text-[12px] py-3" style={{ color: 'var(--text-muted)' }}>
                            托管库为空 — 新建、导入配置文件或从「本机发现」导入
                          </p>
                        ) : (
                          cardGrid(
                            <>
                              {filteredMcps.map((m) => (
                                <LibraryCard
                                  key={m.name}
                                  icon="🔌"
                                  title={m.name}
                                  description={`${m.command} ${m.args.join(" ")}${
                                    Object.keys(m.env || {}).length
                                      ? ` · env: ${Object.keys(m.env).join(", ")}`
                                      : ""
                                  }`}
                                  subtitle={undefined}
                                  source={m.source}
                                  onEdit={() => startEditMcp(m.name)}
                                  onDelete={() => deleteMcp(m.name)}
                                  onExport={() => exportMcp(m.name)}
                                />
                              ))}
                              {newCard("新建 MCP", startCreateMcp)}
                            </>,
                          )
                        )}
                      </div>
                    </section>

                    {scannedMcps.length > 0 && (
                      <section>
                        {sectionHeader("本机发现", scannedMcps.length)}
                        <div className="mt-2.5">
                          {cardGrid(
                            filteredScannedMcps.map((m) => (
                              <LibraryCard
                                key={m.name}
                                icon="🔌"
                                title={m.name}
                                description={`${m.command} ${m.args.join(" ")}`}
                                source={m.source}
                                onImport={() => adoptMcp(m.name)}
                              />
                            )),
                          )}
                        </div>
                      </section>
                    )}
                  </div>
                </>
              )}
            </>
          )}

          {/* ============ INSTRUCTIONS ============ */}
          {tab === "instructions" && (
            <>
              {instEditing ? (
                <div className="max-w-2xl mx-auto">
                  {renderEditorHeader(
                    instCreating ? "新建系统指令" : "编辑系统指令",
                    <>
                      <button onClick={() => { setInstEditing(false); setInstCreating(false); }}
                        className={btnSecondary}
                        style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                        取消
                      </button>
                      <button onClick={saveInst} className={btnPrimary} style={{ background: 'var(--accent-blue)' }}>
                        保存
                      </button>
                    </>,
                  )}
                  <div className="space-y-4">
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        名称 (文件名)
                      </label>
                      <input value={instForm.name}
                        onChange={(e) => setInstForm({ ...instForm, name: e.target.value })}
                        placeholder="code-style"
                        className="w-full px-3 py-2 rounded-xl text-[13px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
                        style={inputStyle} />
                    </div>
                    <div>
                      <label className="text-[12px] font-medium block mb-1.5" style={{ color: 'var(--text-secondary)' }}>
                        内容 (Markdown,可含 frontmatter)
                      </label>
                      <textarea value={instForm.content}
                        onChange={(e) => setInstForm({ ...instForm, content: e.target.value })}
                        rows={20}
                        placeholder="描述这个 agent 应该遵循的规范、风格、约束……"
                        className="w-full px-3 py-2 rounded-xl text-[13px] font-mono border outline-none focus:border-[var(--accent-blue)] transition-colors resize-y"
                        style={inputStyle} />
                    </div>
                  </div>
                </div>
              ) : (
                <>
                  <div className="mb-4">
                    <CardWallToolbar
                      search={search}
                      onSearch={setSearch}
                      placeholder="搜索系统指令…"
                      allTags={instTags}
                      activeTag={activeTag}
                      onTagClick={setActiveTag}
                      actions={
                        <>
                          <button onClick={importInst} className={btnSecondary}
                            style={{ borderColor: 'var(--border)', color: 'var(--text-secondary)' }}>
                            导入 .md
                          </button>
                          <button onClick={startCreateInst} className={btnPrimary}
                            style={{ background: 'var(--accent-blue)' }}>
                            新建系统指令
                          </button>
                        </>
                      }
                    />
                  </div>

                  {instructions.length === 0 ? (
                    <EmptyHint text="系统指令库为空 — 新建或导入 .md 文件" />
                  ) : (
                    cardGrid(
                      <>
                        {filteredInsts.map((i) => (
                          <LibraryCard
                            key={i.name}
                            icon="📄"
                            title={i.name}
                            description={i.description}
                            tags={i.tags}
                            activeTag={activeTag}
                            onTagClick={setActiveTag}
                            source={i.source}
                            onOpen={() => startEditInst(i.name)}
                            onEdit={() => startEditInst(i.name)}
                            onDelete={() => deleteInst(i.name)}
                            onCopy={() => copyInst(i.name)}
                            copied={copiedInst === i.name}
                          />
                        ))}
                        {newCard("新建系统指令", startCreateInst)}
                      </>,
                    )
                  )}
                </>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function EmptyHint({ text }: { text: string }) {
  return (
    <div className="py-20 flex flex-col items-center justify-center">
      <p className="text-sm" style={{ color: 'var(--text-muted)' }}>
        {text}
      </p>
    </div>
  );
}
