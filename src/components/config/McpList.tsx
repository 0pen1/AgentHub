import type { McpServerInfo, McpImportCandidate } from "../../pages/ConfigLib";

export interface McpFormValue {
  name: string;
  serverType: "stdio" | "sse" | "http";
  command: string;
  argsText: string; // one arg per line
  envPairs: { key: string; value: string }[];
  url: string;
}

interface Props {
  servers: McpServerInfo[];
  activeName: string | null;
  onSelect: (name: string) => void;
  onCreate: () => void;
  onImportFile: (path: string) => void;
}

export default function McpList({ servers, activeName, onSelect, onCreate, onImportFile }: Props) {
  return (
    <>
      <div className="px-3 pt-3 pb-2 flex items-center justify-between shrink-0">
        <span className="text-[11px] font-medium uppercase tracking-[0.05em]"
          style={{ color: 'var(--text-muted)' }}>
          MCP · {servers.length}
        </span>
        <div className="flex items-center gap-1">
          <button onClick={() => {
            // import handled by parent via dialog — trigger through callback with no path
            onImportFile("");
          }}
            className="px-2 py-1 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-tertiary)]"
            style={{ color: 'var(--text-secondary)' }}>
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
        {servers.map((mcp) => (
          <button key={mcp.name} onClick={() => onSelect(mcp.name)}
            className="w-full text-left px-3 py-2.5 rounded-lg transition-all"
            style={{ background: activeName === mcp.name ? 'var(--bg-tertiary)' : 'transparent' }}>
            <div className="text-[13px] font-medium truncate" style={{ color: 'var(--text-primary)' }}>
              {mcp.name}
            </div>
            <div className="text-[11px] font-mono truncate mt-0.5" style={{ color: 'var(--text-muted)' }}>
              {mcp.command} {mcp.args.join(" ")}
            </div>
          </button>
        ))}
        {servers.length === 0 && (
          <p className="text-xs px-3 py-6 text-center" style={{ color: 'var(--text-muted)' }}>
            还没有托管的 MCP 服务器
          </p>
        )}
      </div>
    </>
  );
}

export function emptyMcpForm(): McpFormValue {
  return {
    name: "",
    serverType: "stdio",
    command: "",
    argsText: "",
    envPairs: [],
    url: "",
  };
}

export function formFromCandidate(c: McpImportCandidate): McpFormValue {
  return {
    name: c.name,
    serverType: "stdio",
    command: c.command,
    argsText: c.args.join("\n"),
    envPairs: [],
    url: "",
  };
}

export function emptyImportSelection(): string[] {
  return [];
}

// Exported so McpLibrary can build the entry input payload.
export function buildEntryInput(form: McpFormValue) {
  return {
    name: form.name,
    command: form.command,
    args: form.argsText.split("\n").map((s) => s.trim()).filter(Boolean),
    env: Object.fromEntries(
      form.envPairs.filter((p) => p.key.trim()).map((p) => [p.key.trim(), p.value]),
    ),
    type: form.serverType,
    url: form.url || null,
  };
}
