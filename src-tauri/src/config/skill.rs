use crate::commands::SkillInfo;
use std::path::{Path, PathBuf};

/// Shared skill search dirs, in dedup priority order (first hit wins).
/// The AgentHub-managed library dir comes FIRST so a managed ("托管") skill
/// shadows a scanned same-name skill — the user checked the managed entry,
/// so its copy must be the one injected. Deleting the managed copy lets the
/// scanned original resurface naturally.
pub fn skill_search_dirs(home: &Path) -> Vec<(PathBuf, &'static str)> {
    vec![
        (
            home.join(".agenthub").join("library").join("skills"),
            "托管",
        ),
        (home.join(".agents").join("skills"), "global"),
        (home.join(".claude").join("skills"), "claude"),
        (home.join(".codex").join("skills"), "codex"),
        (home.join(".pi").join("agent").join("skills"), "pi"),
        (
            home.join(".config").join("opencode").join("skills"),
            "opencode",
        ),
    ]
}

/// Parse a SKILL.md's frontmatter into (name, description, tags). Falls back to
/// the parent dir name when frontmatter/name is missing. Returns None only when
/// the file is unreadable or the frontmatter block is malformed YAML.
pub fn parse_skill_md(path: &Path) -> Option<(String, String, Vec<String>)> {
    let content = std::fs::read_to_string(path).ok()?;

    let dir_name = || {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    };

    if !content.starts_with("---") {
        return Some((dir_name(), String::new(), Vec::new()));
    }

    let end = content[3..].find("---")?;
    let frontmatter = &content[3..3 + end];

    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;

    let name = yaml
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(dir_name);

    let description = yaml
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tags: Vec<String> = yaml
        .get("tags")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some((name, description, tags))
}

pub struct SkillScanner;

impl SkillScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan_all(&self) -> Vec<SkillInfo> {
        let mut skills = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return skills,
        };

        for (dir, source) in skill_search_dirs(&home) {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let skill_md = path.join("SKILL.md");
                    if !skill_md.exists() {
                        continue;
                    }
                    if let Some((name, description, tags)) = parse_skill_md(&skill_md) {
                        if seen.insert(name.clone()) {
                            skills.push(SkillInfo {
                                name,
                                description,
                                tags,
                                path: skill_md.to_string_lossy().to_string(),
                                source: source.to_string(),
                            });
                        }
                    }
                }
            }
        }

        skills
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_dir_comes_first() {
        let home = dirs::home_dir().unwrap();
        let dirs_list = skill_search_dirs(&home);
        assert_eq!(dirs_list[0].1, "托管");
        assert!(dirs_list[0]
            .0
            .to_string_lossy()
            .contains(".agenthub/library/skills"));
    }
}
