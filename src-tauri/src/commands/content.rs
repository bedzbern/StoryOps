use rusqlite::{params, Connection};
use std::fs;
use tauri::State;

use crate::db;
use crate::models::*;
use crate::repo::{json_strings, map_episode, to_json_list, EPISODE_COLS};
use crate::state::{create_episode_folders, AppState};
use crate::util::{new_id, now, slugify};

fn fetch_episode(conn: &Connection, id: &str) -> Result<Episode, String> {
    let sql = format!("SELECT {EPISODE_COLS} FROM episodes e WHERE e.id = ?1");
    conn.query_row(&sql, params![id], map_episode)
        .map_err(|e| format!("Episode not found: {e}"))
}

fn next_code(conn: &Connection) -> Result<String, String> {
    let mut stmt = conn
        .prepare("SELECT code FROM episodes")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut max = 0i64;
    for row in rows {
        let code = row.map_err(|e| e.to_string())?;
        if let Some(num) = code.strip_prefix("EP-") {
            if let Ok(v) = num.trim_start_matches('0').parse::<i64>() {
                max = max.max(v);
            }
        }
    }
    Ok(format!("EP-{:04}", max + 1))
}

pub fn write_manifest(conn: &Connection, root: &std::path::Path, id: &str) -> Result<(), String> {
    let episode = fetch_episode(conn, id)?;
    let folder = root.join("Projects").join(&episode.folder);

    let scripts: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM scripts WHERE episode_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let narrations: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM narrations WHERE episode_id = ?1 AND is_current = 1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let media: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM files WHERE episode_id = ?1 AND kind = 'media'")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let native_projects: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM editing_projects WHERE episode_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let exports: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM exports WHERE episode_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let platforms: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT pl.key FROM publishing p JOIN platforms pl ON pl.id = p.platform_id
                 WHERE p.episode_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let scheduled_at: Option<String> = conn
        .query_row(
            "SELECT scheduled_at FROM publishing WHERE episode_id = ?1 AND scheduled_at IS NOT NULL
             ORDER BY scheduled_at LIMIT 1",
            params![id],
            |r| r.get(0),
        )
        .ok();

    let manifest = serde_json::json!({
        "episodeId": episode.id,
        "code": episode.code,
        "title": episode.title,
        "description": episode.description,
        "category": episode.category,
        "tags": episode.tags,
        "status": episode.status,
        "priority": episode.priority,
        "progress": episode.progress,
        "scriptRefs": scripts,
        "audioRefs": narrations,
        "mediaRefs": media,
        "nativeProjectRefs": native_projects,
        "exportRefs": exports,
        "targetPlatforms": platforms,
        "scheduledDate": scheduled_at,
        "createdAt": episode.created_at,
        "updatedAt": episode.updated_at,
        "generatedAt": now(),
    });

    if !folder.exists() {
        return Err(format!("Episode folder is missing: {}", folder.display()));
    }
    let path = folder.join("episode.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    )
    .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(())
}

#[tauri::command]
pub fn list_categories() -> Vec<String> {
    CATEGORIES.iter().map(|c| c.to_string()).collect()
}

#[tauri::command]
pub fn list_episodes(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> Result<Vec<Episode>, String> {
    let conn = state.conn()?;
    let sql = if include_archived.unwrap_or(false) {
        format!("SELECT {EPISODE_COLS} FROM episodes e ORDER BY e.updated_at DESC")
    } else {
        format!(
            "SELECT {EPISODE_COLS} FROM episodes e WHERE e.archived = 0 ORDER BY e.updated_at DESC"
        )
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_episode)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn get_episode(state: State<'_, AppState>, id: String) -> Result<Episode, String> {
    let conn = state.conn()?;
    fetch_episode(&conn, &id)
}

#[tauri::command]
pub fn create_episode(state: State<'_, AppState>, input: EpisodeInput) -> Result<Episode, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("A title is required".into());
    }
    let status = input
        .status
        .clone()
        .filter(|s| is_valid_status(s))
        .unwrap_or_else(|| "idea".into());
    let category = if input.category.trim().is_empty() {
        "Other".to_string()
    } else {
        input.category.clone()
    };
    let priority = match input.priority.as_str() {
        "high" | "low" => input.priority.clone(),
        _ => "medium".into(),
    };
    let root = state.root();
    let conn = state.conn()?;
    let code = next_code(&conn)?;
    let folder_name = format!("{}-{}", code, slugify(&title));
    let folder = create_episode_folders(&root, &folder_name)?;
    let id = new_id();
    let ts = now();

    conn.execute(
        "INSERT INTO episodes
         (id, project_id, code, title, description, category, tags, status, priority, due_date,
          progress, archived, folder, created_at, updated_at)
         VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12, ?13)",
        params![
            id,
            code,
            title,
            input.description,
            category,
            to_json_list(&input.tags),
            status,
            priority,
            input.due_date,
            status_progress(&status),
            folder.file_name().unwrap_or_default().to_string_lossy(),
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not create the episode: {e}"))?;

    conn.execute(
        "INSERT INTO episode_history (episode_id, from_status, to_status, note, created_at)
         VALUES (?1, NULL, ?2, 'Episode created', ?3)",
        params![id, status, ts],
    )
    .map_err(|e| e.to_string())?;

    db::log_activity(
        &conn,
        "episode",
        Some("episode"),
        Some(&id),
        &format!("Created {code} — {title}"),
    );

    let episode = fetch_episode(&conn, &id)?;
    let _ = write_manifest(&conn, &root, &id);
    Ok(episode)
}

#[tauri::command]
pub fn update_episode(state: State<'_, AppState>, input: EpisodeUpdate) -> Result<Episode, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("A title is required".into());
    }
    let root = state.root();
    let conn = state.conn()?;
    let before = fetch_episode(&conn, &input.id)?;
    let archived = input.archived.unwrap_or(before.archived);
    let priority = match input.priority.as_str() {
        "high" | "low" => input.priority.clone(),
        _ => "medium".into(),
    };
    conn.execute(
        "UPDATE episodes SET title = ?1, description = ?2, category = ?3, tags = ?4,
         priority = ?5, due_date = ?6, archived = ?7, updated_at = ?8 WHERE id = ?9",
        params![
            title,
            input.description,
            if input.category.trim().is_empty() { "Other".to_string() } else { input.category },
            to_json_list(&input.tags),
            priority,
            input.due_date,
            if archived { 1 } else { 0 },
            now(),
            input.id
        ],
    )
    .map_err(|e| format!("Could not update the episode: {e}"))?;

    db::log_activity(
        &conn,
        "episode",
        Some("episode"),
        Some(&input.id),
        &format!("Updated {} — {}", before.code, title),
    );
    let episode = fetch_episode(&conn, &input.id)?;
    let _ = write_manifest(&conn, &root, &input.id);
    Ok(episode)
}

#[tauri::command]
pub fn set_episode_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
    note: Option<String>,
) -> Result<Episode, String> {
    if !is_valid_status(&status) {
        return Err(format!("Unknown production status: {status}"));
    }
    let root = state.root();
    let conn = state.conn()?;
    let before = fetch_episode(&conn, &id)?;
    let progress = status_progress(&status);
    conn.execute(
        "UPDATE episodes SET status = ?1, progress = ?2, updated_at = ?3 WHERE id = ?4",
        params![status, progress, now(), id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO episode_history (episode_id, from_status, to_status, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            before.status,
            status,
            note.unwrap_or_default(),
            now()
        ],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "episode",
        Some("episode"),
        Some(&id),
        &format!(
            "{} moved from {} to {}",
            before.code,
            status_label(&before.status),
            status_label(&status)
        ),
    );
    let episode = fetch_episode(&conn, &id)?;
    let _ = write_manifest(&conn, &root, &id);
    Ok(episode)
}

#[tauri::command]
pub fn archive_episode(state: State<'_, AppState>, id: String, archived: bool) -> Result<(), String> {
    let conn = state.conn()?;
    let episode = fetch_episode(&conn, &id)?;
    conn.execute(
        "UPDATE episodes SET archived = ?1, updated_at = ?2 WHERE id = ?3",
        params![if archived { 1 } else { 0 }, now(), id],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "episode",
        Some("episode"),
        Some(&id),
        &format!(
            "{} {}",
            episode.code,
            if archived { "archived" } else { "restored" }
        ),
    );
    Ok(())
}

/// Deletes an episode. The working folder is moved into Backups/Deleted so no
/// production file is destroyed without a recoverable copy.
#[tauri::command]
pub fn delete_episode(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let root = state.root();
    let conn = state.conn()?;
    let episode = fetch_episode(&conn, &id)?;
    let folder = root.join("Projects").join(&episode.folder);
    let mut moved_to = None;
    if folder.exists() {
        let trash = root.join("Backups").join("Deleted");
        fs::create_dir_all(&trash).map_err(|e| format!("Could not create {}: {e}", trash.display()))?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let dest = trash.join(format!("{}-{}", episode.folder, stamp));
        fs::rename(&folder, &dest)
            .map_err(|e| format!("Could not move the episode folder to the recycle folder: {e}"))?;
        moved_to = Some(dest);
    }
    conn.execute("DELETE FROM episodes WHERE id = ?1", params![id])
        .map_err(|e| format!("Could not delete the episode: {e}"))?;
    db::log_activity(
        &conn,
        "episode",
        Some("episode"),
        Some(&id),
        &format!(
            "Deleted {} — {}{}",
            episode.code,
            episode.title,
            moved_to
                .map(|p| format!(" (folder moved to {})", p.display()))
                .unwrap_or_default()
        ),
    );
    Ok(())
}

#[tauri::command]
pub fn list_episode_history(
    state: State<'_, AppState>,
    episode_id: String,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, episode_id, from_status, to_status, note, created_at
             FROM episode_history WHERE episode_id = ?1 ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![episode_id], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?,
                episode_id: r.get(1)?,
                from_status: r.get(2)?,
                to_status: r.get(3)?,
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

// ---------------------------------------------------------------- ideas

fn map_idea(row: &rusqlite::Row<'_>) -> rusqlite::Result<Idea> {
    Ok(Idea {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        category: row.get(3)?,
        tags: json_strings(row.get::<_, String>(4)?),
        priority: row.get(5)?,
        status: row.get(6)?,
        notes: row.get(7)?,
        converted_episode_id: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const IDEA_COLS: &str = "id, title, body, category, tags, priority, status, notes,
    converted_episode_id, created_at, updated_at";

#[tauri::command]
pub fn list_ideas(state: State<'_, AppState>) -> Result<Vec<Idea>, String> {
    let conn = state.conn()?;
    let sql = format!(
        "SELECT {IDEA_COLS} FROM ideas ORDER BY
         CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, updated_at DESC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_idea).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn create_idea(state: State<'_, AppState>, input: IdeaInput) -> Result<Idea, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("An idea needs a title".into());
    }
    let conn = state.conn()?;
    let id = new_id();
    let ts = now();
    let status = if input.status.is_empty() { "inbox".into() } else { input.status };
    let priority = match input.priority.as_str() {
        "high" | "low" => input.priority.clone(),
        _ => "medium".into(),
    };
    conn.execute(
        "INSERT INTO ideas (id, title, body, category, tags, priority, status, notes,
         converted_episode_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)",
        params![
            id,
            title,
            input.body,
            if input.category.trim().is_empty() { "Other".to_string() } else { input.category },
            to_json_list(&input.tags),
            priority,
            status,
            input.notes,
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not save the idea: {e}"))?;
    db::log_activity(&conn, "idea", Some("idea"), Some(&id), &format!("New idea — {title}"));
    conn.query_row(
        &format!("SELECT {IDEA_COLS} FROM ideas WHERE id = ?1"),
        params![id],
        map_idea,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_idea(
    state: State<'_, AppState>,
    id: String,
    input: IdeaInput,
) -> Result<Idea, String> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err("An idea needs a title".into());
    }
    let conn = state.conn()?;
    conn.execute(
        "UPDATE ideas SET title = ?1, body = ?2, category = ?3, tags = ?4, priority = ?5,
         status = ?6, notes = ?7, updated_at = ?8 WHERE id = ?9",
        params![
            title,
            input.body,
            if input.category.trim().is_empty() { "Other".to_string() } else { input.category },
            to_json_list(&input.tags),
            match input.priority.as_str() { "high" | "low" => input.priority.clone(), _ => "medium".into() },
            if input.status.is_empty() { "inbox".to_string() } else { input.status },
            input.notes,
            now(),
            id
        ],
    )
    .map_err(|e| format!("Could not update the idea: {e}"))?;
    db::log_activity(&conn, "idea", Some("idea"), Some(&id), &format!("Updated idea — {title}"));
    conn.query_row(
        &format!("SELECT {IDEA_COLS} FROM ideas WHERE id = ?1"),
        params![id],
        map_idea,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_idea(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    let title: String = conn
        .query_row("SELECT title FROM ideas WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| format!("Idea not found: {e}"))?;
    conn.execute("DELETE FROM ideas WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    db::log_activity(&conn, "idea", Some("idea"), Some(&id), &format!("Deleted idea — {title}"));
    Ok(())
}

#[tauri::command]
pub fn convert_idea_to_episode(
    state: State<'_, AppState>,
    id: String,
    category: Option<String>,
) -> Result<Episode, String> {
    let conn = state.conn()?;
    let idea = conn
        .query_row(
            &format!("SELECT {IDEA_COLS} FROM ideas WHERE id = ?1"),
            params![id],
            map_idea,
        )
        .map_err(|e| format!("Idea not found: {e}"))?;

    // Build the episode inside the same connection, then link the idea.
    let root = state.root();
    let code = next_code(&conn)?;
    let folder_name = format!("{}-{}", code, slugify(&idea.title));
    drop(conn);
    let folder = create_episode_folders(&root, &folder_name)?;
    let conn = state.conn()?;
    let episode_id = new_id();
    let ts = now();
    let category = category.unwrap_or_else(|| idea.category.clone());
    let description = if idea.body.trim().is_empty() {
        idea.notes.clone()
    } else {
        idea.body.clone()
    };

    conn.execute(
        "INSERT INTO episodes
         (id, project_id, code, title, description, category, tags, status, priority, due_date,
          progress, archived, folder, created_at, updated_at)
         VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 'idea', ?7, NULL, 0, 0, ?8, ?9, ?10)",
        params![
            episode_id,
            code,
            idea.title,
            description,
            category,
            to_json_list(&idea.tags),
            idea.priority,
            folder.file_name().unwrap_or_default().to_string_lossy(),
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not create the episode: {e}"))?;
    conn.execute(
        "INSERT INTO episode_history (episode_id, from_status, to_status, note, created_at)
         VALUES (?1, NULL, 'idea', 'Created from an idea', ?2)",
        params![episode_id, ts],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ideas SET status = 'converted', converted_episode_id = ?1, updated_at = ?2
         WHERE id = ?3",
        params![episode_id, ts, id],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "idea",
        Some("idea"),
        Some(&id),
        &format!("Converted idea \"{}\" into {code}", idea.title),
    );
    let episode = fetch_episode(&conn, &episode_id)?;
    let _ = write_manifest(&conn, &root, &episode_id);
    Ok(episode)
}
