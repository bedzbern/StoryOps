use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::State;

use crate::db;
use crate::models::*;
use crate::repo::{json_strings, to_json_list};
use crate::state::AppState;
use crate::util::{count_words, estimate_duration_sec, new_id, now};

fn map_script(row: &rusqlite::Row<'_>) -> rusqlite::Result<Script> {
    Ok(Script {
        id: row.get(0)?,
        episode_id: row.get(1)?,
        episode_title: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        word_count: row.get(5)?,
        est_duration_sec: row.get(6)?,
        current_version: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const SCRIPT_SELECT: &str = "SELECT s.id, s.episode_id, e.title, s.title, s.content, s.word_count,
    s.est_duration_sec, s.current_version, s.created_at, s.updated_at
    FROM scripts s LEFT JOIN episodes e ON e.id = s.episode_id";

#[tauri::command]
pub fn list_scripts(state: State<'_, AppState>) -> Result<Vec<Script>, String> {
    let conn = state.conn()?;
    let sql = format!("{SCRIPT_SELECT} ORDER BY s.updated_at DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_script).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn get_script(state: State<'_, AppState>, id: String) -> Result<Script, String> {
    let conn = state.conn()?;
    conn.query_row(
        &format!("{SCRIPT_SELECT} WHERE s.id = ?1"),
        params![id],
        map_script,
    )
    .map_err(|e| format!("Script not found: {e}"))
}

fn create_script_inner(
    conn: &Connection,
    title: &str,
    episode_id: Option<String>,
    content: Option<String>,
) -> Result<String, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("A script needs a title".into());
    }
    if let Some(ref eid) = episode_id {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM episodes WHERE id = ?1",
                params![eid],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists == 0 {
            return Err("That episode no longer exists".into());
        }
    }
    let id = new_id();
    let ts = now();
    let body = content.unwrap_or_default();
    let words = count_words(&body);
    conn.execute(
        "INSERT INTO scripts (id, episode_id, title, content, word_count, est_duration_sec,
         current_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
        params![
            id,
            episode_id,
            title,
            body,
            words,
            estimate_duration_sec(words),
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not create the script: {e}"))?;
    conn.execute(
        "INSERT INTO script_versions (id, script_id, version, content, word_count, note, created_at)
         VALUES (?1, ?2, 1, ?3, ?4, 'Initial draft', ?5)",
        params![new_id(), id, body, words, ts],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        conn,
        "script",
        Some("script"),
        Some(&id),
        &format!("Created script — {title}"),
    );
    Ok(id)
}

#[tauri::command]
pub fn create_script(
    state: State<'_, AppState>,
    title: String,
    episode_id: Option<String>,
    content: Option<String>,
) -> Result<Script, String> {
    let conn = state.conn()?;
    let id = create_script_inner(&conn, &title, episode_id, content)?;
    conn.query_row(
        &format!("{SCRIPT_SELECT} WHERE s.id = ?1"),
        params![id],
        map_script,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_script(
    state: State<'_, AppState>,
    id: String,
    title: String,
    content: String,
    note: Option<String>,
) -> Result<Script, String> {
    let conn = state.conn()?;
    let (current, current_version): (String, i64) = conn
        .query_row(
            "SELECT content, current_version FROM scripts WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("Script not found: {e}"))?;
    let words = count_words(&content);
    let ts = now();
    let changed = current != content;
    let version = if changed { current_version + 1 } else { current_version };

    conn.execute(
        "UPDATE scripts SET title = ?1, content = ?2, word_count = ?3, est_duration_sec = ?4,
         current_version = ?5, updated_at = ?6 WHERE id = ?7",
        params![
            title.trim(),
            content,
            words,
            estimate_duration_sec(words),
            version,
            ts,
            id
        ],
    )
    .map_err(|e| format!("Could not save the script: {e}"))?;

    if changed {
        conn.execute(
            "INSERT INTO script_versions (id, script_id, version, content, word_count, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                id,
                version,
                content,
                words,
                note.unwrap_or_else(|| "Saved".into()),
                ts
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    db::log_activity(
        &conn,
        "script",
        Some("script"),
        Some(&id),
        &format!("Saved script \"{}\" (v{version})", title.trim()),
    );
    conn.query_row(
        &format!("{SCRIPT_SELECT} WHERE s.id = ?1"),
        params![id],
        map_script,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_script(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    let title: String = conn
        .query_row("SELECT title FROM scripts WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| format!("Script not found: {e}"))?;
    conn.execute("DELETE FROM scripts WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "script",
        Some("script"),
        Some(&id),
        &format!("Deleted script — {title}"),
    );
    Ok(())
}

#[tauri::command]
pub fn list_script_versions(
    state: State<'_, AppState>,
    script_id: String,
) -> Result<Vec<ScriptVersion>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, script_id, version, word_count, note, created_at
             FROM script_versions WHERE script_id = ?1 ORDER BY version DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![script_id], |r| {
            Ok(ScriptVersion {
                id: r.get(0)?,
                script_id: r.get(1)?,
                version: r.get(2)?,
                word_count: r.get(3)?,
                note: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn get_script_version(state: State<'_, AppState>, version_id: String) -> Result<String, String> {
    let conn = state.conn()?;
    conn.query_row(
        "SELECT content FROM script_versions WHERE id = ?1",
        params![version_id],
        |r| r.get(0),
    )
    .map_err(|e| format!("Version not found: {e}"))
}

#[tauri::command]
pub fn restore_script_version(
    state: State<'_, AppState>,
    script_id: String,
    version_id: String,
) -> Result<Script, String> {
    let conn = state.conn()?;
    let (content, version, title): (String, i64, String) = conn
        .query_row(
            "SELECT v.content, v.version, s.title FROM script_versions v
             JOIN scripts s ON s.id = v.script_id WHERE v.id = ?1 AND v.script_id = ?2",
            params![version_id, script_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| format!("Version not found: {e}"))?;
    let words = count_words(&content);
    let current_version: i64 = conn
        .query_row(
            "SELECT current_version FROM scripts WHERE id = ?1",
            params![script_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let next_version = current_version + 1;
    let ts = now();
    conn.execute(
        "UPDATE scripts SET content = ?1, word_count = ?2, est_duration_sec = ?3,
         current_version = ?4, updated_at = ?5 WHERE id = ?6",
        params![
            content,
            words,
            estimate_duration_sec(words),
            next_version,
            ts,
            script_id
        ],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO script_versions (id, script_id, version, content, word_count, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            new_id(),
            script_id,
            next_version,
            content,
            words,
            format!("Restored from version {version}"),
            ts
        ],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "script",
        Some("script"),
        Some(&script_id),
        &format!("Restored \"{title}\" from version {version}"),
    );
    conn.query_row(
        &format!("{SCRIPT_SELECT} WHERE s.id = ?1"),
        params![script_id],
        map_script,
    )
    .map_err(|e| e.to_string())
}

const TEXT_EXTS: [&str; 5] = ["md", "markdown", "txt", "text", "fountain"];

#[tauri::command]
pub fn import_script_file(
    state: State<'_, AppState>,
    path: String,
    episode_id: Option<String>,
) -> Result<Script, String> {
    crate::util::validate_path_string(&path)?;
    let p = PathBuf::from(&path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !TEXT_EXTS.contains(&ext.as_str()) {
        return Err("Only Markdown, text and Fountain files can be imported as scripts".into());
    }
    let meta = fs::metadata(&p).map_err(|e| format!("Could not read the file: {e}"))?;
    if meta.len() > 5 * 1024 * 1024 {
        return Err("That file is larger than 5 MB; scripts should be plain text".into());
    }
    let content =
        fs::read_to_string(&p).map_err(|e| format!("Could not read the file as UTF-8 text: {e}"))?;
    let title = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Imported script".into());
    let conn = state.conn()?;
    let id = create_script_inner(&conn, &title, episode_id, Some(content))?;
    db::log_activity(
        &conn,
        "script",
        Some("script"),
        Some(&id),
        &format!("Imported script from {}", p.display()),
    );
    conn.query_row(
        &format!("{SCRIPT_SELECT} WHERE s.id = ?1"),
        params![id],
        map_script,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_script(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<String, String> {
    crate::util::validate_path_string(&path)?;
    let target = PathBuf::from(&path);
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !TEXT_EXTS.contains(&ext.as_str()) {
        return Err("Choose a .md or .txt file name".into());
    }
    if target.exists() {
        return Err("A file with that name already exists; choose another name".into());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let conn = state.conn()?;
    let (title, content): (String, String) = conn
        .query_row(
            "SELECT title, content FROM scripts WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("Script not found: {e}"))?;
    fs::write(&target, content).map_err(|e| format!("Could not write {}: {e}", target.display()))?;
    db::log_activity(
        &conn,
        "script",
        Some("script"),
        Some(&id),
        &format!("Exported \"{title}\" to {}", target.display()),
    );
    Ok(target.display().to_string())
}

// ------------------------------------------------------- prompt templates

fn map_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptTemplate> {
    Ok(PromptTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        instructions: row.get(3)?,
        variables: json_strings(row.get::<_, String>(4)?),
        provider: row.get(5)?,
        builtin: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const TEMPLATE_COLS: &str = "id, name, category, instructions, variables, provider, builtin,
    created_at, updated_at";

#[tauri::command]
pub fn list_prompt_templates(state: State<'_, AppState>) -> Result<Vec<PromptTemplate>, String> {
    let conn = state.conn()?;
    let sql = format!("SELECT {TEMPLATE_COLS} FROM prompt_templates ORDER BY builtin DESC, name");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_template)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn save_prompt_template(
    state: State<'_, AppState>,
    id: Option<String>,
    input: TemplateInput,
) -> Result<PromptTemplate, String> {
    let conn = state.conn()?;
    if input.instructions.trim().is_empty() {
        return Err("Template instructions cannot be empty".into());
    }
    let ts = now();
    let template_id = match id {
        Some(existing) => {
            let builtin: i64 = conn
                .query_row(
                    "SELECT builtin FROM prompt_templates WHERE id = ?1",
                    params![existing],
                    |r| r.get(0),
                )
                .map_err(|e| format!("Template not found: {e}"))?;
            if builtin == 1 {
                return Err(
                    "Built-in templates cannot be edited. Duplicate it to customise it.".into(),
                );
            }
            conn.execute(
                "UPDATE prompt_templates SET name = ?1, category = ?2, instructions = ?3,
                 variables = ?4, provider = ?5, updated_at = ?6 WHERE id = ?7",
                params![
                    input.name,
                    input.category,
                    input.instructions,
                    to_json_list(&input.variables),
                    if input.provider.is_empty() { "any".to_string() } else { input.provider },
                    ts,
                    existing
                ],
            )
            .map_err(|e| e.to_string())?;
            existing
        }
        None => {
            let new = new_id();
            conn.execute(
                "INSERT INTO prompt_templates (id, name, category, instructions, variables,
                 provider, builtin, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8)",
                params![
                    new,
                    input.name,
                    input.category,
                    input.instructions,
                    to_json_list(&input.variables),
                    if input.provider.is_empty() { "any".to_string() } else { input.provider },
                    ts,
                    ts
                ],
            )
            .map_err(|e| format!("Could not save the template: {e}"))?;
            new
        }
    };
    db::log_activity(
        &conn,
        "template",
        Some("prompt_template"),
        Some(&template_id),
        &format!("Saved prompt template — {}", input.name),
    );
    conn.query_row(
        &format!("SELECT {TEMPLATE_COLS} FROM prompt_templates WHERE id = ?1"),
        params![template_id],
        map_template,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_prompt_template(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    let builtin: i64 = conn
        .query_row(
            "SELECT builtin FROM prompt_templates WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Template not found: {e}"))?;
    if builtin == 1 {
        return Err("Built-in templates cannot be deleted".into());
    }
    conn.execute("DELETE FROM prompt_templates WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn render_prompt(
    state: State<'_, AppState>,
    template_id: String,
    episode_id: Option<String>,
    values: Option<HashMap<String, String>>,
) -> Result<String, String> {
    let conn = state.conn()?;
    let instructions: String = conn
        .query_row(
            "SELECT instructions FROM prompt_templates WHERE id = ?1",
            params![template_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Template not found: {e}"))?;

    let mut vars: HashMap<String, String> = HashMap::new();
    if let Some(id) = episode_id {
        let sql = format!(
            "SELECT {0} FROM episodes e WHERE e.id = ?1",
            crate::repo::EPISODE_COLS
        );
        let episode = conn
            .query_row(&sql, params![id], crate::repo::map_episode)
            .map_err(|e| format!("Episode not found: {e}"))?;
        vars.insert("title".into(), episode.title.clone());
        vars.insert("category".into(), episode.category.clone());
        vars.insert("notes".into(), episode.description.clone());
        vars.insert("code".into(), episode.code.clone());
        let script: String = conn
            .query_row(
                "SELECT content FROM scripts WHERE episode_id = ?1 ORDER BY updated_at DESC LIMIT 1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        vars.insert("script".into(), script);
    }
    if let Some(extra) = values {
        for (k, v) in extra {
            vars.insert(k, v);
        }
    }
    vars.entry("duration".into()).or_insert_with(|| "10".into());
    for key in ["title", "category", "notes", "script", "code"] {
        vars.entry(key.into()).or_default();
    }

    let mut out = instructions;
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), &value);
    }
    Ok(out)
}
