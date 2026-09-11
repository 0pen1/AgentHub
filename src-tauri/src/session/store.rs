use rusqlite::{params, Connection};
use std::path::Path;

pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    pub fn new(app_data_dir: &Path) -> Result<Self, rusqlite::Error> {
        let db_path = app_data_dir.join("sessions.db");
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                name TEXT NOT NULL,
                work_dir TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                skills TEXT NOT NULL DEFAULT '[]',
                mcps TEXT NOT NULL DEFAULT '[]',
                instructions TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS presets (
                name TEXT PRIMARY KEY,
                skills TEXT NOT NULL DEFAULT '[]',
                mcps TEXT NOT NULL DEFAULT '[]',
                instructions TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );",
        )?;

        // presets v2 columns (expert = fixed agent + skill/mcp/prompt combo).
        // Idempotent: ignore "duplicate column" errors from already-migrated DBs.
        for col in [
            "ALTER TABLE presets ADD COLUMN description TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE presets ADD COLUMN agent_id TEXT NOT NULL DEFAULT 'claude'",
            "ALTER TABLE presets ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE presets ADD COLUMN work_dir TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE presets ADD COLUMN icon TEXT NOT NULL DEFAULT '🤖'",
        ] {
            let _ = conn.execute_batch(col);
        }

        Ok(Self { conn })
    }

    pub fn insert_session(
        &self,
        id: &str,
        agent_id: &str,
        agent_name: &str,
        name: &str,
        work_dir: &str,
        skills: &[String],
        mcps: &[String],
        instructions: &[String],
    ) -> Result<(), rusqlite::Error> {
        let now = timestamp_now();
        self.conn.execute(
            "INSERT INTO sessions (id, agent_id, agent_name, name, work_dir, status, skills, mcps, instructions, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                agent_id,
                agent_name,
                name,
                work_dir,
                serde_json::to_string(skills).unwrap_or_default(),
                serde_json::to_string(mcps).unwrap_or_default(),
                serde_json::to_string(instructions).unwrap_or_default(),
                now,
            ],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_status(&self, id: &str, status: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, timestamp_now(), id],
        )?;
        Ok(())
    }

    /// Conditional update: only flip "running" to another status (preserves "killed").
    /// Used by the PTY reader on natural exit to avoid overwriting explicit kill status.
    pub fn update_status_if_running(
        &self,
        id: &str,
        new_status: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status = 'running'",
            params![new_status, timestamp_now(), id],
        )?;
        Ok(())
    }

    /// Mark all "running" sessions as "exited" (called at app startup to fix stale state
    /// from previous runs — PTYs died with the old process, but DB still said running).
    pub fn mark_stale_running_exited(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE sessions SET status = 'exited', updated_at = ?1 WHERE status = 'running'",
            params![timestamp_now()],
        )?;
        Ok(())
    }

    /// Retrieve full session config for restart (agent, work_dir, and selected skills/mcps/instructions).
    pub fn get_session(
        &self,
        id: &str,
    ) -> Result<
        Option<(String, String, String, String, Vec<String>, Vec<String>, Vec<String>)>,
        rusqlite::Error,
    > {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id, agent_name, name, work_dir, skills, mcps, instructions FROM sessions WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            let skills_json: String = row.get(4)?;
            let mcps_json: String = row.get(5)?;
            let instructions_json: String = row.get(6)?;
            let skills: Vec<String> =
                serde_json::from_str(&skills_json).unwrap_or_default();
            let mcps: Vec<String> = serde_json::from_str(&mcps_json).unwrap_or_default();
            let instructions: Vec<String> =
                serde_json::from_str(&instructions_json).unwrap_or_default();
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                skills,
                mcps,
                instructions,
            ))
        });
        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_sessions(
        &self,
    ) -> Vec<(String, String, String, String, String, String, String)> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, agent_id, agent_name, name, work_dir, status, created_at FROM sessions ORDER BY created_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        });
        match rows {
            // A broken query must not take the app down — return nothing.
            Err(_) => vec![],
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        }
    }

    pub fn delete_session(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// All session ids (for scrollback snapshot GC at startup).
    pub fn all_session_ids(&self) -> std::collections::HashSet<String> {
        let mut stmt = match self.conn.prepare("SELECT id FROM sessions") {
            Ok(s) => s,
            Err(_) => return Default::default(),
        };
        let rows = stmt.query_map([], |row| row.get::<_, String>(0));
        match rows {
            Err(_) => Default::default(),
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        }
    }

    // --- Presets (experts: fixed agent + skill/mcp/prompt combos) ------------

    /// Insert or overwrite a preset by name (name is the primary key).
    #[allow(clippy::too_many_arguments)]
    pub fn save_preset(
        &self,
        name: &str,
        description: &str,
        agent_id: &str,
        tags: &[String],
        work_dir: &str,
        icon: &str,
        skills: &[String],
        mcps: &[String],
        instructions: &[String],
    ) -> Result<(), rusqlite::Error> {
        let now = timestamp_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO presets
             (name, description, agent_id, tags, work_dir, icon, skills, mcps, instructions, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                name,
                description,
                agent_id,
                serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string()),
                work_dir,
                icon,
                serde_json::to_string(skills).unwrap_or_else(|_| "[]".to_string()),
                serde_json::to_string(mcps).unwrap_or_else(|_| "[]".to_string()),
                serde_json::to_string(instructions).unwrap_or_else(|_| "[]".to_string()),
                now,
            ],
        )?;
        Ok(())
    }

    /// All presets, newest first.
    pub fn list_presets(
        &self,
    ) -> Result<Vec<PresetRow>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT name, description, agent_id, tags, work_dir, icon, skills, mcps, instructions
             FROM presets ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let parse = |raw: String| -> Vec<String> {
                    serde_json::from_str(&raw).unwrap_or_default()
                };
                Ok(PresetRow {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    agent_id: row.get(2)?,
                    tags: parse(row.get(3)?),
                    work_dir: row.get(4)?,
                    icon: row.get(5)?,
                    skills: parse(row.get(6)?),
                    mcps: parse(row.get(7)?),
                    instructions: parse(row.get(8)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn delete_preset(&self, name: &str) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM presets WHERE name = ?1", params![name])?;
        Ok(())
    }
}

/// Full preset row (expert definition).
pub struct PresetRow {
    pub name: String,
    pub description: String,
    pub agent_id: String,
    pub tags: Vec<String>,
    pub work_dir: String,
    pub icon: String,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub instructions: Vec<String>,
}

fn timestamp_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}
