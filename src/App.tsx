import { useState } from "react";
import { NavLink, useLocation } from "react-router-dom";
import { SquarePen, MessageSquare, Settings2, RefreshCw, ArrowUpCircle } from "lucide-react";
import Launcher from "./pages/Launcher";
import Sessions from "./pages/Sessions";
import ConfigLib from "./pages/ConfigLib";
import {
  checkForUpdate,
  downloadAndInstall,
  relaunchApp,
  type UpdateProgress,
} from "./lib/updater";
import type { Update } from "@tauri-apps/plugin-updater";

const navItems = [
  { path: "/", icon: SquarePen, label: "新会话" },
  { path: "/sessions", icon: MessageSquare, label: "会话列表" },
  { path: "/config", icon: Settings2, label: "配置管理" },
];

type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "up-to-date" }
  | { phase: "available"; update: Update }
  | { phase: "downloading"; received: number; total?: number }
  | { phase: "installed" }
  | { phase: "error"; message: string };

const APP_VERSION = "0.2.0";

export default function App() {
  const location = useLocation();
  const currentPath = location.pathname;
  const [updateState, setUpdateState] = useState<UpdateState>({ phase: "idle" });

  const runUpdateCheck = async () => {
    if (updateState.phase === "checking" || updateState.phase === "downloading") return;
    setUpdateState({ phase: "checking" });
    try {
      const update = await checkForUpdate();
      setUpdateState(
        update ? { phase: "available", update } : { phase: "up-to-date" },
      );
    } catch (err) {
      setUpdateState({ phase: "error", message: String(err) });
    }
  };

  const runUpdateInstall = async (update: Update) => {
    setUpdateState({ phase: "downloading", received: 0 });
    const onProgress = ({ received, total }: UpdateProgress) => {
      setUpdateState((prev) =>
        prev.phase === "downloading" && received >= 0
          ? { phase: "downloading", received, total: total ?? prev.total }
          : prev,
      );
    };
    try {
      await downloadAndInstall(update, onProgress);
      setUpdateState({ phase: "installed" });
    } catch (err) {
      setUpdateState({ phase: "error", message: String(err) });
    }
  };

  return (
    <div className="flex h-screen w-screen" style={{ background: 'var(--bg-primary)' }}>
      {/* Sidebar */}
      <aside className="w-56 flex flex-col shrink-0"
        style={{ background: 'var(--bg-secondary)', borderRight: '1px solid var(--border-light)' }}>

        {/* macOS traffic light spacer + drag region */}
        <div className="h-[52px] shrink-0" style={{ WebkitAppRegion: 'drag' } as React.CSSProperties} />

        {/* Brand */}
        <div className="flex items-center gap-2.5 px-4 pb-4">
          <div className="w-[28px] h-[28px] rounded-[8px] flex items-center justify-center text-white text-[12px] font-bold"
            style={{ background: 'linear-gradient(135deg, #5b7fff 0%, #8b5cf6 100%)' }}>
            A
          </div>
          <span className="text-[14px] font-semibold tracking-[-0.01em]" style={{ color: 'var(--text-primary)' }}>
            AgentHub
          </span>
        </div>

        {/* Nav */}
        <nav className="flex-1 overflow-y-auto px-2.5 pb-3">
          <div className="space-y-[2px]">
            {navItems.map(({ path, icon: Icon, label }) => {
              const isActive = path === "/" ? currentPath === "/" : currentPath.startsWith(path);
              return (
                <NavLink key={path} to={path}
                  className="flex items-center gap-2.5 px-2.5 py-[7px] rounded-[8px] text-[13px] transition-smooth"
                  style={{
                    background: isActive ? 'rgba(0,0,0,0.05)' : 'transparent',
                    color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
                    fontWeight: isActive ? 500 : 400,
                  }}>
                  <Icon size={16} strokeWidth={1.7} />
                  <span>{label}</span>
                </NavLink>
              );
            })}
          </div>

          <div className="my-4 mx-2" style={{ borderTop: '1px solid var(--border-light)' }} />

          <p className="text-[11px] font-medium uppercase tracking-[0.05em] px-2.5 mb-2"
            style={{ color: 'var(--text-muted)' }}>
            项目
          </p>
          <p className="text-[12px] px-2.5 py-1" style={{ color: 'var(--text-muted)' }}>
            没有项目
          </p>

          <div className="my-4 mx-2" style={{ borderTop: '1px solid var(--border-light)' }} />

          <p className="text-[11px] font-medium uppercase tracking-[0.05em] px-2.5 mb-2"
            style={{ color: 'var(--text-muted)' }}>
            最近
          </p>
          <p className="text-[12px] px-2.5 py-1" style={{ color: 'var(--text-muted)' }}>
            暂无会话
          </p>
        </nav>

        {/* Footer: version + update check */}
        <div className="px-4 py-2.5 flex flex-col gap-1" style={{ borderTop: '1px solid var(--border-light)' }}>
          <div className="flex items-center gap-2">
            <Settings2 size={14} style={{ color: 'var(--text-muted)' }} />
            <span className="text-[11px]" style={{ color: 'var(--text-muted)' }}>AgentHub v{APP_VERSION}</span>
            <button
              onClick={runUpdateCheck}
              disabled={updateState.phase === "checking" || updateState.phase === "downloading"}
              className="ml-auto flex items-center gap-1 text-[11px] rounded-md px-1.5 py-0.5 transition-colors hover:bg-[var(--bg-tertiary)] disabled:opacity-40"
              style={{ color: 'var(--text-muted)' }}
              title="检查更新"
            >
              <RefreshCw size={11}
                style={updateState.phase === "checking" ? { animation: 'spin 0.8s linear infinite' } : undefined} />
            </button>
          </div>

          {/* Update status line */}
          {updateState.phase === "up-to-date" && (
            <span className="text-[11px]" style={{ color: 'var(--accent-green)' }}>已是最新版本</span>
          )}
          {updateState.phase === "error" && (
            <span className="text-[11px] truncate" style={{ color: 'var(--accent-red)' }} title={updateState.message}>
              检查失败：{updateState.message.slice(0, 60)}
            </span>
          )}
          {updateState.phase === "available" && (
            <div className="flex items-center gap-1.5">
              <ArrowUpCircle size={12} style={{ color: 'var(--accent-yellow)' }} />
              <span className="text-[11px]" style={{ color: 'var(--text-primary)' }}>
                可更新到 v{updateState.update.version}
              </span>
              <button
                onClick={() => runUpdateInstall(updateState.update)}
                className="ml-auto text-[11px] px-2 py-0.5 rounded-md text-white"
                style={{ background: 'var(--accent-blue)' }}
              >
                更新
              </button>
            </div>
          )}
          {updateState.phase === "downloading" && (
            <div className="flex flex-col gap-1">
              <span className="text-[11px]" style={{ color: 'var(--text-muted)' }}>
                下载中{updateState.total ? ` ${Math.round((updateState.received / updateState.total) * 100)}%` : "…"}
              </span>
              <div className="h-1 rounded-full overflow-hidden" style={{ background: 'var(--bg-tertiary)' }}>
                <div
                  className="h-full rounded-full transition-all"
                  style={{
                    width: updateState.total
                      ? `${Math.min(100, (updateState.received / updateState.total) * 100)}%`
                      : '40%',
                    background: 'var(--accent-blue)',
                  }}
                />
              </div>
            </div>
          )}
          {updateState.phase === "installed" && (
            <button
              onClick={relaunchApp}
              className="flex items-center gap-1.5 text-[11px] px-2 py-1 rounded-md text-white w-fit"
              style={{ background: 'var(--accent-green)' }}
              title="重启以完成安装"
            >
              <RefreshCw size={11} />
              安装完成 — 点击重启
            </button>
          )}
        </div>
      </aside>

      {/* Main */}
      <main className="flex-1 overflow-hidden flex flex-col">
        {/* Drag region for main area titlebar */}
        <div className="h-[52px] shrink-0" style={{ WebkitAppRegion: 'drag' } as React.CSSProperties} />
        <div className="flex-1 overflow-hidden relative">
          {/* Keep-alive: all pages stay mounted; only visibility toggles */}
          <div style={{ display: currentPath === "/" ? "flex" : "none", flexDirection: "column", height: "100%" }}>
            <Launcher />
          </div>
          <div style={{ display: currentPath === "/sessions" ? "flex" : "none", flexDirection: "column", height: "100%" }}>
            <Sessions />
          </div>
          <div style={{ display: currentPath === "/config" ? "flex" : "none", flexDirection: "column", height: "100%" }}>
            <ConfigLib />
          </div>
        </div>
      </main>
    </div>
  );
}
