use std::path::{Path, PathBuf};

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
