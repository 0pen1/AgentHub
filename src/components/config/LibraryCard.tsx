import { Edit3, Trash2, Download, FolderInput } from "lucide-react";

export interface TagChipsProps {
  tags: string[];
  activeTag?: string | null;
  onTagClick?: (tag: string) => void;
}

/** Clickable tag chips — click filters the card wall by that tag. */
export function TagChips({ tags, activeTag, onTagClick }: TagChipsProps) {
  if (tags.length === 0) return null;
  return (
    <div className="flex flex-wrap gap-1 mt-2">
      {tags.map((tag) => {
        const active = activeTag === tag;
        return (
          <button
            key={tag}
            onClick={(e) => {
              e.stopPropagation();
              onTagClick?.(tag);
            }}
            className="text-[10px] px-1.5 py-0.5 rounded-md transition-colors"
            style={{
              background: active ? 'var(--accent-blue)' : 'var(--bg-tertiary)',
              color: active ? 'white' : 'var(--text-muted)',
            }}
          >
            {tag}
          </button>
        );
      })}
    </div>
  );
}

export interface SourceBadgeProps {
  source: string;
}

/** 托管 = blue badge; anything else (scanned agent id) = gray badge. */
export function SourceBadge({ source }: SourceBadgeProps) {
  const managed = source === "托管";
  return (
    <span
      className="text-[10px] px-1.5 py-0.5 rounded-md shrink-0"
      style={{
        background: managed ? 'rgba(79,110,247,0.12)' : 'var(--bg-tertiary)',
        color: managed ? 'var(--accent-blue)' : 'var(--text-muted)',
      }}
    >
      {managed ? "托管" : source}
    </span>
  );
}

export interface LibraryCardProps {
  icon: string;
  title: string;
  description: string;
  tags?: string[];
  activeTag?: string | null;
  onTagClick?: (tag: string) => void;
  source?: string;
  /** Extra mono line under the title, e.g. the MCP command. */
  subtitle?: string;
  /** Hover actions shown top-right. 托管 cards: edit/delete/export. Scanned: import. */
  onOpen?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  onExport?: () => void;
  onImport?: () => void;
}

export default function LibraryCard({
  icon,
  title,
  description,
  tags,
  activeTag,
  onTagClick,
  source,
  subtitle,
  onOpen,
  onEdit,
  onDelete,
  onExport,
  onImport,
}: LibraryCardProps) {
  return (
    <div
      onClick={onOpen}
      className="group relative rounded-2xl border p-4 cursor-pointer transition-all hover:shadow-sm flex flex-col h-full"
      style={{
        background: 'var(--bg-secondary)',
        borderColor: 'var(--border-light)',
      }}
    >
      {/* Hover action buttons */}
      <div
        className="absolute top-2.5 right-2.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={(e) => e.stopPropagation()}
      >
        {onImport && (
          <CardAction title="导入托管" onClick={onImport}>
            <FolderInput size={13} />
          </CardAction>
        )}
        {onEdit && (
          <CardAction title="编辑" onClick={onEdit}>
            <Edit3 size={13} />
          </CardAction>
        )}
        {onExport && (
          <CardAction title="导出" onClick={onExport}>
            <Download size={13} />
          </CardAction>
        )}
        {onDelete && (
          <CardAction title="删除" danger onClick={onDelete}>
            <Trash2 size={13} />
          </CardAction>
        )}
      </div>

      <div className="flex items-start gap-3">
        <div
          className="w-9 h-9 rounded-xl flex items-center justify-center text-[17px] shrink-0"
          style={{ background: 'var(--bg-tertiary)' }}
        >
          {icon}
        </div>
        <div className="min-w-0 flex-1 pr-6">
          <div className="flex items-center gap-1.5 min-w-0">
            <h3
              className="text-[13px] font-medium truncate min-w-0"
              style={{ color: 'var(--text-primary)' }}
              title={title}
            >
              {title}
            </h3>
            {source && <SourceBadge source={source} />}
          </div>
          {subtitle && (
            <p
              className="text-[11px] font-mono truncate mt-0.5"
              style={{ color: 'var(--text-muted)' }}
              title={subtitle}
            >
              {subtitle}
            </p>
          )}
        </div>
      </div>

      {/* Description: always exactly two lines tall */}
      <p
        className="text-[12px] leading-[1.5] mt-2 line-clamp-2 h-[3em] overflow-hidden"
        style={{ color: 'var(--text-secondary)' }}
        title={description}
      >
        {description || <span style={{ color: 'var(--text-muted)' }}>暂无描述</span>}
      </p>

      {/* Tag slot: fixed height so no-tag cards align with tagged ones */}
      <div className="min-h-[28px] overflow-hidden mt-auto">
        <TagChips tags={tags || []} activeTag={activeTag} onTagClick={onTagClick} />
      </div>
    </div>
  );
}

function CardAction({
  title,
  danger,
  onClick,
  children,
}: {
  title: string;
  danger?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className="p-1.5 rounded-md transition-colors hover:bg-[var(--bg-tertiary)]"
      style={{ color: danger ? 'var(--accent-red, #ef4444)' : 'var(--text-muted)' }}
    >
      {children}
    </button>
  );
}
