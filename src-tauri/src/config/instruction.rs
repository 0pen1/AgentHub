use std::path::Path;

pub struct InstructionMerger;

impl InstructionMerger {
    pub fn new() -> Self {
        Self
    }

    /// Read and concatenate multiple instruction files.
    ///
    /// Entries are usually full paths (as selected in the Launcher), but expert
    /// presets store bare names (library file stems) — resolve those against
    /// the managed instructions dir so preset launches still inject prompts.
    pub fn merge(&self, paths: &[String]) -> String {
        let mut parts = Vec::new();

        for path_str in paths {
            let path = if path_str.starts_with('~') {
                if let Some(home) = dirs::home_dir() {
                    home.join(&path_str[2..])
                } else {
                    continue;
                }
            } else {
                Path::new(path_str).to_path_buf()
            };

            // Bare name fallback: exact path missing → try <library>/instructions/<name>.md
            let content = std::fs::read_to_string(&path).or_else(|_| {
                if path_str.contains('/') {
                    return Err(());
                }
                let fallback = crate::config::library::instructions_dir()
                    .join(format!("{}.md", path_str));
                std::fs::read_to_string(fallback).map_err(|_| ())
            });

            if let Ok(content) = content {
                if !content.trim().is_empty() {
                    parts.push(content);
                }
            }
        }

        parts.join("\n\n---\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expert presets store bare names; the merger must resolve them against
    /// the managed library dir or preset launches silently inject nothing.
    #[test]
    fn bare_name_resolves_to_library() {
        let tmp = std::env::temp_dir().join(format!("ah-inst-{}", uuid::Uuid::new_v4()));
        let lib = crate::config::library::instructions_dir();
        std::fs::create_dir_all(&lib).unwrap();
        let target = lib.join("ah-test-prompt.md");
        std::fs::write(&target, "LIBRARY CONTENT").unwrap();
        // Note: this writes into the REAL ~/.agenthub library — acceptable
        // for a dev machine; cleaned up below.
        let content = InstructionMerger::new().merge(&["ah-test-prompt".to_string()]);
        std::fs::remove_file(&target).ok();
        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(content, "LIBRARY CONTENT");
    }

    #[test]
    fn merge_joins_multiple_parts_with_separator() {
        let tmp = std::env::temp_dir().join(format!("ah-inst-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let a = tmp.join("a.md");
        let b = tmp.join("b.md");
        std::fs::write(&a, "AAA").unwrap();
        std::fs::write(&b, "BBB").unwrap();

        let content = InstructionMerger::new().merge(&[
            a.to_string_lossy().to_string(),
            b.to_string_lossy().to_string(),
        ]);
        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(content, "AAA\n\n---\n\nBBB");
    }

    #[test]
    fn missing_paths_are_skipped_not_errors() {
        let content = InstructionMerger::new().merge(&["/nonexistent/path/x.md".to_string()]);
        assert_eq!(content, "");
    }

    #[test]
    fn empty_files_contribute_nothing() {
        let tmp = std::env::temp_dir().join(format!("ah-inst-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let a = tmp.join("empty.md");
        let b = tmp.join("real.md");
        std::fs::write(&a, "   \n  ").unwrap();
        std::fs::write(&b, "REAL").unwrap();

        let content = InstructionMerger::new().merge(&[
            a.to_string_lossy().to_string(),
            b.to_string_lossy().to_string(),
        ]);
        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(content, "REAL");
    }
}
