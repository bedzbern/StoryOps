use rusqlite::params;
use std::fs;
use std::path::PathBuf;
use tauri::State;

use crate::db;
use crate::models::*;
use crate::repo::{map_schedule, SCHEDULE_COLS, SCHEDULE_JOINS};
use crate::state::AppState;
use crate::util::{new_id, now, validate_path_string};

#[tauri::command]
pub fn list_platforms(state: State<'_, AppState>) -> Result<Vec<Platform>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare("SELECT id, key, name, enabled, config, created_at FROM platforms ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Platform {
                id: r.get(0)?,
                key: r.get(1)?,
                name: r.get(2)?,
                enabled: r.get::<_, i64>(3)? != 0,
                config: serde_json::from_str(&r.get::<_, String>(4)?)
                    .unwrap_or(serde_json::json!({})),
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
pub fn save_platform(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    enabled: bool,
) -> Result<Platform, String> {
    if name.trim().is_empty() {
        return Err("A platform needs a name".into());
    }
    let conn = state.conn()?;
    let platform_id = match id {
        Some(existing) => {
            conn.execute(
                "UPDATE platforms SET name = ?1, enabled = ?2 WHERE id = ?3",
                params![name.trim(), if enabled { 1 } else { 0 }, existing],
            )
            .map_err(|e| format!("Could not update the platform: {e}"))?;
            existing
        }
        None => {
            let new = new_id();
            let key = format!("custom_{}", new.split('-').next().unwrap_or("platform"));
            conn.execute(
                "INSERT INTO platforms (id, key, name, enabled, config, created_at)
                 VALUES (?1, ?2, ?3, ?4, '{}', ?5)",
                params![new, key, name.trim(), if enabled { 1 } else { 0 }, now()],
            )
            .map_err(|e| format!("Could not add the platform: {e}"))?;
            new
        }
    };
    db::log_activity(
        &conn,
        "platform",
        Some("platform"),
        Some(&platform_id),
        &format!("Saved platform — {}", name.trim()),
    );
    conn.query_row(
        "SELECT id, key, name, enabled, config, created_at FROM platforms WHERE id = ?1",
        params![platform_id],
        |r| {
            Ok(Platform {
                id: r.get(0)?,
                key: r.get(1)?,
                name: r.get(2)?,
                enabled: r.get::<_, i64>(3)? != 0,
                config: serde_json::from_str(&r.get::<_, String>(4)?)
                    .unwrap_or(serde_json::json!({})),
                created_at: r.get(5)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_schedules(
    state: State<'_, AppState>,
    episode_id: Option<String>,
) -> Result<Vec<Schedule>, String> {
    let conn = state.conn()?;
    let (sql, args): (String, Vec<String>) = match episode_id {
        Some(id) => (
            format!("SELECT {SCHEDULE_COLS} {SCHEDULE_JOINS} WHERE p.episode_id = ?1 ORDER BY COALESCE(p.scheduled_at, p.created_at) DESC"),
            vec![id],
        ),
        None => (
            format!("SELECT {SCHEDULE_COLS} {SCHEDULE_JOINS} ORDER BY COALESCE(p.scheduled_at, p.created_at) DESC"),
            Vec::new(),
        ),
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), map_schedule)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn save_schedule(state: State<'_, AppState>, input: ScheduleInput) -> Result<Schedule, String> {
    if input.title.trim().is_empty() {
        return Err("A publishing record needs a title".into());
    }
    let status = if input.status.is_empty() { "draft".into() } else { input.status.clone() };
    if !["draft", "scheduled", "published", "cancelled"].contains(&status.as_str()) {
        return Err("Unknown publishing status".into());
    }
    if status == "scheduled" && input.scheduled_at.is_none() {
        return Err("Choose a scheduled date before marking a post as scheduled".into());
    }
    let conn = state.conn()?;
    let ts = now();
    let checklist = crate::repo::to_json(&input.checklist);

    let id = match input.id {
        Some(existing) => {
            conn.execute(
                "UPDATE publishing SET episode_id = ?1, platform_id = ?2, title = ?3,
                 description = ?4, caption = ?5, hashtags = ?6, target_audience = ?7,
                 scheduled_at = ?8, status = ?9, export_id = ?10, checklist = ?11,
                 post_url = ?12, updated_at = ?13 WHERE id = ?14",
                params![
                    input.episode_id,
                    input.platform_id,
                    input.title.trim(),
                    input.description,
                    input.caption,
                    input.hashtags,
                    input.target_audience,
                    input.scheduled_at,
                    status,
                    input.export_id,
                    checklist,
                    input.post_url,
                    ts,
                    existing
                ],
            )
            .map_err(|e| format!("Could not save the publishing record: {e}"))?;
            existing
        }
        None => {
            let new = new_id();
            conn.execute(
                "INSERT INTO publishing (id, episode_id, platform_id, title, description, caption,
                 hashtags, target_audience, scheduled_at, status, export_id, checklist, post_url,
                 published_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, ?14, ?15)",
                params![
                    new,
                    input.episode_id,
                    input.platform_id,
                    input.title.trim(),
                    input.description,
                    input.caption,
                    input.hashtags,
                    input.target_audience,
                    input.scheduled_at,
                    status,
                    input.export_id,
                    checklist,
                    input.post_url,
                    ts,
                    ts
                ],
            )
            .map_err(|e| format!("Could not create the publishing record: {e}"))?;
            new
        }
    };
    db::log_activity(
        &conn,
        "publishing",
        Some("publishing"),
        Some(&id),
        &format!("Saved publishing record — {}", input.title.trim()),
    );
    fetch_schedule(&conn, &id)
}

fn fetch_schedule(conn: &rusqlite::Connection, id: &str) -> Result<Schedule, String> {
    conn.query_row(
        &format!("SELECT {SCHEDULE_COLS} {SCHEDULE_JOINS} WHERE p.id = ?1"),
        params![id],
        map_schedule,
    )
    .map_err(|e| format!("Publishing record not found: {e}"))
}

#[tauri::command]
pub fn delete_schedule(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    let title: String = conn
        .query_row("SELECT title FROM publishing WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| format!("Publishing record not found: {e}"))?;
    conn.execute("DELETE FROM publishing WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "publishing",
        Some("publishing"),
        Some(&id),
        &format!("Deleted publishing record — {title}"),
    );
    Ok(())
}

/// Marks a post as actually published. This is the only place the "published"
/// state is set, and it requires the external post URL or explicit confirmation.
#[tauri::command]
pub fn mark_published(
    state: State<'_, AppState>,
    id: String,
    post_url: Option<String>,
) -> Result<Schedule, String> {
    let conn = state.conn()?;
    let (episode_id, title): (String, String) = conn
        .query_row(
            "SELECT episode_id, title FROM publishing WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("Publishing record not found: {e}"))?;
    let ts = now();
    conn.execute(
        "UPDATE publishing SET status = 'published', published_at = ?1, post_url = COALESCE(?2, post_url),
         updated_at = ?3 WHERE id = ?4",
        params![ts, post_url, ts, id],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "publishing",
        Some("publishing"),
        Some(&id),
        &format!("Marked \"{title}\" as published"),
    );
    let _ = crate::commands::content::write_manifest(&conn, &state.root(), &episode_id);
    fetch_schedule(&conn, &id)
}

#[tauri::command]
pub fn list_exports(
    state: State<'_, AppState>,
    episode_id: Option<String>,
) -> Result<Vec<ExportRow>, String> {
    let conn = state.conn()?;
    let base = "SELECT x.id, x.episode_id, e.title, x.name, x.file_path, x.format, x.status,
        x.notes, x.created_at, x.updated_at FROM exports x
        JOIN episodes e ON e.id = x.episode_id";
    let (sql, args): (String, Vec<String>) = match episode_id {
        Some(id) => (
            format!("{base} WHERE x.episode_id = ?1 ORDER BY x.created_at DESC"),
            vec![id],
        ),
        None => (format!("{base} ORDER BY x.created_at DESC"), Vec::new()),
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(ExportRow {
                id: r.get(0)?,
                episode_id: r.get(1)?,
                episode_title: r.get(2)?,
                name: r.get(3)?,
                file_path: r.get(4)?,
                format: r.get(5)?,
                status: r.get(6)?,
                notes: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
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
pub fn register_export(
    state: State<'_, AppState>,
    episode_id: String,
    path: String,
    notes: Option<String>,
) -> Result<ExportRow, String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    if !crate::util::is_within(&root, &p) {
        return Err("Exports must be inside the StoryOps library".into());
    }
    if !p.is_file() {
        return Err("That export file does not exist".into());
    }
    let meta = fs::metadata(&p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Export".into());
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let conn = state.conn()?;
    let file_id = crate::commands::files::register_path_public(&conn, &root, Some(&episode_id), &p)?;
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO exports (id, episode_id, file_id, name, file_path, format, status, notes,
         created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, ?8, ?9)",
        params![
            id,
            episode_id,
            file_id,
            name,
            p.display().to_string(),
            ext,
            notes.unwrap_or_default(),
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not register the export: {e}"))?;
    let _ = meta;
    db::log_activity(
        &conn,
        "export",
        Some("export"),
        Some(&id),
        &format!("Registered export {name}"),
    );
    let _ = crate::commands::content::write_manifest(&conn, &root, &episode_id);
    conn.query_row(
        "SELECT x.id, x.episode_id, e.title, x.name, x.file_path, x.format, x.status, x.notes,
         x.created_at, x.updated_at FROM exports x JOIN episodes e ON e.id = x.episode_id
         WHERE x.id = ?1",
        params![id],
        |r| {
            Ok(ExportRow {
                id: r.get(0)?,
                episode_id: r.get(1)?,
                episode_title: r.get(2)?,
                name: r.get(3)?,
                file_path: r.get(4)?,
                format: r.get(5)?,
                status: r.get(6)?,
                notes: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_export_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<(), String> {
    if !["draft", "approved", "superseded"].contains(&status.as_str()) {
        return Err("Unknown export status".into());
    }
    let conn = state.conn()?;
    conn.execute(
        "UPDATE exports SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now(), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_export(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    conn.execute("DELETE FROM exports WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------- analytics

fn map_analytics(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnalyticsRow> {
    Ok(AnalyticsRow {
        id: row.get(0)?,
        publishing_id: row.get(1)?,
        platform_id: row.get(2)?,
        platform_name: row.get(3)?,
        episode_id: row.get(4)?,
        episode_title: row.get(5)?,
        captured_at: row.get(6)?,
        views: row.get(7)?,
        likes: row.get(8)?,
        comments: row.get(9)?,
        shares: row.get(10)?,
        watch_time_sec: row.get(11)?,
        avg_retention_pct: row.get(12)?,
        followers_gained: row.get(13)?,
        countries: crate::repo::json_strings(row.get::<_, String>(14)?),
        source: row.get(15)?,
        notes: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

const ANALYTICS_SELECT: &str = "SELECT a.id, a.publishing_id, a.platform_id, pl.name, a.episode_id,
    e.title, a.captured_at, a.views, a.likes, a.comments, a.shares, a.watch_time_sec,
    a.avg_retention_pct, a.followers_gained, a.countries, a.source, a.notes, a.created_at,
    a.updated_at
    FROM analytics a
    LEFT JOIN platforms pl ON pl.id = a.platform_id
    LEFT JOIN episodes e ON e.id = a.episode_id";

#[tauri::command]
pub fn list_analytics(
    state: State<'_, AppState>,
    from: Option<String>,
    to: Option<String>,
    platform_id: Option<String>,
    episode_id: Option<String>,
) -> Result<Vec<AnalyticsRow>, String> {
    let conn = state.conn()?;
    let mut sql = format!("{ANALYTICS_SELECT} WHERE 1 = 1");
    let mut args: Vec<String> = Vec::new();
    if let Some(f) = from {
        sql.push_str(" AND substr(a.captured_at, 1, 10) >= ?");
        args.push(f);
    }
    if let Some(t) = to {
        sql.push_str(" AND substr(a.captured_at, 1, 10) <= ?");
        args.push(t);
    }
    if let Some(p) = platform_id {
        sql.push_str(" AND a.platform_id = ?");
        args.push(p);
    }
    if let Some(e) = episode_id {
        sql.push_str(" AND a.episode_id = ?");
        args.push(e);
    }
    sql.push_str(" ORDER BY a.captured_at DESC LIMIT 500");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), map_analytics)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn save_analytics(
    state: State<'_, AppState>,
    input: AnalyticsInput,
) -> Result<AnalyticsRow, String> {
    if input.views < 0 || input.likes < 0 || input.comments < 0 || input.shares < 0 {
        return Err("Analytics values cannot be negative".into());
    }
    if input.avg_retention_pct < 0.0 || input.avg_retention_pct > 100.0 {
        return Err("Average retention must be between 0 and 100 percent".into());
    }
    if input.watch_time_sec < 0.0 {
        return Err("Watch time cannot be negative".into());
    }
    let conn = state.conn()?;
    let ts = now();
    let captured = input.captured_at.clone().unwrap_or_else(|| ts.clone());
    let countries = crate::repo::to_json_list(&input.countries);
    let source = if input.source.is_empty() { "manual".into() } else { input.source };

    let id = match input.id {
        Some(existing) => {
            conn.execute(
                "UPDATE analytics SET publishing_id = ?1, platform_id = ?2, episode_id = ?3,
                 captured_at = ?4, views = ?5, likes = ?6, comments = ?7, shares = ?8,
                 watch_time_sec = ?9, avg_retention_pct = ?10, followers_gained = ?11,
                 countries = ?12, source = ?13, notes = ?14, updated_at = ?15 WHERE id = ?16",
                params![
                    input.publishing_id,
                    input.platform_id,
                    input.episode_id,
                    captured,
                    input.views,
                    input.likes,
                    input.comments,
                    input.shares,
                    input.watch_time_sec,
                    input.avg_retention_pct,
                    input.followers_gained,
                    countries,
                    source,
                    input.notes,
                    ts,
                    existing
                ],
            )
            .map_err(|e| format!("Could not update the analytics entry: {e}"))?;
            existing
        }
        None => {
            let new = new_id();
            conn.execute(
                "INSERT INTO analytics (id, publishing_id, platform_id, episode_id, captured_at,
                 views, likes, comments, shares, watch_time_sec, avg_retention_pct,
                 followers_gained, countries, source, notes, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    new,
                    input.publishing_id,
                    input.platform_id,
                    input.episode_id,
                    captured,
                    input.views,
                    input.likes,
                    input.comments,
                    input.shares,
                    input.watch_time_sec,
                    input.avg_retention_pct,
                    input.followers_gained,
                    countries,
                    source,
                    input.notes,
                    ts,
                    ts
                ],
            )
            .map_err(|e| format!("Could not save the analytics entry: {e}"))?;
            new
        }
    };
    db::log_activity(
        &conn,
        "analytics",
        Some("analytics"),
        Some(&id),
        "Recorded performance data",
    );
    conn.query_row(
        &format!("{ANALYTICS_SELECT} WHERE a.id = ?1"),
        params![id],
        map_analytics,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_analytics(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.conn()?;
    conn.execute("DELETE FROM analytics WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn analytics_summary(
    state: State<'_, AppState>,
    from: Option<String>,
    to: Option<String>,
) -> Result<Totals, String> {
    let conn = state.conn()?;
    let mut sql = String::from(
        "SELECT COALESCE(SUM(views),0), COALESCE(SUM(likes),0), COALESCE(SUM(comments),0),
         COALESCE(SUM(shares),0), COALESCE(SUM(watch_time_sec),0), COALESCE(AVG(avg_retention_pct),0),
         COALESCE(SUM(followers_gained),0) FROM analytics WHERE 1 = 1",
    );
    let mut args: Vec<String> = Vec::new();
    if let Some(f) = from {
        sql.push_str(" AND substr(captured_at, 1, 10) >= ?");
        args.push(f);
    }
    if let Some(t) = to {
        sql.push_str(" AND substr(captured_at, 1, 10) <= ?");
        args.push(t);
    }
    conn.query_row(&sql, rusqlite::params_from_iter(args.iter()), |r| {
        let views: i64 = r.get(0)?;
        let likes: i64 = r.get(1)?;
        let comments: i64 = r.get(2)?;
        let shares: i64 = r.get(3)?;
        let engagement = likes + comments + shares;
        Ok(Totals {
            views,
            likes,
            comments,
            shares,
            watch_time_sec: r.get(4)?,
            avg_retention_pct: r.get(5)?,
            followers_gained: r.get(6)?,
            engagement_rate: if views > 0 {
                (engagement as f64 / views as f64) * 100.0
            } else {
                0.0
            },
        })
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn category_performance(
    state: State<'_, AppState>,
) -> Result<Vec<LabeledValue>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT e.category, COALESCE(SUM(a.views),0) FROM analytics a
             JOIN episodes e ON e.id = a.episode_id GROUP BY e.category ORDER BY 2 DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LabeledValue {
                label: r.get(0)?,
                value: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}
