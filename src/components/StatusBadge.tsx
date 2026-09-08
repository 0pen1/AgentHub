const STATUS_MAP: Record<string, { color: string; bg: string; label: string }> = {
  running: { color: "#16a34a", bg: "#f0fdf4", label: "运行中" },
  waiting: { color: "#ca8a04", bg: "#fefce8", label: "等待中" },
  idle: { color: "#6b7280", bg: "#f3f4f6", label: "空闲" },
  error: { color: "#dc2626", bg: "#fef2f2", label: "错误" },
  stopped: { color: "#6b7280", bg: "#f3f4f6", label: "已停止" },
};

export default function StatusBadge({ status }: { status: string }) {
  const info = STATUS_MAP[status] || STATUS_MAP.idle;
  return (
    <span className="inline-flex items-center gap-1.5 text-xs px-2 py-0.5 rounded-full"
      style={{ background: info.bg, color: info.color }}>
      <span className="w-1.5 h-1.5 rounded-full" style={{ background: info.color }} />
      {info.label}
    </span>
  );
}
