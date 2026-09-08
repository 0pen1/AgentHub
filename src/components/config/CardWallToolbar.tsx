import { Search, X } from "lucide-react";

interface Props {
  search: string;
  onSearch: (v: string) => void;
  placeholder?: string;
  /** All tags present on the current tab's cards (with counts). */
  allTags: { tag: string; count: number }[];
  activeTag: string | null;
  onTagClick: (tag: string | null) => void;
  actions?: React.ReactNode;
}

/** Card-wall toolbar: search box + tag filter chips + optional right actions. */
export default function CardWallToolbar({
  search,
  onSearch,
  placeholder,
  allTags,
  activeTag,
  onTagClick,
  actions,
}: Props) {
  return (
    <div className="flex items-center gap-2 flex-wrap">
      <div className="relative flex-1 min-w-[180px] max-w-xs">
        <Search
          size={13}
          className="absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none"
          style={{ color: 'var(--text-muted)' }}
        />
        <input
          value={search}
          onChange={(e) => onSearch(e.target.value)}
          placeholder={placeholder || "搜索…"}
          className="w-full pl-7 pr-6 py-1.5 rounded-lg text-[12px] border outline-none focus:border-[var(--accent-blue)] transition-colors"
          style={{
            background: 'var(--bg-primary)',
            borderColor: 'var(--border-light)',
            color: 'var(--text-primary)',
          }}
        />
        {search && (
          <button
            onClick={() => onSearch("")}
            className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded hover:bg-[var(--bg-tertiary)]"
            style={{ color: 'var(--text-muted)' }}
          >
            <X size={11} />
          </button>
        )}
      </div>

      {allTags.length > 0 && (
        <div className="flex items-center gap-1 flex-wrap">
          {allTags.map(({ tag, count }) => {
            const active = activeTag === tag;
            return (
              <button
                key={tag}
                onClick={() => onTagClick(active ? null : tag)}
                className="px-2 py-1 rounded-lg text-[11px] transition-colors"
                style={{
                  background: active ? 'var(--accent-blue)' : 'var(--bg-tertiary)',
                  color: active ? 'white' : 'var(--text-secondary)',
                }}
                title={`按标签过滤: ${tag}`}
              >
                {tag}
                <span className="ml-1 opacity-60">{count}</span>
              </button>
            );
          })}
          {activeTag && (
            <button
              onClick={() => onTagClick(null)}
              className="p-1 rounded-lg hover:bg-[var(--bg-tertiary)]"
              style={{ color: 'var(--text-muted)' }}
              title="清除标签过滤"
            >
              <X size={12} />
            </button>
          )}
        </div>
      )}

      {actions && <div className="flex items-center gap-2 ml-auto">{actions}</div>}
    </div>
  );
}

/** Collect unique tags with counts from a list of items carrying `tags`. */
export function collectTags<T extends { tags?: string[] }>(items: T[]): { tag: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const item of items) {
    for (const tag of item.tags || []) {
      counts.set(tag, (counts.get(tag) || 0) + 1);
    }
  }
  return [...counts.entries()]
    .map(([tag, count]) => ({ tag, count }))
    .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
}

/** Substring filter on name + description. */
export function matchesSearch<T extends { name?: string; description?: string }>(
  item: T,
  search: string,
): boolean {
  if (!search) return true;
  const q = search.toLowerCase();
  return (
    (item.name || "").toLowerCase().includes(q) ||
    (item.description || "").toLowerCase().includes(q)
  );
}
