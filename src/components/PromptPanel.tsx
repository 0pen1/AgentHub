import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Search,
  Copy,
  Check,
  Send,
  X,
  Plus,
  Trash2,
  Pencil,
  FileText,
} from "lucide-react";
import { copyText } from "../lib/clipboard";

interface PromptInfo {
  name: string;
  path: string;
  description: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  /** Send content into the active session's PTY (payload assembled by parent). */
  onSend: (content: string) => void;
  /** Whether the active session is running (enables the Send action). */
  canSend: boolean;
}

/**
 * Strip YAML frontmatter from a prompt's raw content — metadata belongs to
 * the library, not to the text a user pastes into a conversation.
 */
export function stripFrontmatter(raw: string): string {
  return raw.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, "");
}

/**
 * Slide-out panel for 常用提示词 (saved prompt snippets) in the session
 * terminal. Each snippet can be copied to the clipboard or sent straight
 * into the running session. Snippets are managed here inline (add /
 * rename / delete) — this library is separate from the session-injected
 * 系统指令 (instructions).
 */
export default function PromptPanel({
  open,
  onClose,
  onSend,
  canSend,
}: Props) {
  const [prompts, setPrompts] = useState<PromptInfo[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [search, setSearch] = useState("");
  // Transient per-item action feedback (Copy → Check, Send → 已发送).
  const [copiedName, setCopiedName] = useState<string | null>(null);
  const [sentName, setSentName] = useState<string | null>(null);
  // Inline editor state (create or rename).
  const [editing, setEditing] = useState<null | { original: string | null; name: string; content: string }>(null);
  const feedbackTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const load = () => {
    invoke<PromptInfo[]>("list_prompts")
      .then((list) => {
        setPrompts(list);
        setLoaded(true);
      })
      .catch((err) => console.error("list_prompts failed:", err));
  };

  // Refresh on each open so edits from anywhere show up.
  useEffect(() => {
    if (open) load();
  }, [open]);

  // Esc closes the panel (or the inline editor first).
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (editing) {
          setEditing(null);
        } else {
          onClose();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, editing, onClose]);

  // Clear any pending feedback timers on unmount.
  useEffect(() => {
    const timers = feedbackTimers.current;
    return () => timers.forEach((t) => clearTimeout(t));
  }, []);

  const flash = (key: string, set: (v: string | null) => void) => {
    set(key);
    const old = feedbackTimers.current.get(key);
    if (old) clearTimeout(old);
    feedbackTimers.current.set(
      key,
      setTimeout(() => {
        set(null);
        feedbackTimers.current.delete(key);
      }, 1500),
    );
  };

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return prompts
      .filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          p.description.toLowerCase().includes(q),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [prompts, search]);

  const handleCopy = async (name: string) => {
    try {
      const content = await invoke<string>("read_prompt", { name });
      await copyText(stripFrontmatter(content));
      flash(`copy:${name}`, setCopiedName);
    } catch (err) {
      alert(err);
    }
  };

  const handleSend = async (name: string) => {
    if (!canSend) return;
    try {
      const content = await invoke<string>("read_prompt", { name });
      onSend(stripFrontmatter(content));
      flash(`send:${name}`, setSentName);
    } catch (err) {
      alert(err);
    }
  };

  const saveEditing = async () => {
    if (!editing) return;
    const { original, name, content } = editing;
    if (!name.trim() || !content.trim()) return;
    try {
      if (original === null) {
        await invoke("create_prompt", { name: name.trim(), content });
      } else {
        await invoke("update_prompt", { name: original, newName: name.trim(), content });
      }
      setEditing(null);
      load();
    } catch (err) {
      alert(err);
    }
  };

  const handleDelete = async (name: string) => {
    if (!confirm(`删除常用提示词「${name}」？`)) return;
    try {
      await invoke("delete_prompt", { name });
      load();
    } catch (err) {
      alert(err);
    }
  };

  const startCreate = async () => {
    // Prefill from clipboard — the common flow is "save what I just have".
    let content = "";
    try {
      content = await navigator.clipboard.readText().catch(() => "");
    } catch {
      // read permission not granted — start empty
    }
    setEditing({ original: null, name: "", content });
  };

  return (
    <div
      className={`absolute top-0 right-0 h-full w-80 z-20 border-l flex flex-col transition-transform duration-200 ${
        open ? "translate-x-0" : "translate-x-full invisible pointer-events-none"
      }`}
      style={{
        background: 'var(--bg-secondary)',
        borderColor: 'var(--border-light)',
      }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 h-11 shrink-0 border-b"
        style={{ borderColor: 'var(--border-light)' }}>
        <span className="text-[12px] font-medium" style={{ color: 'var(--text-primary)' }}>
          常用提示词
        </span>
        <div className="flex items-center gap-0.5">
          <button
            onClick={startCreate}
            className="p-1 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
            style={{ color: 'var(--accent-blue)' }}
            title="新建（从剪贴板粘贴）"
          >
            <Plus size={14} />
          </button>
          <button
            onClick={onClose}
            className="p-1 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
            style={{ color: 'var(--text-muted)' }}
            title="关闭 (Esc)"
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* Inline editor */}
      {editing && (
        <div className="px-3 pt-3 pb-2 shrink-0 border-b" style={{ borderColor: 'var(--border-light)' }}>
          <input
            type="text"
            value={editing.name}
            onChange={(e) => setEditing({ ...editing, name: e.target.value })}
            placeholder="名称"
            autoFocus
            className="w-full px-3 py-1.5 rounded-lg text-[12px] border outline-none mb-2"
            style={{
              background: 'var(--bg-primary)',
              borderColor: 'var(--border)',
              color: 'var(--text-primary)',
            }}
          />
          <textarea
            value={editing.content}
            onChange={(e) => setEditing({ ...editing, content: e.target.value })}
            placeholder="提示词内容"
            rows={5}
            className="w-full px-3 py-1.5 rounded-lg text-[12px] border outline-none mb-2 resize-y"
            style={{
              background: 'var(--bg-primary)',
              borderColor: 'var(--border)',
              color: 'var(--text-primary)',
            }}
          />
          <div className="flex items-center gap-2 justify-end">
            <button
              onClick={() => setEditing(null)}
              className="px-2.5 py-1 rounded-md text-[11px] hover:bg-[var(--bg-tertiary)]"
              style={{ color: 'var(--text-muted)' }}
            >
              取消
            </button>
            <button
              onClick={saveEditing}
              disabled={!editing.name.trim() || !editing.content.trim()}
              className="px-2.5 py-1 rounded-md text-[11px] font-medium text-white disabled:opacity-30"
              style={{ background: 'var(--accent-blue)' }}
            >
              {editing.original === null ? "保存" : "更新"}
            </button>
          </div>
        </div>
      )}

      {/* Search */}
      {!editing && (
        <div className="px-3 pt-3 pb-2 shrink-0">
          <div className="relative">
            <Search size={13} className="absolute left-3 top-1/2 -translate-y-1/2"
              style={{ color: 'var(--text-muted)' }} />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="搜索提示词..."
              className="w-full pl-8 pr-3 py-1.5 rounded-lg text-[12px] border outline-none"
              style={{
                background: 'var(--bg-primary)',
                borderColor: 'var(--border)',
                color: 'var(--text-primary)',
              }}
            />
          </div>
        </div>
      )}

      {/* List */}
      <div className="flex-1 overflow-y-auto px-2 pb-2">
        {filtered.map((p) => {
          const isCopied = copiedName === p.name;
          const isSent = sentName === p.name;
          return (
            <div
              key={p.path}
              className="group/item px-2.5 py-2 rounded-xl mb-1"
              style={{ background: 'var(--bg-primary)' }}
            >
              <div className="flex items-start gap-2">
                <div className="min-w-0 flex-1">
                  <div className="text-[12px] font-medium truncate" style={{ color: 'var(--text-primary)' }}
                    title={p.name}>
                    {p.name}
                  </div>
                  {p.description && (
                    <div className="text-[11px] mt-0.5 line-clamp-2" style={{ color: 'var(--text-muted)' }}>
                      {p.description}
                    </div>
                  )}
                </div>
                <div className="flex items-center gap-0.5 shrink-0 opacity-0 group-hover/item:opacity-100 transition-opacity">
                  <button
                    onClick={() =>
                      invoke<string>("read_prompt", { name: p.name })
                        .then((content) => setEditing({ original: p.name, name: p.name, content }))
                        .catch(alert)
                    }
                    className="p-1 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
                    style={{ color: 'var(--text-muted)' }}
                    title="编辑"
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    onClick={() => handleDelete(p.name)}
                    className="p-1 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
                    style={{ color: 'var(--accent-red, #ef4444)' }}
                    title="删除"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              </div>
              <div className="flex items-center gap-1.5 mt-1.5">
                <button
                  onClick={() => handleCopy(p.name)}
                  className="flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-tertiary)]"
                  style={{ color: isCopied ? 'var(--accent-green)' : 'var(--text-secondary)' }}
                  title="复制到剪贴板"
                >
                  {isCopied ? <Check size={11} /> : <Copy size={11} />}
                  {isCopied ? "已复制" : "复制"}
                </button>
                <button
                  onClick={() => handleSend(p.name)}
                  disabled={!canSend || isSent}
                  className="flex items-center gap-1 px-2 py-0.5 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-tertiary)] disabled:opacity-30 disabled:cursor-not-allowed"
                  style={{ color: isSent ? 'var(--accent-green)' : 'var(--accent-blue)' }}
                  title={canSend ? "直接发送到当前会话" : "会话未运行，无法发送"}
                >
                  <Send size={11} />
                  {isSent ? "已发送" : "发送"}
                </button>
              </div>
            </div>
          );
        })}
        {loaded && prompts.length === 0 && !editing && (
          <div className="flex flex-col items-center py-10 gap-2" style={{ color: 'var(--text-muted)' }}>
            <FileText size={24} className="opacity-30" />
            <p className="text-[12px] text-center px-4">
              还没有常用提示词
              <br />
              点 + 新建，或从剪贴板粘贴保存
            </p>
          </div>
        )}
        {loaded && prompts.length > 0 && filtered.length === 0 && (
          <p className="text-[12px] py-6 text-center" style={{ color: 'var(--text-muted)' }}>
            无匹配结果
          </p>
        )}
        {!loaded && (
          <p className="text-[12px] py-6 text-center" style={{ color: 'var(--text-muted)' }}>
            加载中…
          </p>
        )}
      </div>
    </div>
  );
}
