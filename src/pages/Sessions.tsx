import { useState, useEffect, useCallback, useRef } from "react";
import { TerminalSquare, Plus, X } from "lucide-react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import SessionTree from "../components/SessionTree";
import Terminal from "../components/Terminal";

export interface SessionInfo {
  id: string;
  agent_id: string;
  agent_name: string;
  name: string;
  work_dir: string;
  status: string;
  created_at: string;
}

// ---- Module-level PTY plumbing -------------------------------------------
// Sessions and their PTYs live in the Rust backend for the whole app
// lifetime, so their frontend plumbing is module-level too (NOT component
// state). This survives Launcher ↔ Sessions route changes and — critically —
// guarantees each session's Tauri event listeners are registered EXACTLY ONCE.
// The previous per-`sessions`-change re-subscribe leaked listeners (async
// listen() resolving after its own cleanup had already run), so every PTY
// chunk got written to the terminal TWICE → overlapping/ghosted text.
const buffers = new Map<string, Uint8Array>();
const writeFns = new Map<string, (data: Uint8Array) => void>();
const unlisteners = new Map<string, UnlistenFn[]>();
// Set by the mounted Sessions page so pty-closed can update live UI state.
let notifyClosed: ((sessionId: string) => void) | null = null;

function handlePtyOutput(sessionId: string, b64: string) {
  if (!b64) return;
  const raw = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  const writeFn = writeFns.get(sessionId);
  if (writeFn) {
    // Terminal is mounted — write directly
    writeFn(raw);
    return;
  }
  // Buffer until a Terminal mounts (512KB cap)
  const buf = buffers.get(sessionId) || new Uint8Array();
  const merged = new Uint8Array(buf.length + raw.length);
  merged.set(buf);
  merged.set(raw, buf.length);
  buffers.set(
    sessionId,
    merged.length > 512 * 1024 ? merged.slice(-512 * 1024) : merged
  );
}

/// Subscribe to a session's PTY events exactly once per app lifetime.
/// The map reservation happens SYNCHRONOUSLY before any await, so concurrent
/// effect runs can never double-subscribe the same session.
function ensureSubscribed(sessionId: string) {
  if (unlisteners.has(sessionId)) return;
  const uns: UnlistenFn[] = [];
  unlisteners.set(sessionId, uns); // reserve before awaiting

  listen<string>(`pty-output:${sessionId}`, (event) =>
    handlePtyOutput(sessionId, event.payload)
  ).then((un) => uns.push(un));

  listen(`pty-closed:${sessionId}`, () => notifyClosed?.(sessionId)).then(
    (un) => uns.push(un)
  );
}

export default function Sessions() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  // Per-session restart counter — forces Terminal remount only for the
  // restarted session, not for whichever session the user is viewing.
  const [restartTicks, setRestartTicks] = useState<Record<string, number>>({});
  // In-flight guard to prevent double-restart on fast double-click.
  const restartingRef = useRef<Set<string>>(new Set());
  const navigate = useNavigate();
  const location = useLocation();

  const refreshSessions = useCallback(async () => {
    try {
      const list = await invoke<SessionInfo[]>("list_sessions");
      setSessions(list);
      return list;
    } catch (err) {
      console.error("Failed to load sessions:", err);
      return [];
    }
  }, []);

  useEffect(() => {
    refreshSessions();
  }, [refreshSessions]);

  // Route PTY-closed notifications into this mount's state
  useEffect(() => {
    notifyClosed = (sid) =>
      setSessions((prev) =>
        prev.map((s) => (s.id === sid ? { ...s, status: "exited" } : s))
      );
    return () => {
      notifyClosed = null;
    };
  }, []);

  // Refresh session list when the backend signals changes (kill/exit/restart)
  useEffect(() => {
    const un = listen("sessions-changed", () => {
      refreshSessions();
    });
    return () => {
      un.then((f) => f());
    };
  }, [refreshSessions]);

  // Subscribe (once, app-wide) to every known session's PTY events
  useEffect(() => {
    sessions.forEach((s) => ensureSubscribed(s.id));
  }, [sessions]);

  // Handle a freshly-launched session passed from the Launcher via router state
  useEffect(() => {
    const state = location.state as { launched?: SessionInfo } | null;
    if (state?.launched) {
      const info = state.launched;
      setSessions((prev) => (prev.some((s) => s.id === info.id) ? prev : [...prev, info]));
      setActiveSession(info.id);
      // Clear the router state so refresh doesn't re-add it
      navigate("/sessions", { replace: true, state: {} });
    }
  }, [location.state, navigate]);

  const handleKill = useCallback(
    async (sessionId: string) => {
      try {
        await invoke("kill_session", { sessionId });
        await refreshSessions();
        if (activeSession === sessionId) {
          setActiveSession(null);
        }
      } catch (err) {
        console.error("Kill failed:", err);
      }
    },
    [activeSession, refreshSessions]
  );

  // Selecting a session auto-resumes it when it's not running: dead sessions
  // are restarted (claude picks up its previous conversation via --resume)
  // before their terminal mounts, so "open" and "continue" are one action.
  const handleSelect = useCallback(
    async (sessionId: string) => {
      setActiveSession(sessionId);
      const target = sessions.find((s) => s.id === sessionId);
      if (!target || target.status === "running") return;
      if (restartingRef.current.has(sessionId)) return;
      restartingRef.current.add(sessionId);
      try {
        await invoke("restart_session", { sessionId });
        // Clear buffered output from the previous run — the new run starts fresh
        buffers.delete(sessionId);
        setRestartTicks((t) => ({ ...t, [sessionId]: (t[sessionId] || 0) + 1 }));
        await refreshSessions();
      } catch (err) {
        console.error("Auto-resume failed:", err);
        alert(`继续会话失败: ${err}`);
      } finally {
        restartingRef.current.delete(sessionId);
      }
    },
    [sessions, refreshSessions]
  );

  const handleDelete = useCallback(
    async (sessionId: string) => {
      const name = sessions.find((s) => s.id === sessionId)?.name || sessionId;
      if (!confirm(`删除会话「${name}」？\n\n运行中的进程会被终止，会话记录和临时配置将被清除（Claude 对话历史保留）。`)) {
        return;
      }
      try {
        await invoke("delete_session", { sessionId });
        buffers.delete(sessionId);
        if (activeSession === sessionId) {
          setActiveSession(null);
        }
        await refreshSessions();
      } catch (err) {
        console.error("Delete failed:", err);
        alert(`删除失败: ${err}`);
      }
    },
    [sessions, activeSession, refreshSessions]
  );

  const handleTerminalData = useCallback((sessionId: string, data: string) => {
    invoke("pty_write", { sessionId, data }).catch(console.error);
  }, []);

  const handleTerminalResize = useCallback(
    (sessionId: string, cols: number, rows: number) => {
      invoke("pty_resize", { sessionId, cols, rows }).catch(console.error);
    },
    []
  );

  // Register the write function when a Terminal mounts
  const registerWriter = useCallback((sessionId: string, writeFn: (data: Uint8Array) => void) => {
    writeFns.set(sessionId, writeFn);
    // Flush output buffered while this session was in the background
    const buffered = buffers.get(sessionId);
    if (buffered && buffered.length > 0) {
      writeFn(buffered);
      buffers.delete(sessionId);
    }
  }, []);

  const unregisterWriter = useCallback((sessionId: string) => {
    writeFns.delete(sessionId);
  }, []);

  const activeSessionInfo = sessions.find((s) => s.id === activeSession);

  return (
    <div className="h-full flex flex-col">
      <div className="flex-1 flex overflow-hidden">
        {/* Session tree */}
        <div className="w-64 border-r overflow-y-auto shrink-0"
          style={{ borderColor: 'var(--border-light)', background: 'var(--bg-secondary)' }}>
          <div className="px-3 pt-3 pb-2 flex items-center justify-between">
            <span className="text-[11px] font-medium uppercase tracking-[0.05em]"
              style={{ color: 'var(--text-muted)' }}>
              会话
            </span>
            <button
              onClick={() => navigate("/")}
              className="p-1 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
              style={{ color: 'var(--accent-blue)' }}
              title="新建会话"
            >
              <Plus size={15} />
            </button>
          </div>
          <SessionTree
            sessions={sessions}
            activeId={activeSession}
            onSelect={handleSelect}
            onKill={handleKill}
            onDelete={handleDelete}
          />
        </div>

        {/* Terminal area */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {activeSession && activeSessionInfo ? (
            <>
              {/* Terminal header */}
              <div className="h-10 flex items-center justify-between px-4 shrink-0 border-b"
                style={{ borderColor: 'var(--border-light)', background: 'var(--bg-secondary)' }}>
                <div className="flex items-center gap-2 min-w-0">
                  <span className="text-[13px] font-medium truncate" style={{ color: 'var(--text-primary)' }}>
                    {activeSessionInfo.name}
                  </span>
                  <span className="text-[11px] px-1.5 py-0.5 rounded-md shrink-0"
                    style={{
                      background: activeSessionInfo.status === 'running' ? 'rgba(52,199,89,0.12)' : 'var(--bg-tertiary)',
                      color: activeSessionInfo.status === 'running' ? 'var(--accent-green)' : 'var(--text-muted)',
                    }}>
                    {activeSessionInfo.status}
                  </span>
                </div>
                <div className="flex items-center gap-1 shrink-0">
                  <button
                    onClick={() => handleKill(activeSessionInfo.id)}
                    className="p-1.5 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
                    style={{ color: 'var(--text-muted)' }}
                    title="终止会话"
                  >
                    <X size={15} />
                  </button>
                </div>
              </div>

              {/* Terminal */}
              <div className="flex-1 overflow-hidden">
                <Terminal
                  key={`${activeSessionInfo.id}-${restartTicks[activeSessionInfo.id] || 0}`}
                  sessionId={activeSessionInfo.id}
                  onMount={(writeFn) => registerWriter(activeSessionInfo.id, writeFn)}
                  onUnmount={() => unregisterWriter(activeSessionInfo.id)}
                  onData={(data) => handleTerminalData(activeSessionInfo.id, data)}
                  onResize={(cols, rows) => handleTerminalResize(activeSessionInfo.id, cols, rows)}
                />
              </div>
            </>
          ) : (
            <div className="h-full flex items-center justify-center">
              <div className="text-center" style={{ color: 'var(--text-muted)' }}>
                <TerminalSquare size={48} className="mx-auto mb-4 opacity-15" />
                <p className="text-sm">
                  {sessions.length === 0
                    ? "还没有会话，去新会话页创建一个"
                    : "选择一个会话查看终端"}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
