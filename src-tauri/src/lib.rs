mod agent;
mod commands;
mod config;
mod pty;
mod session;

use tauri::Manager;

fn chrono_like_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("ts={}", now)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Panic hook: write panic details to a log file so crashes are diagnosable
    // (a panic crossing wry's extern "C" boundary aborts silently otherwise)
    let log_path = dirs::home_dir()
        .map(|h| h.join(".agenthub-panic.log"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/agenthub-panic.log"));
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!(
            "[{}] PANIC: {}\n  at: {}\n\n",
            chrono_like_now(),
            info,
            info.location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown".to_string())
        );
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = f.write_all(msg.as_bytes());
        }
        eprintln!("{}", msg);
    }));

    // Fix macOS GUI $PATH inheritance:
    // GUI apps on macOS don't inherit shell $PATH, so CLI tools won't be found.
    // We source the user's shell profile to get the real PATH.
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("/bin/zsh")
            .args(["-l", "-c", "echo $PATH"])
            .output()
        {
            if output.status.success() {
                let shell_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !shell_path.is_empty() {
                    std::env::set_var("PATH", &shell_path);
                }
            }
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize session store
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();

            let store = session::store::SessionStore::new(&app_data_dir)
                .expect("failed to init session store");

            // PTYs from a previous app run are gone — mark their sessions as exited
            // so the UI doesn't show green dots for dead sessions.
            store.mark_stale_running_exited().ok();

            app.manage(std::sync::Mutex::new(store));

            // Initialize PTY manager
            let pty_manager = pty::manager::PtyManager::new();
            app.manage(std::sync::Mutex::new(pty_manager));

            // Per-session PTY tail buffers (scrollback persistence)
            app.manage(pty::stream::ScrollbackMap::new());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Persist every session's terminal tail before the window goes away.
            // Sync + best-effort: this is the last reliable point to save.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                pty::stream::persist_all(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_agents,
            commands::list_skills,
            commands::list_mcp_servers,
            commands::launch_session,
            commands::restart_session,
            commands::pty_attach,
            commands::list_sessions,
            commands::kill_session,
            commands::delete_session,
            commands::get_scrollback_snapshot,
            commands::pty_write,
            commands::pty_resize,
            commands::select_directory,
            // Config library (skills / MCP / instructions CRUD + import/export)
            commands::library::list_library_skills,
            commands::library::read_library_skill,
            commands::library::create_library_skill,
            commands::library::update_library_skill,
            commands::library::delete_library_skill,
            commands::library::import_library_skill,
            commands::library::export_library_skill,
            commands::library::list_library_mcps,
            commands::library::create_library_mcp,
            commands::library::update_library_mcp,
            commands::library::delete_library_mcp,
            commands::library::adopt_scanned_mcp,
            commands::library::parse_mcp_import_file,
            commands::library::import_library_mcps,
            commands::library::export_library_mcps,
            commands::library::list_library_instructions,
            commands::library::read_library_instruction,
            commands::library::create_library_instruction,
            commands::library::update_library_instruction,
            commands::library::delete_library_instruction,
            commands::library::import_library_instructions,
            commands::list_presets,
            commands::save_preset,
            commands::delete_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
