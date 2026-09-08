import { Send } from "lucide-react";
import { useState } from "react";

interface Props {
  onSend: (text: string) => void;
  disabled?: boolean;
}

export default function QuickPrompt({ onSend, disabled }: Props) {
  const [text, setText] = useState("");

  const handleSend = () => {
    if (text.trim() && !disabled) {
      onSend(text.trim());
      setText("");
    }
  };

  return (
    <div className="flex items-center gap-2 px-3 h-10 border-t"
      style={{ borderColor: 'var(--border)', background: 'var(--bg-secondary)' }}>
      <span className="text-xs" style={{ color: 'var(--text-muted)' }}>▶</span>
      <input
        type="text"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && handleSend()}
        placeholder="快速发送 prompt..."
        disabled={disabled}
        className="flex-1 bg-transparent text-xs outline-none disabled:opacity-40"
        style={{ color: 'var(--text-primary)' }}
      />
      <button
        onClick={handleSend}
        disabled={!text.trim() || disabled}
        className="p-1 rounded transition-opacity disabled:opacity-20"
        style={{ color: 'var(--accent-blue)' }}
      >
        <Send size={14} />
      </button>
    </div>
  );
}
