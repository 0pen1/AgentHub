import { NavLink, useLocation } from "react-router-dom";
import { SquarePen, MessageSquare, Settings2 } from "lucide-react";
import Launcher from "./pages/Launcher";
import Sessions from "./pages/Sessions";
import ConfigLib from "./pages/ConfigLib";

const navItems = [
  { path: "/", icon: SquarePen, label: "新会话" },
  { path: "/sessions", icon: MessageSquare, label: "会话列表" },
  { path: "/config", icon: Settings2, label: "配置管理" },
];

export default function App() {
  const location = useLocation();
  const currentPath = location.pathname;

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

        {/* Footer */}
        <div className="px-4 py-2.5 flex items-center gap-2" style={{ borderTop: '1px solid var(--border-light)' }}>
          <Settings2 size={14} style={{ color: 'var(--text-muted)' }} />
          <span className="text-[11px]" style={{ color: 'var(--text-muted)' }}>AgentHub v0.1.0</span>
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
