use rusqlite::params;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

use crate::db;
use crate::models::Narration;
use crate::state::AppState;
use crate::util::{new_id, now, slugify};

const AUDIO_EXTS: [&str; 8] = ["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "wma"];

fn map_narration(row: &rusqlite::Row<'_>) -> rusqlite::Result<Narration> {
    Ok(Narration {
        id: row.get(0)?,
        episode_id: row.get(1)?,
        episode_title: row.get(2)?,
        title: row.get(3)?,
        file_path: row.get(4)?,
        format: row.get(5)?,
        duration_sec: row.get(6)?,
        sample_rate: row.get(7)?,
        channels: row.get(8)?,
        bitrate: row.get(9)?,
        size_bytes: row.get(10)?,
        status: row.get(11)?,
        notes: row.get(12)?,
        version: row.get(13)?,
        is_current: row.get::<_, i64>(14)? != 0,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

const NARRATION_SELECT: &str = "SELECT n.id, n.episode_id, e.title, n.title, n.file_path, n.format,
    n.duration_sec, n.sample_rate, n.channels, n.bitrate, n.size_bytes, n.status, n.notes,
    n.version, n.is_current, n.created_at, n.updated_at
    FROM narrations n LEFT JOIN episodes e ON e.id = n.episode_id";

fn fetch(conn: &rusqlite::Connection, id: &str) -> Result<Narration, String> {
    conn.query_row(
        &format!("{NARRATION_SELECT} WHERE n.id = ?1"),
        params![id],
        map_narration,
    )
    .map_err(|e| format!("Narration not found: {e}"))
}

/// Reads a PCM WAV header for real duration/format metadata.
fn probe_wav(path: &Path) -> Option<(f64, i64, i64, i64)> {
    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 12];
    file.read_exact(&mut header).ok()?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return None;
    }
    let mut byte_rate = 0u32;
    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits = 0u16;
    let mut data_len = 0u32;

    let mut buf = [0u8; 8];
    loop {
        if file.read_exact(&mut buf).is_err() {
            break;
        }
        let id = &buf[0..4];
        let size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        if id == b"fmt " {
            let mut fmt = vec![0u8; size.min(64) as usize];
            file.read_exact(&mut fmt).ok()?;
            if fmt.len() >= 16 {
                channels = u16::from_le_bytes([fmt[2], fmt[3]]);
                sample_rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
                byte_rate = u32::from_le_bytes([fmt[8], fmt[9], fmt[10], fmt[11]]);
                bits = u16::from_le_bytes([fmt[14], fmt[15]]);
            }
            if size as usize > fmt.len() {
                let skip = size as usize - fmt.len();
                let mut sink = vec![0u8; skip.min(4096)];
                file.read_exact(&mut sink).ok()?;
            }
        } else if id == b"data" {
            data_len = size;
            break;
        } else {
            let skip = size as i64;
            if skip > 0 {
                use std::io::Seek;
                file.seek(std::io::SeekFrom::Current(skip)).ok()?;
            }
        }
    }
    if byte_rate == 0 {
        return None;
    }
    let duration = data_len as f64 / byte_rate as f64;
    let bitrate = if bits > 0 {
        (sample_rate as i64 * channels as i64 * bits as i64) / 1000
    } else {
        (byte_rate as i64 * 8) / 1000
    };
    Some((duration, sample_rate as i64, channels as i64, bitrate))
}

fn unique_destination(dir: &Path, file_name: &str) -> PathBuf {
    let mut candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".into());
    let ext = Path::new(file_name)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut i = 2;
    loop {
        let name = if ext.is_empty() {
            format!("{stem} ({i})")
        } else {
            format!("{stem} ({i}).{ext}")
        };
        candidate = dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
        i += 1;
    }
}

#[tauri::command]
pub fn list_narrations(state: State<'_, AppState>) -> Result<Vec<Narration>, String> {
    let conn = state.conn()?;
    let sql = format!("{NARRATION_SELECT} ORDER BY n.created_at DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_narration)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn import_narration(
    state: State<'_, AppState>,
    episode_id: String,
    source_path: String,
    title: Option<String>,
) -> Result<Narration, String> {
    crate::util::validate_path_string(&source_path)?;
    let source = PathBuf::from(&source_path);
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !AUDIO_EXTS.contains(&ext.as_str()) {
        return Err(format!(
            "Unsupported audio format. Supported: {}",
            AUDIO_EXTS.join(", ")
        ));
    }
    let root = state.root();
    let conn = state.conn()?;
    let (folder, code): (String, String) = conn
        .query_row(
            "SELECT folder, code FROM episodes WHERE id = ?1",
            params![episode_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("Episode not found: {e}"))?;

    let target_dir = root.join("Projects").join(&folder).join("Narration");
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Could not create {}: {e}", target_dir.display()))?;

    let base = title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| {
            source
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Narration".into())
        });
    let file_name = format!("{}-{}.{}", slugify(&base), code.to_lowercase(), ext);
    let dest = unique_destination(&target_dir, &file_name);
    fs::copy(&source, &dest).map_err(|e| format!("Could not copy the audio file: {e}"))?;

    let meta = fs::metadata(&dest).map_err(|e| e.to_string())?;
    let probe = if ext == "wav" {
        probe_wav(&dest)
    } else {
        None
    };
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO narrations (id, episode_id, title, file_path, format, duration_sec,
         sample_rate, channels, bitrate, size_bytes, status, notes, version, is_current,
         created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'draft', '', 1, 1, ?11, ?12)",
        params![
            id,
            episode_id,
            base,
            dest.display().to_string(),
            ext,
            probe.map(|p| p.0).unwrap_or(0.0),
            probe.map(|p| p.1),
            probe.map(|p| p.2),
            probe.map(|p| p.3),
            meta.len() as i64,
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not register the narration: {e}"))?;

    conn.execute(
        "INSERT INTO files (id, episode_id, kind, name, path, ext, size_bytes, modified_at,
         hash, app_id, missing, created_at, updated_at)
         VALUES (?1, ?2, 'narration', ?3, ?4, ?5, ?6, ?7, NULL, NULL, 0, ?8, ?9)
         ON CONFLICT(path) DO UPDATE SET episode_id = excluded.episode_id, size_bytes = excluded.size_bytes,
         modified_at = excluded.modified_at, updated_at = excluded.updated_at",
        params![
            new_id(),
            episode_id,
            dest.file_name().unwrap_or_default().to_string_lossy(),
            dest.display().to_string(),
            ext,
            meta.len() as i64,
            ts,
            ts,
            ts
        ],
    )
    .map_err(|e| e.to_string())?;

    db::log_activity(
        &conn,
        "narration",
        Some("narration"),
        Some(&id),
        &format!("Imported narration — {base}"),
    );
    fetch(&conn, &id)
}

#[tauri::command]
pub fn update_narration(
    state: State<'_, AppState>,
    id: String,
    title: String,
    notes: String,
    status: String,
    duration_sec: Option<f64>,
) -> Result<Narration, String> {
    if !["draft", "recorded", "review", "approved", "rejected"].contains(&status.as_str()) {
        return Err("Unknown narration status".into());
    }
    let conn = state.conn()?;
    match duration_sec {
        Some(d) if d.is_finite() && d >= 0.0 => {
            conn.execute(
                "UPDATE narrations SET title = ?1, notes = ?2, status = ?3, duration_sec = ?4,
                 updated_at = ?5 WHERE id = ?6",
                params![title.trim(), notes, status, d, now(), id],
            )
            .map_err(|e| e.to_string())?;
        }
        _ => {
            conn.execute(
                "UPDATE narrations SET title = ?1, notes = ?2, status = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![title.trim(), notes, status, now(), id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    db::log_activity(
        &conn,
        "narration",
        Some("narration"),
        Some(&id),
        &format!("Updated narration — {}", title.trim()),
    );
    let narration = fetch(&conn, &id)?;
    let _ = crate::commands::content::write_manifest(&conn, &state.root(), &narration.episode_id.clone().unwrap_or_default());
    Ok(narration)
}

#[tauri::command]
pub fn replace_narration(
    state: State<'_, AppState>,
    id: String,
    source_path: String,
) -> Result<Narration, String> {
    crate::util::validate_path_string(&source_path)?;
    let source = PathBuf::from(&source_path);
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !AUDIO_EXTS.contains(&ext.as_str()) {
        return Err("Unsupported audio format".into());
    }
    let conn = state.conn()?;
    let old = fetch(&conn, &id)?;
    let Some(episode_id) = old.episode_id.clone() else {
        return Err("This narration is not linked to an episode".into());
    };
    let root = state.root();
    let folder: String = conn
        .query_row(
            "SELECT folder FROM episodes WHERE id = ?1",
            params![episode_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let target_dir = root.join("Projects").join(&folder).join("Narration");
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    let file_name = format!(
        "{}-v{}.{}",
        slugify(&old.title),
        old.version + 1,
        ext
    );
    let dest = unique_destination(&target_dir, &file_name);
    fs::copy(&source, &dest).map_err(|e| format!("Could not copy the audio file: {e}"))?;
    let meta = fs::metadata(&dest).map_err(|e| e.to_string())?;
    let probe = if ext == "wav" { probe_wav(&dest) } else { None };

    conn.execute(
        "UPDATE narrations SET is_current = 0, updated_at = ?1 WHERE id = ?2",
        params![now(), id],
    )
    .map_err(|e| e.to_string())?;

    let new_id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO narrations (id, episode_id, title, file_path, format, duration_sec,
         sample_rate, channels, bitrate, size_bytes, status, notes, version, is_current,
         created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'draft', ?11, ?12, 1, ?13, ?14)",
        params![
            new_id,
            episode_id,
            old.title,
            dest.display().to_string(),
            ext,
            probe.map(|p| p.0).unwrap_or(0.0),
            probe.map(|p| p.1),
            probe.map(|p| p.2),
            probe.map(|p| p.3),
            meta.len() as i64,
            old.notes,
            old.version + 1,
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not save the new narration version: {e}"))?;
    db::log_activity(
        &conn,
        "narration",
        Some("narration"),
        Some(&new_id),
        &format!("Replaced narration \"{}\" with version {}", old.title, old.version + 1),
    );
    fetch(&conn, &new_id)
}

#[tauri::command]
pub fn delete_narration(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    let narration = fetch(&conn, &id)?;
    let root = state.root();
    let path = PathBuf::from(&narration.file_path);
    if path.exists() {
        let trash = root.join("Backups").join("Deleted");
        fs::create_dir_all(&trash).ok();
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "audio".into());
        let _ = fs::rename(&path, trash.join(format!("{stamp}-{name}")));
    }
    conn.execute("DELETE FROM narrations WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM files WHERE path = ?1", params![narration.file_path])
        .ok();
    db::log_activity(
        &conn,
        "narration",
        Some("narration"),
        Some(&id),
        &format!("Deleted narration — {}", narration.title),
    );
    Ok(())
}
