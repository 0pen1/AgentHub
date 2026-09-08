import { useEffect, useRef } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { invoke } from "@tauri-apps/api/core";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  sessionId: string;
  /** Called when terminal is ready; provides the write function for PTY output */
  onMount: (writeFn: (data: Uint8Array) => void) => void;
  /** Called when terminal unmounts */
  onUnmount: () => void;
  /** User keyboard input from xterm → forward to PTY */
  onData: (data: string) => void;
  /** Terminal dimensions changed → resize PTY (after initial attach) */
  onResize: (cols: number, rows: number) => void;
}


export default function Terminal({ sessionId, onMount, onUnmount, onData, onResize }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<XTerm | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  // Whether pty_attach has completed for this session's terminal instance
  const attachedRef = useRef(false);
  // Pending resize while attach is in flight
  const pendingResizeRef = useRef<{ cols: number; rows: number } | null>(null);

  // Keep latest callbacks without re-creating the terminal
  const onDataRef = useRef(onData);
  onDataRef.current = onData;
  const onResizeRef = useRef(onResize);
  onResizeRef.current = onResize;

  useEffect(() => {
    if (!containerRef.current) return;
    attachedRef.current = false;
    pendingResizeRef.current = null;

    const term = new XTerm({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: "'SF Mono', Menlo, Monaco, 'Courier New', monospace",
      theme: {
        background: "#1e1e1e",
        foreground: "#d4d4d4",
        cursor: "#7b8cff",
        selectionBackground: "#4f6ef740",
      },
      scrollback: 10000,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);

    // GPU rendering (best effort)
    try {
      term.loadAddon(new WebglAddon());
    } catch {
      // canvas fallback is automatic
    }

    fitAddon.fit();

    // Keyboard input → PTY
    term.onData((data) => onDataRef.current(data));

    const syncResize = (cols: number, rows: number) => {
      if (!attachedRef.current) {
        pendingResizeRef.current = { cols, rows };
        return;
      }
      onResizeRef.current(cols, rows);
    };

    // Initial size → PTY
    syncResize(term.cols, term.rows);

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      syncResize(term.cols, term.rows);
    });
    resizeObserver.observe(containerRef.current);

    // PTY output → terminal
    const writeFn = (data: Uint8Array) => {
      term.write(data);
    };
    onMount(writeFn);

    termRef.current = term;
    fitRef.current = fitAddon;
    term.focus();

    // Attach: fire pty_attach and scrollback snapshot in parallel.
    // pty_attach registers the PTY listener; snapshot restores prior output as
    // plain text so the TUI agent's fresh screen paints below it.
    const attachPromise = invoke("pty_attach", { sessionId, cols: term.cols, rows: term.rows });
    const snapshotPromise = invoke<string | null>("get_scrollback_snapshot", { sessionId }).catch(() => null);

    attachPromise
      .then(() => {
        attachedRef.current = true;
        const pending = pendingResizeRef.current;
        if (pending) {
          pendingResizeRef.current = null;
          onResizeRef.current(pending.cols, pending.rows);
        }
      })
      .catch((err) => {
        console.error("pty_attach failed:", err);
        term.writeln(`\x1b[31m⚠ 会话启动失败: ${err}\x1b[0m`);
      });

    snapshotPromise.then((b64) => {
      if (b64) {
        const raw = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
        term.write(raw);
        term.writeln("");
      }
    });

    return () => {
      resizeObserver.disconnect();
      onUnmount();
      term.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  return <div ref={containerRef} className="w-full h-full" style={{ background: "#1e1e1e" }} />;
}
