//! Tauri commands for the AgentHub-managed config library
//! (~/.agenthub/library — skills, MCP servers, instruction/prompt files).

use super::{
    LibrarySkillDetail, InstructionDetail, InstructionInfo, McpEntryInput, McpImportCandidate,
    McpImportResult, SkillInfo, McpServerInfo,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Skills
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_library_skills() -> Result<Vec<SkillInfo>, String> {
    crate::config::library::list_skills()
}

#[tauri::command]
pub fn read_library_skill(name: String) -> Result<LibrarySkillDetail, String> {
    crate::config::library::read_skill(&name).map(|d| LibrarySkillDetail {
        name: d.name,
        description: d.description,
        tags: d.tags,
        path: d.path,
        content: d.content,
        files: d.files,
    })
}

#[tauri::command]
pub fn create_library_skill(
    name: String,
    description: String,
    tags: Vec<String>,
    content: String,
) -> Result<SkillInfo, String> {
    crate::config::library::create_skill(&name, &description, &tags, &content)
}

#[tauri::command]
pub fn update_library_skill(
    name: String,
    new_name: String,
    description: String,
    tags: Vec<String>,
    content: String,
) -> Result<SkillInfo, String> {
    crate::config::library::update_skill(&name, &new_name, &description, &tags, &content)
}

#[tauri::command]
pub fn delete_library_skill(name: String) -> Result<(), String> {
    crate::config::library::delete_skill(&name)
}

#[tauri::command]
pub fn import_library_skill(source_dir: String, name: Option<String>) -> Result<SkillInfo, String> {
    crate::config::library::import_skill(&source_dir, name.as_deref())
}

#[tauri::command]
pub fn export_library_skill(name: String, dest_dir: String) -> Result<String, String> {
    crate::config::library::export_skill(&name, &dest_dir)
}

// ---------------------------------------------------------------------------
// MCP servers
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_library_mcps() -> Result<Vec<McpServerInfo>, String> {
    crate::config::library::list_mcps()
}

#[tauri::command]
pub fn create_library_mcp(entry: McpEntryInput) -> Result<(), String> {
    crate::config::library::create_mcp(&entry)
}

#[tauri::command]
pub fn update_library_mcp(old_name: String, entry: McpEntryInput) -> Result<(), String> {
    crate::config::library::update_mcp(&old_name, &entry)
}

#[tauri::command]
pub fn delete_library_mcp(name: String) -> Result<(), String> {
    crate::config::library::delete_mcp(&name)
}

#[tauri::command]
pub fn adopt_scanned_mcp(name: String) -> Result<(), String> {
    crate::config::library::adopt_scanned_mcp(&name)
}

#[tauri::command]
pub fn parse_mcp_import_file(path: String) -> Result<Vec<McpImportCandidate>, String> {
    crate::config::library::parse_mcp_import_file(&path)
}

#[tauri::command]
pub fn import_library_mcps(path: String, names: Vec<String>) -> Result<McpImportResult, String> {
    crate::config::library::import_mcps(&path, &names)
}

#[tauri::command]
pub fn export_library_mcps(
    names: Vec<String>,
    dest_path: String,
    format: String,
) -> Result<String, String> {
    crate::config::library::export_mcps(&names, &dest_path, &format)
}

// ---------------------------------------------------------------------------
// Instructions / prompts
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_library_instructions() -> Result<Vec<InstructionInfo>, String> {
    crate::config::library::list_instructions()
}

#[tauri::command]
pub fn read_library_instruction(name: String) -> Result<InstructionDetail, String> {
    crate::config::library::read_instruction(&name)
}

#[tauri::command]
pub fn create_library_instruction(name: String, content: String) -> Result<String, String> {
    crate::config::library::create_instruction(&name, &content)
}

#[tauri::command]
pub fn update_library_instruction(
    name: String,
    new_name: String,
    content: String,
) -> Result<String, String> {
    crate::config::library::update_instruction(&name, &new_name, &content)
}

#[tauri::command]
pub fn delete_library_instruction(name: String) -> Result<(), String> {
    crate::config::library::delete_instruction(&name)
}

#[tauri::command]
pub fn import_library_instructions(paths: Vec<String>) -> Result<Vec<String>, String> {
    crate::config::library::import_instructions(&paths)
}

// Re-export for convenience of callers constructing inputs (unused now, but
// keeps HashMap import meaningful if signatures evolve).
#[allow(unused_imports)]
use HashMap as _HashMapAlias;
