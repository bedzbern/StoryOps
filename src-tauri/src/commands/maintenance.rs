use rusqlite::{backup::Backup, params, Connection};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::State;

use crate::db;
use crate::models::{BackupRow, IntegrityReport};
use crate::state::AppState;
use crate::util::{now, validate_path_string};

fn backup_dir(conn: &Connection, root: &Path) -> PathBuf {
    db::get_setting(conn, "backupDirectory")
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("Backups"))
}

fn create_backup(
    conn: &Connection,
    root: &Path,
    kind: &str,
    note: &str,
) -> Result<BackupRow, String> {
    let dir = backup_dir(conn, root);
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let mut target = dir.join(format!("storyops-{stamp}.db"));
    let mut i = 2;
    while target.exists() {
        target = dir.join(format!("storyops-{stamp}-{i}.db"));
        i += 1;
    }
    let target_str = target.display().to_string();
    conn.execute("VACUUM INTO ?1", params![target_str])
        .map_err(|e| format!("Could not write the backup: {e}"))?;
    let size = fs::metadata(&target).map(|m| m.len() as i64).unwrap_or(0);
    let id = db::new_backup_id();
    let ts = now();
    conn.execute(
        "INSERT INTO backups (id, path, size_bytes, kind, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, target_str, size, kind, note, ts],
    )
    .map_err(|e| format!("Could not record the backup: {e}"))?;
    db::set_setting(conn, "lastBackupAt", &ts).ok();
    db::log_activity(
        conn,
        "backup",
        Some("backup"),
        Some(&id),
        &format!("Created {kind} backup ({size} bytes)"),
    );
    Ok(BackupRow {
        id,
        path: target_str,
        size_bytes: size,
        kind: kind.to_string(),
        note: note.to_string(),
        created_at: ts,
        exists: true,
    })
}

#[tauri::command]
pub fn backup_database(
    state: State<'_, AppState>,
    note: Option<String>,
) -> Result<BackupRow, String> {
    let conn = state.conn()?;
    let root = state.root();
    create_backup(
        &conn,
        &root,
        "manual",
        note.as_deref().unwrap_or("Manual backup from Settings"),
    )
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupRow>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, path, size_bytes, kind, note, created_at FROM backups
             ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let path: String = r.get(1)?;
            Ok(BackupRow {
                id: r.get(0)?,
                exists: PathBuf::from(&path).exists(),
                path,
                size_bytes: r.get(2)?,
                kind: r.get(3)?,
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

/// Restores the live database from a backup. A safety copy of the current
/// database is always taken first, and the file is never overwritten in place.
#[tauri::command]
pub fn restore_backup(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let root = state.root();
    let mut guard = state.conn()?;
    let path: String = guard
        .query_row("SELECT path FROM backups WHERE id = ?1", params![id], |r| {
            r.get(0)
        })
        .map_err(|e| format!("Backup not found: {e}"))?;
    validate_path_string(&path)?;
    let source = PathBuf::from(&path);
    if !source.is_file() {
        return Err(format!("That backup file is missing: {path}"));
    }
    if !crate::util::is_within(&root, &source) {
        return Err("Only backups inside the StoryOps library can be restored".into());
    }

    let safety = create_backup(
        &guard,
        &root,
        "pre-restore",
        "Automatic safety copy taken before restoring a backup",
    )?;

    let src = Connection::open(&source).map_err(|e| format!("Could not open the backup: {e}"))?;
    {
        let backup = Backup::new(&src, &mut guard)
            .map_err(|e| format!("Could not start the restore: {e}"))?;
        backup
            .run_to_completion(200, Duration::from_millis(5), None)
            .map_err(|e| format!("Restore failed: {e}"))?;
    }
    guard.pragma_update(None, "foreign_keys", "ON").ok();
    db::migrate(&guard)?;
    db::log_activity(
        &guard,
        "backup",
        Some("backup"),
        Some(&id),
        &format!("Restored the database from {path}"),
    );
    Ok(format!(
        "Database restored. A safety copy of the previous state is at {}",
        safety.path
    ))
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_files(&p, out, depth + 1);
        } else if p.file_name().map(|n| n != "episode.json").unwrap_or(true) {
            out.push(p);
        }
    }
}

#[tauri::command]
pub fn database_integrity(state: State<'_, AppState>) -> Result<IntegrityReport, String> {
    let conn = state.conn()?;
    let root = state.root();

    let database_message: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap_or_else(|e| format!("integrity_check failed: {e}"));
    let database_ok = database_message.eq_ignore_ascii_case("ok");

    let episodes: Vec<(String, String, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, code, folder FROM episodes")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut missing_folders = Vec::new();
    let mut manifest_issues = Vec::new();
    for (id, code, folder) in &episodes {
        let base = root.join("Projects").join(folder);
        if !base.is_dir() {
            missing_folders.push(format!("{code}: {}", base.display()));
            continue;
        }
        let manifest = base.join("episode.json");
        if !manifest.is_file() {
            manifest_issues.push(format!("{code}: episode.json is missing"));
            continue;
        }
        match fs::read_to_string(&manifest)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        {
            Some(value) => {
                let recorded = value
                    .get("episodeId")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if recorded != id {
                    manifest_issues.push(format!("{code}: episode.json belongs to another episode"));
                }
            }
            None => manifest_issues.push(format!("{code}: episode.json is not valid JSON")),
        }
    }

    let registered: HashSet<String> = {
        let mut stmt = conn
            .prepare("SELECT path FROM files")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut missing_files = Vec::new();
    for path in &registered {
        if !PathBuf::from(path).is_file() {
            missing_files.push(path.clone());
        }
    }

    let mut on_disk = Vec::new();
    walk_files(&root.join("Projects"), &mut on_disk, 0);
    let orphan_files = on_disk
        .iter()
        .filter(|p| !registered.contains(&p.display().to_string()))
        .count() as i64;

    Ok(IntegrityReport {
        database_ok,
        database_message,
        episodes_checked: episodes.len() as i64,
        missing_folders,
        missing_files,
        orphan_files,
        manifest_issues,
        checked_at: now(),
    })
}

#[tauri::command]
pub fn repair_manifests(state: State<'_, AppState>) -> Result<i64, String> {
    let conn = state.conn()?;
    let root = state.root();
    let ids: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM episodes")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let mut repaired = 0;
    for id in ids {
        if crate::commands::content::write_manifest(&conn, &root, &id).is_ok() {
            repaired += 1;
        }
    }
    db::log_activity(
        &conn,
        "maintenance",
        Some("manifest"),
        None,
        &format!("Rebuilt {repaired} episode manifests"),
    );
    Ok(repaired)
}

#[tauri::command]
pub fn log_tail(state: State<'_, AppState>, lines: Option<usize>) -> Result<Vec<String>, String> {
    let path = state.log_path();
    let text = fs::read_to_string(&path).unwrap_or_default();
    let take = lines.unwrap_or(200).clamp(1, 2000);
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(take);
    Ok(all[start..].iter().map(|s| s.to_string()).collect())
}
