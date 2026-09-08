use std::collections::VecDeque;
use std::path::PathBuf;

/// Per-session in-memory tail buffer of raw PTY output, persisted to disk on
/// app exit so a reopened session shows its last screen instead of a blank
/// terminal. Inspired by Orca's terminal scrollback snapshots.
///
/// Bounds: 512 KiB per session (tail only). Truncation drops the FRONT and is
/// done on UTF-8 chunk boundaries — chunks are the unit, so a multi-byte
/// character split across two chunks still reassembles correctly at restore
/// time; only the character straddling the cut point may be dropped.
const MAX_BYTES: usize = 512 * 1024;

pub struct ScrollbackBuffer {
    chunks: VecDeque<Vec<u8>>,
    total: usize,
}

impl Default for ScrollbackBuffer {
    fn default() -> Self {
        Self {
            chunks: VecDeque::new(),
            total: 0,
        }
    }
}

impl ScrollbackBuffer {
    pub fn push(&mut self, chunk: &[u8]) {
        self.chunks.push_back(chunk.to_vec());
        self.total += chunk.len();
        self.trim();
    }

    /// Drop oldest whole chunks until under the cap.
    fn trim(&mut self) {
        while self.total > MAX_BYTES {
            match self.chunks.front() {
                Some(front) => {
                    self.total -= front.len();
                    self.chunks.pop_front();
                }
                None => break,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Concatenated tail, capped at MAX_BYTES. The reader feeds whole chunks,
    /// so this reconstructs the exact byte stream (minus trimmed front chunks).
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.total.min(MAX_BYTES));
        for chunk in &self.chunks {
            out.extend_from_slice(chunk);
        }
        if out.len() > MAX_BYTES {
            out = out.split_off(out.len() - MAX_BYTES);
        }
        out
    }

    /// Replace contents with a snapshot read back from disk.
    pub fn restore(&mut self, data: Vec<u8>) {
        self.chunks.clear();
        self.total = 0;
        if data.is_empty() {
            return;
        }
        self.push(&data);
    }
}

/// Snapshot storage root: ~/.agenthub/scrollback/
fn snapshot_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".agenthub").join("scrollback"))
}

fn snapshot_path(session_id: &str) -> Option<PathBuf> {
    // session ids are UUIDs; refuse anything path-like
    if session_id.is_empty() || session_id.contains('/') || session_id.contains('\\') {
        return None;
    }
    snapshot_root().map(|r| r.join(format!("{}.bin", session_id)))
}

/// Write the buffer tail to disk atomically (tmp + rename), best-effort.
pub fn save_snapshot(session_id: &str, data: &[u8]) {
    let Some(path) = snapshot_path(session_id) else {
        return;
    };
    if std::fs::create_dir_all(path.parent().unwrap_or(&path)).is_err() {
        return;
    }
    let tmp = path.with_extension(format!("bin.{}.tmp", std::process::id()));
    if std::fs::write(&tmp, data).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Load a persisted snapshot, dropping any partial leading UTF-8 sequence so
/// the frontend writes valid text only.
pub fn load_snapshot(session_id: &str) -> Option<Vec<u8>> {
    let path = snapshot_path(session_id)?;
    let data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }
    let mut start = 0;
    while start < data.len() && (data[start] & 0xc0) == 0x80 {
        start += 1;
    }
    Some(data[start..].to_vec())
}

/// Convert a raw PTY tail (full of cursor moves, color codes, alternate-screen
/// switches) into plain text lines for restore. We deliberately do NOT replay
/// the raw bytes: TUI agents (claude code etc.) paint on the alternate screen
/// and clear it on resume, which would wipe a raw replay. Plain text written
/// before attach lands in the normal scrollback instead — the new TUI's clear
/// only affects its own screen, and the user can scroll up to see history.
///
/// Strategy: strip ANSI escape sequences (CSI/OSC/simple escapes), keep
/// printable runs, drop cursor-positioning artifacts, cap at ~200 lines.
pub fn snapshot_to_plain_text(data: &[u8]) -> Vec<u8> {
    // Strip ANSI: ESC [ ... final-byte, ESC ] ... BEL/ST, ESC followed by single char
    let mut out: Vec<u8> = Vec::with_capacity(data.len() / 2);
    let mut i = 0;
    let mut last_was_newline = false;

    while i < data.len() {
        let b = data[i];
        if b == 0x1b {
            i += skip_escape(data, i);
            continue;
        }
        if b == b'\n' {
            out.push(b'\n');
            last_was_newline = true;
            i += 1;
            continue;
        }
        if b == b'\r' || b == 0x08 || b == 0x07 {
            i += 1;
            continue;
        }
        if b < 0x20 {
            i += 1;
            continue;
        }
        // Printable or multibyte UTF-8 lead/continuation
        out.push(b);
        last_was_newline = false;
        i += 1;
    }
    let _ = last_was_newline;

    // Collapse runs of blank lines and cap the tail at 200 lines
    let text = String::from_utf8_lossy(&out).to_string();
    let mut lines: Vec<&str> = Vec::new();
    let mut blank_run = 0;
    for line in text.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank {
            blank_run += 1;
            if blank_run > 2 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        lines.push(line);
    }
    let capped: Vec<&str> = if lines.len() > 200 {
        lines[lines.len() - 200..].to_vec()
    } else {
        lines
    };
    capped.join("\n").into_bytes()
}

/// Bytes consumed by the escape sequence starting at data[i] (data[i] == ESC).
fn skip_escape(data: &[u8], i: usize) -> usize {
    if i + 1 >= data.len() {
        return data.len() - i;
    }
    match data[i + 1] {
        b'[' => {
            // CSI: params + intermediate bytes, terminated by 0x40..=0x7E
            let mut j = i + 2;
            while j < data.len() {
                let c = data[j];
                if (0x40..=0x7e).contains(&c) {
                    return j - i + 1;
                }
                // param/intermediate range
                if !(0x20..=0x3f).contains(&c) {
                    return j - i; // malformed; stop here
                }
                j += 1;
            }
            data.len() - i
        }
        b']' => {
            // OSC: terminated by BEL or ESC \
            let mut j = i + 2;
            while j < data.len() {
                if data[j] == 0x07 {
                    return j - i + 1;
                }
                if data[j] == 0x1b && j + 1 < data.len() && data[j + 1] == b'\\' {
                    return j - i + 2;
                }
                j += 1;
            }
            data.len() - i
        }
        b'P' => {
            // DCS: same terminator as OSC
            let mut j = i + 2;
            while j < data.len() {
                if data[j] == 0x07 {
                    return j - i + 1;
                }
                if data[j] == 0x1b && j + 1 < data.len() && data[j + 1] == b'\\' {
                    return j - i + 2;
                }
                j += 1;
            }
            data.len() - i
        }
        _ => 2, // simple two-byte escape (ESC 7, ESC 8, ESC =, etc.)
    }
}

/// Delete the snapshot for a session (used on delete_session).
pub fn delete_snapshot(session_id: &str) {
    if let Some(path) = snapshot_path(session_id) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_small() {
        let mut b = ScrollbackBuffer::default();
        b.push(b"hello ");
        b.push("世界".as_bytes());
        let snap = b.snapshot();
        assert_eq!(snap, b"hello \xe4\xb8\x96\xe7\x95\x8c");
    }

    #[test]
    fn trims_to_cap_keeping_tail() {
        let mut b = ScrollbackBuffer::default();
        let big = vec![b'a'; 600 * 1024];
        b.push(&big);
        b.push(b"TAIL");
        let snap = b.snapshot();
        assert!(snap.len() <= 512 * 1024);
        assert!(snap.ends_with(b"TAIL"));
    }

    #[test]
    fn utf8_boundary_trim_on_load() {
        // Leading continuation byte (middle of a multibyte char) must be dropped
        let data = vec![0xb8, 0xe4, 0xb8, 0x96]; // broken lead + 世
        let mut b = ScrollbackBuffer::default();
        b.restore(data.clone());
        let out = load_snapshot_checked(data);
        assert_eq!(out, vec![0xe4, 0xb8, 0x96]);
    }

    fn load_snapshot_checked(data: Vec<u8>) -> Vec<u8> {
        let mut start = 0;
        while start < data.len() && (data[start] & 0xc0) == 0x80 {
            start += 1;
        }
        data[start..].to_vec()
    }

    #[test]
    fn strips_claude_tui_sequences() {
        // Realistic Claude Code startup: alt screen, clear, cursor moves, styled text, OSC title
        let raw = b"\x1b7\x1b[r\x1b8\x1b[?1049h\x1b[2J\x1b[H\x1b]0;Claude Code\x07\
                    \x1b[38;2;215;119;87mClaude Code\x1b[18G\x1b[38;2;153;153;153mv2.1.236\r\n\
                    \xe4\xb8\x96\xe7\x95\x8c\xe4\xbd\xa0\xe5\xa5\xbd\x1b[6G";
        let out = snapshot_to_plain_text(raw);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Claude Code"));
        assert!(text.contains("v2.1.236"));
        assert!(text.contains("世界你好"));        assert!(!text.contains('\x1b'));
        assert!(!text.contains('\x07'));
    }

    #[test]
    fn caps_at_200_lines() {
        let mut raw: Vec<u8> = Vec::new();
        for i in 0..400 {
            raw.extend_from_slice(format!("line {}\r\n", i).as_bytes());
        }
        let out = snapshot_to_plain_text(&raw);
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 200);
        assert!(text.contains("line 399"));
        assert!(!text.contains("line 100\n"));
    }

    #[test]
    fn handles_truncated_escape_at_end() {
        // ESC [ with no terminator — must not loop forever or panic
        let raw = b"ok\x1b[38;2";
        let out = snapshot_to_plain_text(raw);
        assert_eq!(String::from_utf8(out).unwrap(), "ok");
    }

    #[test]
    fn keeps_osc_with_esc_backslash_terminator() {
        let raw = b"\x1b]8;;https://x.com\x1b\\link text\x1b]8;;\x1b\\done";
        let out = snapshot_to_plain_text(raw);
        assert_eq!(String::from_utf8(out).unwrap(), "link textdone");
    }
}
