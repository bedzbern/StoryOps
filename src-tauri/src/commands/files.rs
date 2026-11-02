use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::State;

use crate::db;
use crate::models::*;
use crate::state::AppState;
use crate::util::{is_within, new_id, now, validate_path_string};

const VIDEO_EXTS: [&str; 12] = [
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "flv", "mts", "m2ts", "mpg", "mpeg",
];
const AUDIO_EXTS: [&str; 8] = ["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "wma"];
const IMAGE_EXTS: [&str; 10] = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "svg", "exr"];
const DOC_EXTS: [&str; 14] = [
    "pdf", "docx", "doc", "odt", "rtf", "md", "txt", "html", "htm", "tex", "json", "xlsx", "csv",
    "pptx",
];
const SUB_EXTS: [&str; 6] = ["srt", "vtt", "ass", "ssa", "sub", "sbv"];
const PROJECT_EXTS: [&str; 14] = [
    "fcproj", "pcraft", "scraft", "deckcraft", "designcraft", "figcraft", "penpot", "sc", "step",
    "ait", "dwg", "dxf", "dwt", "cany",
];

pub fn infer_kind(ext: &str) -> &'static str {
    if VIDEO_EXTS.contains(&ext) {
        "video"
    } else if AUDIO_EXTS.contains(&ext) {
        "audio"
    } else if IMAGE_EXTS.contains(&ext) {
        "image"
    } else if SUB_EXTS.contains(&ext) {
        "subtitle"
    } else if PROJECT_EXTS.contains(&ext) {
        "project"
    } else if DOC_EXTS.contains(&ext) {
        "document"
    } else {
        "other"
    }
}

fn modified_iso(meta: &fs::Metadata) -> Option<String> {
    meta.modified().ok().and_then(|t| {
        let dt: chrono::DateTime<chrono::Utc> = t.into();
        Some(dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    })
}

fn map_file(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileEntry> {
    Ok(FileEntry {
        id: row.get(0)?,
        episode_id: row.get(1)?,
        episode_title: row.get(2)?,
        kind: row.get(3)?,
        name: row.get(4)?,
        path: row.get(5)?,
        ext: row.get(6)?,
        size_bytes: row.get(7)?,
        modified_at: row.get(8)?,
        app_id: row.get(9)?,
        app_name: row.get(10)?,
        missing: row.get::<_, i64>(11)? != 0,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

const FILE_SELECT: &str = "SELECT f.id, f.episode_id, e.title, f.kind, f.name, f.path, f.ext,
    f.size_bytes, f.modified_at, f.app_id, a.name, f.missing, f.created_at, f.updated_at
    FROM files f
    LEFT JOIN episodes e ON e.id = f.episode_id
    LEFT JOIN applications a ON a.id = f.app_id";

fn fetch_file(conn: &Connection, id: &str) -> Result<FileEntry, String> {
    conn.query_row(
        &format!("{FILE_SELECT} WHERE f.id = ?1"),
        params![id],
        map_file,
    )
    .map_err(|e| format!("File record not found: {e}"))
}

fn ensure_inside(root: &Path, p: &Path) -> Result<(), String> {
    if !is_within(root, p) {
        return Err("That path is outside the StoryOps library".into());
    }
    Ok(())
}

fn valid_child_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    if name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("The name contains characters Windows does not allow".into());
    }
    if name == "." || name == ".." {
        return Err("That name is not allowed".into());
    }
    if name.len() > 200 {
        return Err("The name is too long".into());
    }
    Ok(())
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let mut candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut i = 2;
    loop {
        let candidate_name = if ext.is_empty() {
            format!("{stem} ({i})")
        } else {
            format!("{stem} ({i}).{ext}")
        };
        candidate = dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
        i += 1;
    }
}

fn record_version(conn: &Connection, file_id: &str, path: &str, size: i64, modified: Option<&String>, note: &str) {
    let _ = conn.execute(
        "INSERT INTO file_versions (file_id, path, size_bytes, modified_at, hash, note, created_at)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
        params![file_id, path, size, modified, note, now()],
    );
}

fn register_path(
    conn: &Connection,
    root: &Path,
    episode_id: Option<&str>,
    path: &Path,
    kind: Option<&str>,
    app_id: Option<&str>,
) -> Result<String, String> {
    ensure_inside(root, path)?;
    let meta = fs::metadata(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let kind = kind.map(|k| k.to_string()).unwrap_or_else(|| infer_kind(&ext).to_string());
    let ts = now();
    let modified = modified_iso(&meta);
    let path_str = path.display().to_string();

    let existing: Option<String> = conn
        .query_row("SELECT id FROM files WHERE path = ?1", params![path_str], |r| r.get(0))
        .ok();

    if let Some(id) = existing {
        conn.execute(
            "UPDATE files SET name = ?1, ext = ?2, size_bytes = ?3, modified_at = ?4,
             missing = 0, kind = ?5, episode_id = COALESCE(?6, episode_id), app_id = COALESCE(?7, app_id),
             updated_at = ?8 WHERE id = ?9",
            params![
                name,
                ext,
                meta.len() as i64,
                modified,
                kind,
                episode_id,
                app_id,
                ts,
                id
            ],
        )
        .map_err(|e| e.to_string())?;
        return Ok(id);
    }

    let id = new_id();
    conn.execute(
        "INSERT INTO files (id, episode_id, kind, name, path, ext, size_bytes, modified_at,
         hash, app_id, missing, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, 0, ?10, ?11)",
        params![
            id,
            episode_id,
            kind,
            name,
            path_str,
            ext,
            meta.len() as i64,
            modified,
            app_id,
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not register the file: {e}"))?;
    Ok(id)
}

#[tauri::command]
pub fn list_files(
    state: State<'_, AppState>,
    episode_id: Option<String>,
    kind: Option<String>,
    app_id: Option<String>,
    search: Option<String>,
    include_missing: Option<bool>,
) -> Result<Vec<FileEntry>, String> {
    let conn = state.conn()?;
    let mut sql = format!("{FILE_SELECT} WHERE 1 = 1");
    let mut args: Vec<String> = Vec::new();
    if let Some(e) = episode_id {
        sql.push_str(" AND f.episode_id = ?");
        args.push(e);
    }
    if let Some(k) = kind {
        sql.push_str(" AND f.kind = ?");
        args.push(k);
    }
    if let Some(a) = app_id {
        sql.push_str(" AND f.app_id = ?");
        args.push(a);
    }
    if let Some(s) = search {
        sql.push_str(" AND (LOWER(f.name) LIKE ? OR LOWER(f.path) LIKE ?)");
        let like = format!("%{}%", s.to_lowercase());
        args.push(like.clone());
        args.push(like);
    }
    if !include_missing.unwrap_or(true) {
        sql.push_str(" AND f.missing = 0");
    }
    sql.push_str(" ORDER BY COALESCE(f.modified_at, f.created_at) DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), map_file)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn import_file(
    state: State<'_, AppState>,
    episode_id: String,
    source_path: String,
    kind: Option<String>,
) -> Result<FileEntry, String> {
    validate_path_string(&source_path)?;
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("The file no longer exists: {source_path}"));
    }
    if !source.is_file() {
        return Err("Only files can be imported. Use Import folder for directories.".into());
    }
    let root = state.root();
    let conn = state.conn()?;
    let folder: String = conn
        .query_row(
            "SELECT folder FROM episodes WHERE id = ?1",
            params![episode_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Episode not found: {e}"))?;
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "import".into());
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let resolved_kind = kind
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| infer_kind(&ext).to_string());
    let sub = match resolved_kind.as_str() {
        "audio" | "narration" => "Narration",
        "video" | "image" => "Media",
        "project" => "Editing",
        "subtitle" => "Subtitles",
        "document" => "Documents",
        _ => "Media",
    };
    let target_dir = root.join("Projects").join(&folder).join(sub);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    let dest = unique_path(&target_dir, &name);
    fs::copy(&source, &dest).map_err(|e| format!("Could not copy the file: {e}"))?;
    let id = register_path(&conn, &root, Some(&episode_id), &dest, Some(&resolved_kind), None)?;
    db::log_activity(
        &conn,
        "file",
        Some("file"),
        Some(&id),
        &format!("Imported {} into {sub}", name),
    );
    let _ = crate::commands::content::write_manifest(&conn, &root, &episode_id);
    fetch_file(&conn, &id)
}

#[tauri::command]
pub fn register_file(
    state: State<'_, AppState>,
    episode_id: Option<String>,
    path: String,
    kind: Option<String>,
) -> Result<FileEntry, String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("That file does not exist".into());
    }
    let conn = state.conn()?;
    let id = register_path(&conn, &root, episode_id.as_deref(), &p, kind.as_deref(), None)?;
    fetch_file(&conn, &id)
}

#[tauri::command]
pub fn create_text_file(
    state: State<'_, AppState>,
    episode_id: String,
    name: String,
    content: Option<String>,
) -> Result<FileEntry, String> {
    valid_child_name(&name)?;
    let root = state.root();
    let conn = state.conn()?;
    let folder: String = conn
        .query_row(
            "SELECT folder FROM episodes WHERE id = ?1",
            params![episode_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Episode not found: {e}"))?;
    let dir = root.join("Projects").join(&folder).join("Documents");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = unique_path(&dir, &name);
    let body = content.unwrap_or_default();
    fs::write(&dest, body).map_err(|e| format!("Could not write {}: {e}", dest.display()))?;
    let id = register_path(&conn, &root, Some(&episode_id), &dest, Some("document"), None)?;
    db::log_activity(
        &conn,
        "file",
        Some("file"),
        Some(&id),
        &format!("Created document {}", dest.display()),
    );
    fetch_file(&conn, &id)
}

#[tauri::command]
pub fn create_folder(
    state: State<'_, AppState>,
    parent_path: String,
    name: String,
) -> Result<String, String> {
    validate_path_string(&parent_path)?;
    valid_child_name(&name)?;
    let root = state.root();
    let parent = PathBuf::from(&parent_path);
    ensure_inside(&root, &parent)?;
    if !parent.is_dir() {
        return Err("The parent folder does not exist".into());
    }
    let dest = parent.join(&name);
    if dest.exists() {
        return Err("A file or folder with that name already exists".into());
    }
    fs::create_dir(&dest).map_err(|e| format!("Could not create the folder: {e}"))?;
    if let Ok(conn) = state.conn() {
        db::log_activity(
            &conn,
            "file",
            Some("folder"),
            None,
            &format!("Created folder {}", dest.display()),
        );
    }
    Ok(dest.display().to_string())
}

#[tauri::command]
pub fn rename_entry(state: State<'_, AppState>, path: String, new_name: String) -> Result<String, String> {
    validate_path_string(&path)?;
    valid_child_name(&new_name)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    ensure_inside(&root, &p)?;
    if p == root {
        return Err("The library root cannot be renamed here".into());
    }
    if p == root.join("Projects") || p == root.join("Backups") {
        return Err("Core library folders cannot be renamed".into());
    }
    if !p.exists() {
        return Err("The entry no longer exists".into());
    }
    let parent = p.parent().ok_or("That path has no parent folder")?;
    let dest = parent.join(&new_name);
    if dest.exists() {
        return Err("A file or folder with that name already exists".into());
    }
    fs::rename(&p, &dest).map_err(|e| format!("Could not rename: {e}"))?;

    let conn = state.conn()?;
    let old_str = p.display().to_string();
    let new_str = dest.display().to_string();
    conn.execute(
        "UPDATE files SET path = ?1, name = ?2, ext = ?3, updated_at = ?4 WHERE path = ?5",
        params![
            new_str,
            new_name,
            dest.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase(),
            now(),
            old_str
        ],
    )
    .ok();
    // Keep nested registrations valid when a folder was renamed.
    let prefix = format!("{}{}", old_str, std::path::MAIN_SEPARATOR);
    let new_prefix = format!("{}{}", new_str, std::path::MAIN_SEPARATOR);
    conn.execute(
        "UPDATE files SET path = ?1 || substr(path, ?2) WHERE path LIKE ?3",
        params![
            new_prefix,
            old_str.len() + 1,
            format!("{}%", prefix.replace('%', "\\%"))
        ],
    )
    .ok();
    conn.execute(
        "UPDATE narrations SET file_path = ?1 || substr(file_path, ?2) WHERE file_path LIKE ?3",
        params![
            new_prefix,
            old_str.len() + 1,
            format!("{}%", prefix.replace('%', "\\%"))
        ],
    )
    .ok();

    db::log_activity(
        &conn,
        "file",
        Some("file"),
        None,
        &format!("Renamed {} to {new_name}", p.display()),
    );
    Ok(new_str)
}

fn copy_dir(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dest.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn move_entry(
    state: State<'_, AppState>,
    path: String,
    destination_dir: String,
) -> Result<String, String> {
    validate_path_string(&path)?;
    validate_path_string(&destination_dir)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    let dest_dir = PathBuf::from(&destination_dir);
    ensure_inside(&root, &p)?;
    ensure_inside(&root, &dest_dir)?;
    if !p.exists() {
        return Err("The entry no longer exists".into());
    }
    if !dest_dir.is_dir() {
        return Err("The destination is not a folder".into());
    }
    if p.is_dir() && (dest_dir == p || dest_dir.starts_with(&p)) {
        return Err("A folder cannot be moved inside itself".into());
    }
    let name = p.file_name().ok_or("Invalid path")?;
    let dest = dest_dir.join(name);
    if dest.exists() {
        return Err("The destination already has an entry with that name".into());
    }
    fs::rename(&p, &dest).map_err(|e| format!("Could not move: {e}"))?;
    let conn = state.conn()?;
    let old_str = p.display().to_string();
    let new_str = dest.display().to_string();
    conn.execute(
        "UPDATE files SET path = ?1, updated_at = ?2 WHERE path = ?3",
        params![new_str, now(), old_str],
    )
    .ok();
    let prefix = format!("{}{}", old_str, std::path::MAIN_SEPARATOR);
    let new_prefix = format!("{}{}", new_str, std::path::MAIN_SEPARATOR);
    conn.execute(
        "UPDATE files SET path = ?1 || substr(path, ?2) WHERE path LIKE ?3",
        params![
            new_prefix,
            old_str.len() + 1,
            format!("{}%", prefix.replace('%', "\\%"))
        ],
    )
    .ok();
    db::log_activity(
        &conn,
        "file",
        Some("file"),
        None,
        &format!("Moved {} to {}", p.display(), dest_dir.display()),
    );
    Ok(new_str)
}

#[tauri::command]
pub fn copy_entry(
    state: State<'_, AppState>,
    path: String,
    destination_dir: String,
) -> Result<String, String> {
    validate_path_string(&path)?;
    validate_path_string(&destination_dir)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    let dest_dir = PathBuf::from(&destination_dir);
    ensure_inside(&root, &p)?;
    ensure_inside(&root, &dest_dir)?;
    if !p.exists() {
        return Err("The entry no longer exists".into());
    }
    if !dest_dir.is_dir() {
        return Err("The destination is not a folder".into());
    }
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("Invalid path")?;
    let dest = unique_path(&dest_dir, &name);
    if p.is_dir() {
        copy_dir(&p, &dest)?;
    } else {
        fs::copy(&p, &dest).map_err(|e| format!("Could not copy: {e}"))?;
    }
    let conn = state.conn()?;
    if !dest.is_dir() {
        let _ = register_path(&conn, &root, None, &dest, None, None);
    }
    db::log_activity(
        &conn,
        "file",
        Some("file"),
        None,
        &format!("Copied {} to {}", p.display(), dest_dir.display()),
    );
    Ok(dest.display().to_string())
}

#[tauri::command]
pub fn delete_entry(state: State<'_, AppState>, path: String) -> Result<(), String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    ensure_inside(&root, &p)?;
    if p == root || p == root.join("Projects") || p == root.join("Backups") {
        return Err("Core library folders cannot be deleted".into());
    }
    if !p.exists() {
        return Err("The entry no longer exists".into());
    }
    let trash = root.join("Backups").join("Deleted");
    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "entry".into());
    let dest = trash.join(format!("{stamp}-{name}"));
    fs::rename(&p, &dest).map_err(|e| format!("Could not move the entry to the recycle folder: {e}"))?;

    let conn = state.conn()?;
    let old_str = p.display().to_string();
    conn.execute("DELETE FROM files WHERE path = ?1", params![old_str])
        .ok();
    conn.execute(
        "DELETE FROM files WHERE path LIKE ?1",
        params![format!("{}{}%", old_str, std::path::MAIN_SEPARATOR)],
    )
    .ok();
    db::log_activity(
        &conn,
        "file",
        Some("file"),
        None,
        &format!("Deleted {} (moved to Backups/Deleted)", p.display()),
    );
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathMeta {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: i64,
    pub modified_at: Option<String>,
    pub created_at: Option<String>,
    pub ext: String,
    pub registered_id: Option<String>,
    pub episode_id: Option<String>,
    pub exists: bool,
}

#[tauri::command]
pub fn path_metadata(state: State<'_, AppState>, path: String) -> Result<PathMeta, String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    ensure_inside(&root, &p)?;
    let exists = p.exists();
    let meta = if exists {
        fs::metadata(&p).ok()
    } else {
        None
    };
    let conn = state.conn()?;
    let registered: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT id, episode_id FROM files WHERE path = ?1",
            params![path],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    Ok(PathMeta {
        path: path.clone(),
        name: p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone()),
        is_dir: p.is_dir(),
        size_bytes: meta.as_ref().map(|m| m.len() as i64).unwrap_or(0),
        modified_at: meta.as_ref().and_then(modified_iso),
        created_at: meta.as_ref().and_then(|m| m.created().ok()).map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        }),
        ext: p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase(),
        registered_id: registered.as_ref().map(|r| r.0.clone()),
        episode_id: registered.and_then(|r| r.1),
        exists,
    })
}

fn build_tree(
    dir: &Path,
    depth: usize,
    registered: &HashMap<String, String>,
) -> Vec<TreeEntry> {
    let mut out = Vec::new();
    let Ok(read) = fs::read_dir(dir) else {
        return out;
    };
    let mut entries: Vec<_> = read.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| {
        let is_dir = e.path().is_dir();
        (!is_dir, e.file_name().to_string_lossy().to_lowercase())
    });
    for entry in entries {
        let path = entry.path();
        let is_dir = path.is_dir();
        let meta = entry.metadata().ok();
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_dir && name == "episode.json" {
            continue;
        }
        let children = if is_dir && depth < 3 {
            build_tree(&path, depth + 1, registered)
        } else {
            Vec::new()
        };
        out.push(TreeEntry {
            name,
            path: path.display().to_string(),
            is_dir,
            size_bytes: meta.as_ref().map(|m| m.len() as i64).unwrap_or(0),
            modified_at: meta.as_ref().and_then(modified_iso),
            ext: path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase(),
            registered_id: registered.get(&path.display().to_string()).cloned(),
            children,
        });
    }
    out
}

#[tauri::command]
pub fn episode_tree(state: State<'_, AppState>, episode_id: String) -> Result<EpisodeTree, String> {
    let root = state.root();
    let conn = state.conn()?;
    let folder: String = conn
        .query_row(
            "SELECT folder FROM episodes WHERE id = ?1",
            params![episode_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Episode not found: {e}"))?;
    let base = root.join("Projects").join(&folder);
    if !base.exists() {
        return Err(format!("Episode folder is missing: {}", base.display()));
    }
    let mut registered = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT path, id FROM files WHERE episode_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![episode_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            if let Ok((path, id)) = row {
                registered.insert(path, id);
            }
        }
    }
    Ok(EpisodeTree {
        path: base.display().to_string(),
        entries: build_tree(&base, 0, &registered),
    })
}

#[tauri::command]
pub fn file_versions(state: State<'_, AppState>, file_id: String) -> Result<Vec<FileVersion>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, file_id, path, size_bytes, modified_at, hash, note, created_at
             FROM file_versions WHERE file_id = ?1 ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![file_id], |r| {
            Ok(FileVersion {
                id: r.get(0)?,
                file_id: r.get(1)?,
                path: r.get(2)?,
                size_bytes: r.get(3)?,
                modified_at: r.get(4)?,
                hash: r.get(5)?,
                note: r.get(6)?,
                created_at: r.get(7)?,
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
pub fn relink_file(
    state: State<'_, AppState>,
    file_id: String,
    new_path: String,
) -> Result<FileEntry, String> {
    validate_path_string(&new_path)?;
    let root = state.root();
    let p = PathBuf::from(&new_path);
    ensure_inside(&root, &p)?;
    if !p.exists() {
        return Err("That file does not exist".into());
    }
    let meta = fs::metadata(&p).map_err(|e| e.to_string())?;
    let conn = state.conn()?;
    let old: String = conn
        .query_row("SELECT path FROM files WHERE id = ?1", params![file_id], |r| r.get(0))
        .map_err(|e| format!("File record not found: {e}"))?;
    conn.execute(
        "UPDATE files SET path = ?1, name = ?2, ext = ?3, size_bytes = ?4, modified_at = ?5,
         missing = 0, updated_at = ?6 WHERE id = ?7",
        params![
            p.display().to_string(),
            p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase(),
            meta.len() as i64,
            modified_iso(&meta),
            now(),
            file_id
        ],
    )
    .map_err(|e| format!("Could not relink the file: {e}"))?;
    record_version(
        &conn,
        &file_id,
        &p.display().to_string(),
        meta.len() as i64,
        modified_iso(&meta).as_ref(),
        &format!("Relinked from {old}"),
    );
    db::log_activity(
        &conn,
        "file",
        Some("file"),
        Some(&file_id),
        &format!("Relinked a file to {}", p.display()),
    );
    fetch_file(&conn, &file_id)
}

/// Compares registered files with the disk, registers newly discovered files
/// inside episode folders, and records external modifications.
#[tauri::command]
pub fn scan_external_changes(state: State<'_, AppState>) -> Result<ChangeReport, String> {
    let root = state.root();
    let conn = state.conn()?;
    let ts = now();
    let mut report = ChangeReport {
        checked: 0,
        new: Vec::new(),
        modified: Vec::new(),
        missing: Vec::new(),
        checked_at: ts.clone(),
    };

    let rows: Vec<(String, String, i64, Option<String>, i64)> = {
        let mut stmt = conn
            .prepare("SELECT id, path, size_bytes, modified_at, missing FROM files")
            .map_err(|e| e.to_string())?;
        let iter = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in iter {
            out.push(row.map_err(|e| e.to_string())?);
        }
        out
    };

    for (id, path, size, modified, missing) in rows {
        report.checked += 1;
        let p = PathBuf::from(&path);
        if !p.exists() {
            if missing == 0 {
                conn.execute(
                    "UPDATE files SET missing = 1, updated_at = ?1 WHERE id = ?2",
                    params![ts, id],
                )
                .ok();
                if let Ok(entry) = fetch_file(&conn, &id) {
                    report.missing.push(entry);
                }
            }
            continue;
        }
        let Ok(meta) = fs::metadata(&p) else { continue };
        let now_modified = modified_iso(&meta);
        let changed = meta.len() as i64 != size || now_modified != modified;
        if changed {
            conn.execute(
                "UPDATE files SET size_bytes = ?1, modified_at = ?2, missing = 0, updated_at = ?3
                 WHERE id = ?4",
                params![meta.len() as i64, now_modified, ts, id],
            )
            .ok();
            record_version(
                &conn,
                &id,
                &path,
                meta.len() as i64,
                now_modified.as_ref(),
                "Detected an external modification",
            );
            if let Ok(entry) = fetch_file(&conn, &id) {
                report.modified.push(entry);
            }
        } else if missing == 1 {
            conn.execute(
                "UPDATE files SET missing = 0, updated_at = ?1 WHERE id = ?2",
                params![ts, id],
            )
            .ok();
        }
    }

    // Discover files placed in episode folders by other applications.
    let episodes: Vec<(String, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, folder FROM episodes WHERE archived = 0")
            .map_err(|e| e.to_string())?;
        let iter = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in iter {
            out.push(row.map_err(|e| e.to_string())?);
        }
        out
    };

    fn walk(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        if depth > 3 {
            return;
        }
        let Ok(read) = fs::read_dir(dir) else { return };
        for entry in read.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out, depth + 1);
            } else if p.file_name().map(|n| n != "episode.json").unwrap_or(true) {
                out.push(p);
            }
        }
    }

    for (episode_id, folder) in episodes {
        let base = root.join("Projects").join(&folder);
        let mut files = Vec::new();
        walk(&base, &mut files, 0);
        for path in files {
            let path_str = path.display().to_string();
            let known: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM files WHERE path = ?1",
                    params![path_str],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if known > 0 {
                continue;
            }
            if let Ok(id) = register_path(&conn, &root, Some(&episode_id), &path, None, None) {
                if let Ok(entry) = fetch_file(&conn, &id) {
                    report.new.push(entry);
                }
            }
        }
    }

    if !report.new.is_empty() || !report.modified.is_empty() || !report.missing.is_empty() {
        db::log_activity(
            &conn,
            "file",
            Some("scan"),
            None,
            &format!(
                "File monitor: {} new, {} modified, {} missing",
                report.new.len(),
                report.modified.len(),
                report.missing.len()
            ),
        );
    }
    Ok(report)
}

/// Returns a data URL for previews and audio playback. Only files inside the
/// library, of a previewable type, and under the size cap are served.
#[tauri::command]
pub fn read_file_data(state: State<'_, AppState>, path: String) -> Result<String, String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    ensure_inside(&root, &p)?;
    if !p.is_file() {
        return Err("That is not a file".into());
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = match ext.as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "ogg" | "opus" => "audio/ogg",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "txt" | "md" | "srt" | "vtt" | "json" | "csv" => "text/plain",
        _ => return Err(format!("Preview is not supported for .{ext} files")),
    };
    let meta = fs::metadata(&p).map_err(|e| e.to_string())?;
    if meta.len() > 80 * 1024 * 1024 {
        return Err("That file is too large to preview (limit 80 MB)".into());
    }
    let bytes = fs::read(&p).map_err(|e| format!("Could not read the file: {e}"))?;
    Ok(format!("data:{};base64,{}", mime, crate::b64::encode(&bytes)))
}

pub fn now_stamp() -> String {
    let t: SystemTime = SystemTime::now();
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Registers a path on behalf of another command module.
pub fn register_path_public(
    conn: &Connection,
    root: &Path,
    episode_id: Option<&str>,
    path: &Path,
) -> Result<String, String> {
    register_path(conn, root, episode_id, path, None, None)
}
