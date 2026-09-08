//! AgentHub-managed config library at ~/.agenthub/library/.
//!
//! Skills, MCP servers, and instruction/prompt files the user manages in-app.
//! Nothing here ever writes into agent-native dirs (~/.claude, ~/.codex, ...);
//! session launch injects selected entries dynamically via the existing seams.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::commands::{
    InstructionDetail, InstructionInfo, McpEntryInput, McpImportCandidate, McpImportResult,
    McpServerInfo, SkillInfo,
};

pub const LIBRARY_SOURCE: &str = "托管";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

pub fn library_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".agenthub")
        .join("library")
}

pub fn skills_dir() -> PathBuf {
    library_dir().join("skills")
}

pub fn instructions_dir() -> PathBuf {
    library_dir().join("instructions")
}

fn mcps_file() -> PathBuf {
    library_dir().join("mcps.json")
}

pub fn ensure_dirs() -> Result<(), String> {
    std::fs::create_dir_all(skills_dir()).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(instructions_dir()).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Name validation
// ---------------------------------------------------------------------------

/// Validate + normalize a library item name. The result doubles as the
/// on-disk dir/file name and the SKILL.md frontmatter `name`, so they can
/// never drift apart. Rejects path separators and other filesystem-hostile
/// characters; allows CJK and common punctuation; spaces collapse to '-'.
pub fn slugify_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if trimmed.chars().count() > 64 {
        return Err("名称过长（最多 64 字符）".to_string());
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut last_was_dash = true; // suppress leading dashes
    for c in trimmed.chars() {
        if matches!(
            c,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
        ) || c.is_control()
        {
            return Err(format!("名称不能包含字符 '{}'", c));
        }
        if c == ' ' {
            if !last_was_dash {
                out.push('-');
                last_was_dash = true;
            }
        } else {
            out.push(c);
            last_was_dash = false;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if out.starts_with('.') {
        return Err("名称不能以 '.' 开头".to_string());
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Skills
// ---------------------------------------------------------------------------

pub fn list_skills() -> Result<Vec<SkillInfo>, String> {
    let dir = skills_dir();
    let mut skills = Vec::new();
    if !dir.exists() {
        return Ok(skills);
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();
    names.sort();
    for name in names {
        let skill_md = dir.join(&name).join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let (_, description, tags) = parse_library_skill_md(&skill_md, &name);
        skills.push(SkillInfo {
            name: name.clone(),
            description,
            tags,
            path: skill_md.to_string_lossy().to_string(),
            source: LIBRARY_SOURCE.to_string(),
        });
    }
    Ok(skills)
}

/// Parse managed SKILL.md frontmatter; falls back to the dir name (managed
/// skills keep dir name == frontmatter name, but tolerate drift on read).
fn parse_library_skill_md(path: &Path, dir_name: &str) -> (String, String, Vec<String>) {
    match crate::config::skill::parse_skill_md(path) {
        Some((name, desc, tags)) => (name, desc, tags),
        None => (dir_name.to_string(), String::new(), Vec::new()),
    }
}

/// Extract `tags` from a raw file's frontmatter (empty when absent/malformed).
fn frontmatter_tags(raw: &str) -> Vec<String> {
    let rest = match raw.strip_prefix("---") {
        Some(r) => r,
        None => return Vec::new(),
    };
    let end = match rest.find("---") {
        Some(e) => e,
        None => return Vec::new(),
    };
    serde_yaml::from_str::<serde_yaml::Value>(&rest[..end])
        .ok()
        .and_then(|y| {
            y.get("tags").and_then(|v| v.as_sequence()).map(|seq| {
                seq.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
        .unwrap_or_default()
}

pub struct SkillDetail {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub path: String,
    pub content: String,
    pub files: Vec<String>,
}

pub fn read_skill(name: &str) -> Result<SkillDetail, String> {
    let dir = skills_dir().join(name);
    let skill_md = dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err(format!("skill 不存在: {}", name));
    }
    let raw = std::fs::read_to_string(&skill_md).map_err(|e| e.to_string())?;
    let (description, content) = split_frontmatter(&raw);
    let fm_name = crate::config::skill::parse_skill_md(&skill_md)
        .map(|(n, _, _)| n)
        .unwrap_or_else(|| name.to_string());
    Ok(SkillDetail {
        name: fm_name,
        description,
        tags: frontmatter_tags(&raw),
        path: skill_md.to_string_lossy().to_string(),
        content,
        files: list_files_relative(&dir)?,
    })
}

/// Split a `--- frontmatter ---` header off; returns (description, body).
/// Description comes from the frontmatter's description key when present.
fn split_frontmatter(raw: &str) -> (String, String) {
    if let Some(rest) = raw.strip_prefix("---") {
        if let Some(end) = rest.find("---") {
            let fm = &rest[..end];
            let body = rest[end + 3..].trim_start_matches('\n').to_string();
            let description = serde_yaml::from_str::<serde_yaml::Value>(fm)
                .ok()
                .and_then(|y| y.get("description").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_default();
            return (description, body);
        }
    }
    (String::new(), raw.to_string())
}

fn list_files_relative(dir: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, out)?;
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
        Ok(())
    }
    walk(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

pub fn create_skill(
    name: &str,
    description: &str,
    tags: &[String],
    content: &str,
) -> Result<SkillInfo, String> {
    ensure_dirs()?;
    let slug = slugify_name(name)?;
    let dir = skills_dir().join(&slug);
    if dir.exists() {
        return Err(format!("同名 skill 已存在: {}", slug));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let skill_md = format!(
        "---\nname: {}\ndescription: {}\ntags:\n{}---\n\n{}",
        slug,
        description,
        yaml_tags_block(tags),
        content
    );
    std::fs::write(dir.join("SKILL.md"), skill_md).map_err(|e| e.to_string())?;
    Ok(SkillInfo {
        name: slug,
        description: description.to_string(),
        tags: tags.to_vec(),
        path: dir.join("SKILL.md").to_string_lossy().to_string(),
        source: LIBRARY_SOURCE.to_string(),
    })
}

/// Format a tags list as YAML sequence lines ("" when empty), e.g.
/// "  - tdd\n  - testing\n" — ready to splice into a frontmatter template.
fn yaml_tags_block(tags: &[String]) -> String {
    tags.iter()
        .map(|t| format!("  - {}\n", t))
        .collect()
}

pub fn update_skill(
    name: &str,
    new_name: &str,
    description: &str,
    tags: &[String],
    content: &str,
) -> Result<SkillInfo, String> {
    ensure_dirs()?;
    let old_dir = skills_dir().join(name);
    if !old_dir.join("SKILL.md").exists() {
        return Err(format!("skill 不存在: {}", name));
    }
    let slug = slugify_name(new_name)?;
    let new_dir = skills_dir().join(&slug);
    if slug != name && new_dir.exists() {
        return Err(format!("同名 skill 已存在: {}", slug));
    }

    // Preserve any extra frontmatter keys (allowed-tools etc.); only rewrite
    // name/description. Rewrite the whole file from (frontmatter, body) since
    // the editor owns the body.
    let mut fm = serde_yaml::Mapping::new();
    let existing = std::fs::read_to_string(old_dir.join("SKILL.md")).unwrap_or_default();
    if let Some(rest) = existing.strip_prefix("---") {
        if let Some(end) = rest.find("---") {
            if let Ok(serde_yaml::Value::Mapping(m)) =
                serde_yaml::from_str::<serde_yaml::Value>(&rest[..end])
            {
                fm = m;
            }
        }
    }
    fm.insert(
        serde_yaml::Value::String("name".into()),
        serde_yaml::Value::String(slug.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("description".into()),
        serde_yaml::Value::String(description.to_string()),
    );
    // tags: replace with the form's list (drop the key entirely when empty)
    if tags.is_empty() {
        fm.remove(serde_yaml::Value::String("tags".into()));
    } else {
        fm.insert(
            serde_yaml::Value::String("tags".into()),
            serde_yaml::Value::Sequence(
                tags.iter()
                    .map(|t| serde_yaml::Value::String(t.clone()))
                    .collect(),
            ),
        );
    }
    let fm_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(fm))
        .map_err(|e| e.to_string())?;
    let file = format!("---\n{}---\n\n{}", fm_str, content);

    if slug != name {
        std::fs::rename(&old_dir, &new_dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(new_dir.join("SKILL.md"), file).map_err(|e| e.to_string())?;
    Ok(SkillInfo {
        name: slug,
        description: description.to_string(),
        tags: tags.to_vec(),
        path: new_dir.join("SKILL.md").to_string_lossy().to_string(),
        source: LIBRARY_SOURCE.to_string(),
    })
}

pub fn delete_skill(name: &str) -> Result<(), String> {
    let dir = skills_dir().join(name);
    if !dir.exists() {
        return Err(format!("skill 不存在: {}", name));
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

pub fn import_skill(source_dir: &str, name_override: Option<&str>) -> Result<SkillInfo, String> {
    ensure_dirs()?;
    let src = Path::new(source_dir);
    if !src.is_dir() {
        return Err(format!("源目录不存在: {}", source_dir));
    }
    if !src.join("SKILL.md").exists() {
        return Err("所选目录缺少 SKILL.md,不是有效的 skill 目录".to_string());
    }
    let name = match name_override {
        Some(n) if !n.trim().is_empty() => slugify_name(n)?,
        _ => {
            let (fm_name, _, _) =
                parse_library_skill_md(&src.join("SKILL.md"), "placeholder");
            if fm_name == "placeholder" {
                slugify_name(
                    &src.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                )?
            } else {
                slugify_name(&fm_name)?
            }
        }
    };
    let dst = skills_dir().join(&name);
    if dst.exists() {
        return Err(format!("同名 skill 已存在: {}", name));
    }
    copy_dir_recursive(src, &dst)?;
    // Force frontmatter name to match the dir name so name→path resolution
    // at launch time hits this copy.
    let skill_md_path = dst.join("SKILL.md");
    let raw = std::fs::read_to_string(&skill_md_path).unwrap_or_default();
    let (_, body) = split_frontmatter(&raw);
    let (description, tags) = crate::config::skill::parse_skill_md(&skill_md_path)
        .map(|(_, d, t)| (d, t))
        .unwrap_or_default();
    let fm = format!(
        "---\nname: {}\ndescription: {}\ntags:\n{}---\n\n{}",
        name,
        description,
        yaml_tags_block(&tags),
        body
    );
    std::fs::write(&skill_md_path, fm).map_err(|e| e.to_string())?;
    Ok(SkillInfo {
        name,
        description,
        tags,
        path: skill_md_path.to_string_lossy().to_string(),
        source: LIBRARY_SOURCE.to_string(),
    })
}

pub fn export_skill(name: &str, dest_dir: &str) -> Result<String, String> {
    let src = skills_dir().join(name);
    if !src.is_dir() {
        return Err(format!("skill 不存在: {}", name));
    }
    let dst = Path::new(dest_dir).join(name);
    if dst.exists() {
        return Err(format!("目标已存在: {}", dst.display()));
    }
    copy_dir_recursive(&src, &dst)?;
    Ok(dst.to_string_lossy().to_string())
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MCP servers
// ---------------------------------------------------------------------------

pub fn load_library_mcps() -> HashMap<String, serde_json::Value> {
    let path = mcps_file();
    if !path.exists() {
        return HashMap::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

pub fn save_library_mcps(map: &HashMap<String, serde_json::Value>) -> Result<(), String> {
    ensure_dirs()?;
    let path = mcps_file();
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let content = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Convert a stored/selected MCP config value into the display struct.
/// Shared by McpScanner (scanned entries) and the library listing.
pub fn mcp_info_from_value(name: String, value: &serde_json::Value, source: &str) -> McpServerInfo {
    let command = match value.get("command") {
        Some(serde_json::Value::String(s)) => s.clone(),
        // opencode style: command is an array [bin, arg0, arg1, ...]
        Some(serde_json::Value::Array(arr)) => arr
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        _ => {
            if value.get("url").is_some() {
                "(http)".to_string()
            } else {
                "unknown".to_string()
            }
        }
    };
    let args: Vec<String> = match value.get("command") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .skip(1)
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => value
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    };
    let env: HashMap<String, String> = value
        .get("env")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let server_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .map(String::from);
    let url = value
        .get("url")
        .and_then(|v| v.as_str())
        .map(String::from);
    McpServerInfo {
        name,
        command,
        args,
        env,
        server_type,
        url,
        source: source.to_string(),
    }
}

pub fn list_mcps() -> Result<Vec<McpServerInfo>, String> {
    let map = load_library_mcps();
    let mut infos: Vec<McpServerInfo> = map
        .into_iter()
        .map(|(name, value)| mcp_info_from_value(name, &value, LIBRARY_SOURCE))
        .collect();
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(infos)
}

/// Build the stored JSON value from a form input; strips the inner "name"
/// key (the map key IS the name — a stray inner key would leak into
/// generated agent configs, e.g. codex's [mcp_servers.x] table).
pub fn mcp_value_from_input(input: &McpEntryInput) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    let is_stdio = input.server_type.as_deref().unwrap_or("stdio") == "stdio";
    if !is_stdio {
        if let Some(t) = &input.server_type {
            obj.insert("type".into(), serde_json::Value::String(t.clone()));
        }
        if let Some(url) = &input.url {
            obj.insert("url".into(), serde_json::Value::String(url.clone()));
        }
    }
    if is_stdio {
        if !input.command.is_empty() {
            obj.insert("command".into(), serde_json::Value::String(input.command.clone()));
        }
        if !input.args.is_empty() {
            obj.insert(
                "args".into(),
                serde_json::Value::Array(
                    input
                        .args
                        .iter()
                        .map(|a| serde_json::Value::String(a.clone()))
                        .collect(),
                ),
            );
        }
    }
    if !input.env.is_empty() {
        obj.insert(
            "env".into(),
            serde_json::Value::Object(
                input
                    .env
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect(),
            ),
        );
    }
    for (k, v) in &input.extra {
        obj.insert(k.clone(), v.clone());
    }
    serde_json::Value::Object(obj)
}

pub fn create_mcp(input: &McpEntryInput) -> Result<(), String> {
    let name = slugify_name(&input.name)?;
    let mut map = load_library_mcps();
    if map.contains_key(&name) {
        return Err(format!("同名 MCP 已存在: {}", name));
    }
    map.insert(name, mcp_value_from_input(input));
    save_library_mcps(&map)
}

pub fn update_mcp(old_name: &str, input: &McpEntryInput) -> Result<(), String> {
    let name = slugify_name(&input.name)?;
    let mut map = load_library_mcps();
    if !map.contains_key(old_name) {
        return Err(format!("MCP 不存在: {}", old_name));
    }
    if name != old_name && map.contains_key(&name) {
        return Err(format!("同名 MCP 已存在: {}", name));
    }
    map.remove(old_name);
    map.insert(name, mcp_value_from_input(input));
    save_library_mcps(&map)
}

pub fn delete_mcp(name: &str) -> Result<(), String> {
    let mut map = load_library_mcps();
    if map.remove(name).is_none() {
        return Err(format!("MCP 不存在: {}", name));
    }
    save_library_mcps(&map)
}

/// Copy a scanned (agent-native) MCP entry into the managed library by name,
/// preserving the raw config value (type/url/extra included — not just the
/// display projection). Used by the "本机发现 → 导入托管" card action.
pub fn adopt_scanned_mcp(name: &str) -> Result<(), String> {
    for agent_id in ["claude", "codex", "gemini", "pi", "opencode"] {
        let Some(adapter) = crate::agent::adapters::get_adapter(agent_id) else {
            continue;
        };
        for (n, value) in adapter.read_mcp_servers() {
            if n == name {
                let mut map = load_library_mcps();
                if map.contains_key(name) {
                    return Err(format!("同名 MCP 已存在于托管库: {}", name));
                }
                map.insert(name.to_string(), value);
                return save_library_mcps(&map);
            }
        }
    }
    Err(format!("未找到扫描到的 MCP: {}", name))
}

// --- MCP import/export ------------------------------------------------------

/// Parse an MCP config file (any agent's format) into importable candidates.
/// Supports: JSON with `mcpServers` (claude/gemini), `mcp` (opencode), or a
/// bare map; TOML with `[mcp_servers.*]` (codex).
pub fn parse_mcp_import_file(path: &str) -> Result<Vec<McpImportCandidate>, String> {
    let p = Path::new(path);
    let content = std::fs::read_to_string(p).map_err(|e| format!("无法读取文件: {}", e))?;
    let map: HashMap<String, serde_json::Value> = if path.ends_with(".toml") {
        let toml_val: toml::Value = content.parse().map_err(|e| format!("TOML 解析失败: {}", e))?;
        toml_val
            .get("mcp_servers")
            .and_then(|v| v.as_table())
            .map(|t| {
                t.iter()
                    .map(|(k, v)| (k.clone(), crate::agent::adapters::codex::toml_to_json(v)))
                    .collect()
            })
            .ok_or_else(|| "文件中没有 [mcp_servers] 配置".to_string())?
    } else {
        let json: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;
        let servers = json
            .get("mcpServers")
            .or_else(|| json.get("mcp"))
            .cloned()
            .unwrap_or(json);
        serde_json::from_value(servers).map_err(|e| format!("文件格式不像 MCP 配置: {}", e))?
    };
    let existing = load_library_mcps();
    let mut candidates = Vec::new();
    for (name, value) in map {
        let info = mcp_info_from_value(name.clone(), &value, "");
        candidates.push(McpImportCandidate {
            env_keys: info.env.keys().cloned().collect(),
            exists_in_library: existing.contains_key(&name),
            name,
            command: info.command,
            args: info.args,
        });
    }
    candidates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(candidates)
}

pub fn import_mcps(path: &str, names: &[String]) -> Result<McpImportResult, String> {
    // Reuse the parse path by round-tripping through a temp map of raw values.
    let raw = parse_mcp_import_file_raw(path)?;
    let mut map = load_library_mcps();
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    for name in names {
        if let Some(value) = raw.get(name) {
            if map.contains_key(name) {
                skipped.push(name.clone());
            } else {
                map.insert(name.clone(), value.clone());
                imported.push(name.clone());
            }
        } else {
            skipped.push(name.clone());
        }
    }
    save_library_mcps(&map)?;
    Ok(McpImportResult { imported, skipped })
}

fn parse_mcp_import_file_raw(path: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let p = Path::new(path);
    let content = std::fs::read_to_string(p).map_err(|e| format!("无法读取文件: {}", e))?;
    if path.ends_with(".toml") {
        let toml_val: toml::Value = content.parse().map_err(|e| format!("TOML 解析失败: {}", e))?;
        return toml_val
            .get("mcp_servers")
            .and_then(|v| v.as_table())
            .map(|t| {
                t.iter()
                    .map(|(k, v)| (k.clone(), crate::agent::adapters::codex::toml_to_json(v)))
                    .collect()
            })
            .ok_or_else(|| "文件中没有 [mcp_servers] 配置".to_string());
    }
    let json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;
    let servers = json
        .get("mcpServers")
        .or_else(|| json.get("mcp"))
        .cloned()
        .unwrap_or(json);
    serde_json::from_value(servers).map_err(|e| format!("文件格式不像 MCP 配置: {}", e))
}

pub fn export_mcps(names: &[String], dest_path: &str, format: &str) -> Result<String, String> {
    let all = load_library_mcps();
    let mut selected = serde_json::Map::new();
    for name in names {
        match all.get(name) {
            Some(v) => {
                selected.insert(name.clone(), v.clone());
            }
            None => return Err(format!("MCP 不存在: {}", name)),
        }
    }
    let dest = Path::new(dest_path);
    match format {
        "codex-toml" => {
            let toml_value = json_map_to_toml(&selected)?;
            let out = toml::to_string_pretty(&toml_value)
                .map_err(|e| format!("TOML 序列化失败: {}", e))?;
            std::fs::write(dest, out).map_err(|e| e.to_string())?;
        }
        _ => {
            // claude-json (default)
            let out = serde_json::to_string_pretty(&serde_json::json!({ "mcpServers": selected }))
                .map_err(|e| e.to_string())?;
            std::fs::write(dest, out).map_err(|e| e.to_string())?;
        }
    }
    Ok(dest.to_string_lossy().to_string())
}

fn json_map_to_toml(map: &serde_json::Map<String, serde_json::Value>) -> Result<toml::Value, String> {
    let mut servers = toml::map::Map::new();
    for (name, value) in map {
        servers.insert(name.clone(), json_to_toml(value)?);
    }
    let mut root = toml::map::Map::new();
    root.insert("mcp_servers".to_string(), toml::Value::Table(servers));
    Ok(toml::Value::Table(root))
}

fn json_to_toml(value: &serde_json::Value) -> Result<toml::Value, String> {
    Ok(match value {
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        serde_json::Value::Array(arr) => {
            // TOML arrays are homogeneous; mixed arrays degrade to strings.
            let items: Vec<toml::Value> = arr.iter().filter_map(|v| json_to_toml(v).ok()).collect();
            toml::Value::Array(items)
        }
        serde_json::Value::Object(obj) => {
            let mut t = toml::map::Map::new();
            for (k, v) in obj {
                t.insert(k.clone(), json_to_toml(v)?);
            }
            toml::Value::Table(t)
        }
        serde_json::Value::Null => return Err("TOML 不支持 null 值".to_string()),
    })
}

// ---------------------------------------------------------------------------
// Instructions / prompts
// ---------------------------------------------------------------------------

// Instructions reuse the commands.rs structs (InstructionInfo/InstructionDetail).

pub fn list_instructions() -> Result<Vec<InstructionInfo>, String> {
    let dir = instructions_dir();
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            (name.ends_with(".md")).then_some(name)
        })
        .collect();
    names.sort();
    for file_name in names {
        let path = dir.join(&file_name);
        let stem = file_name.trim_end_matches(".md").to_string();
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let (description, _) = split_instruction_description(&raw);
        out.push(InstructionInfo {
            name: stem,
            path: path.to_string_lossy().to_string(),
            description,
            tags: frontmatter_tags(&raw),
            source: LIBRARY_SOURCE.to_string(),
        });
    }
    Ok(out)
}

/// Description = frontmatter `description` if present, else the first
/// non-empty, non-heading line, else empty.
fn split_instruction_description(raw: &str) -> (String, String) {
    if let Some(rest) = raw.strip_prefix("---") {
        if let Some(end) = rest.find("---") {
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&rest[..end]) {
                if let Some(d) = yaml.get("description").and_then(|v| v.as_str()) {
                    let body = rest[end + 3..].trim_start_matches('\n').to_string();
                    return (d.to_string(), body);
                }
            }
        }
    }
    let first = raw
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("");
    (first.chars().take(80).collect(), String::new())
}

pub fn read_instruction(name: &str) -> Result<InstructionDetail, String> {
    let path = instructions_dir().join(format!("{}.md", name));
    if !path.exists() {
        return Err(format!("提示词不存在: {}", name));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    // The editor edits the raw file (frontmatter included if present).
    Ok(InstructionDetail {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        content: raw,
    })
}

pub fn create_instruction(name: &str, content: &str) -> Result<String, String> {
    ensure_dirs()?;
    let slug = slugify_name(name)?;
    let path = instructions_dir().join(format!("{}.md", slug));
    if path.exists() {
        return Err(format!("同名提示词已存在: {}", slug));
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(slug)
}

pub fn update_instruction(name: &str, new_name: &str, content: &str) -> Result<String, String> {
    ensure_dirs()?;
    let old_path = instructions_dir().join(format!("{}.md", name));
    if !old_path.exists() {
        return Err(format!("提示词不存在: {}", name));
    }
    let slug = slugify_name(new_name)?;
    let new_path = instructions_dir().join(format!("{}.md", slug));
    if slug != name && new_path.exists() {
        return Err(format!("同名提示词已存在: {}", slug));
    }
    std::fs::write(&old_path, content).map_err(|e| e.to_string())?;
    if slug != name {
        std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    }
    Ok(slug)
}

pub fn delete_instruction(name: &str) -> Result<(), String> {
    let path = instructions_dir().join(format!("{}.md", name));
    if !path.exists() {
        return Err(format!("提示词不存在: {}", name));
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

pub fn import_instructions(paths: &[String]) -> Result<Vec<String>, String> {
    ensure_dirs()?;
    let mut imported = Vec::new();
    for p in paths {
        let src = Path::new(p);
        if !src.is_file() {
            continue;
        }
        let stem = src
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let slug = slugify_name(&stem)?;
        let dst = instructions_dir().join(format!("{}.md", slug));
        if dst.exists() {
            continue; // skip existing, report via returned list
        }
        std::fs::copy(src, &dst).map_err(|e| e.to_string())?;
        imported.push(slug);
    }
    Ok(imported)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_rejects_traversal_and_hostile_chars() {
        assert!(slugify_name("../etc/passwd").is_err());
        assert!(slugify_name("a/b").is_err());
        assert!(slugify_name("a\\b").is_err());
        assert!(slugify_name("").is_err());
        assert!(slugify_name("   ").is_err());
        assert!(slugify_name(".hidden").is_err());
        assert!(slugify_name("a:b").is_err());
    }

    #[test]
    fn slugify_normalizes() {
        assert_eq!(slugify_name("Hello World").unwrap(), "Hello-World");
        assert_eq!(slugify_name("  spaced  out  ").unwrap(), "spaced-out");
        assert_eq!(slugify_name("中文 提示词").unwrap(), "中文-提示词");
        assert_eq!(slugify_name("trailing- ").unwrap(), "trailing");
        assert_eq!(slugify_name("ok_name.v2").unwrap(), "ok_name.v2");
    }

    #[test]
    fn skill_md_roundtrip_preserves_extra_frontmatter() {
        // update_skill preserves extra frontmatter keys — verify the mapping
        // rewrite path directly.
        let raw = "---\nname: old\ndescription: old desc\nallowed-tools: \"Bash(*)\"\n---\n\nbody here";
        let mut fm = serde_yaml::Mapping::new();
        if let Some(rest) = raw.strip_prefix("---") {
            if let Some(end) = rest.find("---") {
                if let Ok(serde_yaml::Value::Mapping(m)) =
                    serde_yaml::from_str::<serde_yaml::Value>(&rest[..end])
                {
                    fm = m;
                }
            }
        }
        fm.insert(
            serde_yaml::Value::String("name".into()),
            serde_yaml::Value::String("new".into()),
        );
        fm.insert(
            serde_yaml::Value::String("description".into()),
            serde_yaml::Value::String("new desc".into()),
        );
        let out = serde_yaml::to_string(&serde_yaml::Value::Mapping(fm)).unwrap();
        assert!(out.contains("allowed-tools"));
        assert!(out.contains("name: new"));
        assert!(out.contains("description: new desc"));
    }

    #[test]
    fn mcp_value_roundtrip_preserves_unknown_fields() {
        let input = McpEntryInput {
            name: "srv".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "pkg".into()],
            env: [("K".to_string(), "v".to_string())].into_iter().collect(),
            server_type: None,
            url: None,
            extra: [("headers".to_string(), serde_json::json!({"Auth": "x"}))]
                .into_iter()
                .collect(),
        };
        let v = mcp_value_from_input(&input);
        assert_eq!(v["command"], "npx");
        assert_eq!(v["env"]["K"], "v");
        assert_eq!(v["headers"]["Auth"], "x");
        assert!(v.get("name").is_none(), "inner name key must be stripped");
    }

    #[test]
    fn mcp_value_http_shape() {
        let input = McpEntryInput {
            name: "remote".into(),
            command: String::new(),
            args: vec![],
            env: Default::default(),
            server_type: Some("http".into()),
            url: Some("https://example.com/mcp".into()),
            extra: Default::default(),
        };
        let v = mcp_value_from_input(&input);
        assert_eq!(v["type"], "http");
        assert_eq!(v["url"], "https://example.com/mcp");
        assert!(v.get("command").is_none());
    }

    #[test]
    fn mcp_info_handles_opencode_command_array() {
        let v = serde_json::json!({
            "type": "local",
            "command": ["bun", "x", "server.js"],
            "enabled": true
        });
        let info = mcp_info_from_value("test".into(), &v, "test");
        assert_eq!(info.command, "bun");
        assert_eq!(info.args, vec!["x", "server.js"]);
    }

    #[test]
    fn parse_mcp_import_supports_three_json_shapes() {
        let dir = std::env::temp_dir().join(format!("agenthub-lib-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // claude shape
        let claude_path = dir.join("claude.json");
        std::fs::write(
            &claude_path,
            r#"{"mcpServers": {"a": {"command": "npx", "args": ["x"], "env": {"E": "1"}}}}"#,
        )
        .unwrap();
        let cands = parse_mcp_import_file(claude_path.to_str().unwrap()).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].name, "a");
        assert_eq!(cands[0].env_keys, vec!["E"]);

        // bare map shape
        let bare_path = dir.join("bare.json");
        std::fs::write(&bare_path, r#"{"b": {"command": "uvx", "args": []}}"#).unwrap();
        let cands = parse_mcp_import_file(bare_path.to_str().unwrap()).unwrap();
        assert_eq!(cands[0].name, "b");

        // codex toml shape
        let toml_path = dir.join("codex.toml");
        std::fs::write(
            &toml_path,
            "[mcp_servers.c]\ncommand = \"npx\"\nargs = [\"-y\", \"z\"]\n",
        )
        .unwrap();
        let cands = parse_mcp_import_file(toml_path.to_str().unwrap()).unwrap();
        assert_eq!(cands[0].name, "c");
        assert_eq!(cands[0].command, "npx");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_mcp_codex_toml_includes_env() {
        let dir = std::env::temp_dir().join(format!("agenthub-lib-exp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = McpEntryInput {
            name: "srv".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "pkg".into()],
            env: [("KEY".to_string(), "val".to_string())].into_iter().collect(),
            server_type: None,
            url: None,
            extra: Default::default(),
        };
        create_mcp(&input).unwrap();
        let dest = dir.join("out.toml");
        export_mcps(&["srv".to_string()], dest.to_str().unwrap(), "codex-toml").unwrap();
        let content = std::fs::read_to_string(&dest).unwrap();
        assert!(content.contains("[mcp_servers.srv]"));
        assert!(content.contains("command = \"npx\""));
        assert!(content.contains("KEY = \"val\""));
        delete_mcp("srv").unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skill_crud_and_import_roundtrip() {
        let created =
            create_skill("test-crud-skill", "a test", &["tdd".into(), "rust".into()], "# hello\nbody")
                .unwrap();
        assert_eq!(created.source, "托管");
        assert_eq!(created.tags, vec!["tdd", "rust"]);
        let detail = read_skill("test-crud-skill").unwrap();
        assert_eq!(detail.content.trim(), "# hello\nbody");
        assert_eq!(detail.description, "a test");
        assert_eq!(detail.tags, vec!["tdd", "rust"]);

        // list surfaces the tags too
        let listed = list_skills().unwrap();
        let item = listed.iter().find(|s| s.name == "test-crud-skill").unwrap();
        assert_eq!(item.tags, vec!["tdd", "rust"]);

        // update replaces tags; empty tags drop the key
        update_skill(
            "test-crud-skill",
            "test-crud-skill",
            "new desc",
            &["web".into()],
            "new body",
        )
        .unwrap();
        let detail = read_skill("test-crud-skill").unwrap();
        assert_eq!(detail.description, "new desc");
        assert!(detail.content.contains("new body"));
        assert_eq!(detail.tags, vec!["web"]);

        update_skill("test-crud-skill", "test-crud-skill", "new desc", &[], "new body").unwrap();
        let detail = read_skill("test-crud-skill").unwrap();
        assert!(detail.tags.is_empty());

        delete_skill("test-crud-skill").unwrap();
        assert!(read_skill("test-crud-skill").is_err());
    }

    #[test]
    fn skill_import_preserves_existing_tags() {
        let dir = std::env::temp_dir().join(format!("agenthub-tag-imp-{}", std::process::id()));
        let src = dir.join("src-skill");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: src-skill\ndescription: imported\ntags:\n  - alpha\n  - beta\n---\n\nbody",
        )
        .unwrap();
        let info = import_skill(src.to_str().unwrap(), None).unwrap();
        assert_eq!(info.tags, vec!["alpha", "beta"]);
        delete_skill("src-skill").unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preset_crud_roundtrip() {
        let dir = std::env::temp_dir().join(format!("agenthub-preset-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::session::store::SessionStore::new(&dir).unwrap();

        store
            .save_preset(
                "tdd-expert",
                "Test-driven development expert",
                "claude",
                &["dev".into(), "tdd".into()],
                "/tmp/work",
                "🧪",
                &["tdd-skill".to_string()],
                &["playwright".to_string()],
                &["tdd-prompt".to_string()],
            )
            .unwrap();
        // same name save = overwrite
        store
            .save_preset(
                "tdd-expert",
                "Updated",
                "codex",
                &[],
                "",
                "🔧",
                &["tdd-skill".to_string(), "extra".to_string()],
                &[],
                &[],
            )
            .unwrap();

        let rows = store.list_presets().unwrap();
        assert_eq!(rows.len(), 1);
        let p = &rows[0];
        assert_eq!(p.name, "tdd-expert");
        assert_eq!(p.description, "Updated");
        assert_eq!(p.agent_id, "codex");
        assert_eq!(p.icon, "🔧");
        assert_eq!(p.skills, vec!["tdd-skill".to_string(), "extra".to_string()]);
        assert!(p.mcps.is_empty());

        store.delete_preset("tdd-expert").unwrap();
        assert!(store.list_presets().unwrap().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preset_migration_is_idempotent() {
        // Re-opening the same DB re-runs the ALTER TABLE batch — must not fail.
        let dir = std::env::temp_dir().join(format!("agenthub-preset-mig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        {
            crate::session::store::SessionStore::new(&dir).unwrap();
        }
        {
            crate::session::store::SessionStore::new(&dir).unwrap();
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn instruction_crud_roundtrip() {
        create_instruction("test-inst", "# Title\nSome prompt").unwrap();
        let list = list_instructions().unwrap();
        let item = list.iter().find(|i| i.name == "test-inst").unwrap();
        assert_eq!(item.description, "Some prompt");
        let detail = read_instruction("test-inst").unwrap();
        assert!(detail.content.contains("Some prompt"));
        update_instruction("test-inst", "test-inst-renamed", "# New").unwrap();
        assert!(read_instruction("test-inst").is_err());
        assert!(read_instruction("test-inst-renamed").is_ok());
        delete_instruction("test-inst-renamed").unwrap();
    }
}
