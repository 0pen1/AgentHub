import { useState } from "react";
import { Search, Plug } from "lucide-react";
import type { McpServerInfo } from "../pages/Launcher";

interface Props {
  servers: McpServerInfo[];
  selected: string[];
  onToggle: (name: string) => void;
}

export default function McpPicker({ servers: allServers, selected, onToggle }: Props) {
  const [search, setSearch] = useState("");
  const mcps = allServers.filter((m) =>
    m.name.toLowerCase().includes(search.toLowerCase()) ||
    m.command.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div>
      <h2 className="text-sm font-medium mb-3 flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
        <Plug size={15} style={{ color: 'var(--text-muted)' }} />
        MCP 服务器
        {allServers.length > 0 && (
          <span className="text-xs font-normal px-1.5 py-0.5 rounded-md"
            style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}>
            {allServers.length}
          </span>
        )}
      </h2>
      {allServers.length > 0 && (
        <div className="relative mb-3">
          <Search size={14} className="absolute left-3.5 top-1/2 -translate-y-1/2"
            style={{ color: 'var(--text-muted)' }} />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索 MCP 服务器..."
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
        {mcps.map((mcp) => (
          <label
            key={mcp.name}
            className="flex items-start gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer transition-colors"
            style={{ background: selected.includes(mcp.name) ? 'rgba(79,110,247,0.06)' : 'var(--bg-secondary)' }}
          >
            <input
              type="checkbox"
              checked={selected.includes(mcp.name)}
              onChange={() => onToggle(mcp.name)}
              className="mt-0.5 w-4 h-4 rounded accent-[var(--accent-blue)]"
            />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
                  {mcp.name}
                </span>
                <span className="text-xs px-1.5 py-0.5 rounded-md"
                  style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}>
                  {mcp.source}
                </span>
              </div>
              <div className="text-xs font-mono mt-0.5 truncate" style={{ color: 'var(--text-secondary)' }}>
                {mcp.command} {mcp.args.join(" ")}
              </div>
            </div>
          </label>
        ))}
        {allServers.length === 0 && (
          <p className="text-sm py-4 text-center" style={{ color: 'var(--text-muted)' }}>
            未发现 MCP 服务器配置
          </p>
        )}
        {allServers.length > 0 && mcps.length === 0 && (
          <p className="text-sm py-3 text-center" style={{ color: 'var(--text-muted)' }}>
            无匹配结果
          </p>
        )}
      </div>
    </div>
  );
}
