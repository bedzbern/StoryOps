use rusqlite::params;
use std::collections::HashMap;
use tauri::State;

use crate::db;
use crate::models::*;
use crate::repo::{map_episode, map_schedule, EPISODE_COLS, SCHEDULE_COLS, SCHEDULE_JOINS};
use crate::state::AppState;
use crate::util::{is_within, now};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    if value.len() > 20_000 {
        return Err("Setting value is too large".into());
    }
    let allowed = [
        "theme",
        "projectRoot",
        "backupDirectory",
        "defaultExportDirectory",
        "fileMonitoring",
        "notifications",
        "backupReminderDays",
        "autoBackupOnExit",
        "logRetentionDays",
        "onboardingDismissed",
        "lastBackupAt",
    ];
    if !allowed.contains(&key.as_str()) {
        return Err(format!("Unknown setting: {key}"));
    }

    if key == "projectRoot" {
        let path = std::path::PathBuf::from(&value);
        if value.trim().is_empty() {
            return Err("Project root cannot be empty".into());
        }
        state.set_root(path)?;
        let conn = state.conn()?;
        db::set_setting(&conn, &key, &value)?;
        db::log_activity(
            &conn,
            "settings",
            Some("setting"),
            Some(&key),
            "Project root directory changed",
        );
        return Ok(());
    }

    if key == "backupDirectory" && !value.trim().is_empty() {
        let root = state.root();
        if !is_within(&root, std::path::Path::new(&value)) {
            return Err("The backup directory must be inside the project root".into());
        }
    }

    let conn = state.conn()?;
    db::set_setting(&conn, &key, &value)?;
    Ok(())
}

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> Result<AppInfo, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut settings = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        settings.insert(k, v);
    }
    Ok(AppInfo {
        name: "StoryOps Desktop".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        tauri_version: tauri::VERSION.into(),
        data_dir: state.data_dir.display().to_string(),
        database_path: state.db_path.display().to_string(),
        root_path: state.root().display().to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        settings,
    })
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<Dashboard, String> {
    let conn = state.conn()?;
    let root = state.root();

    let count = |sql: &str| -> i64 {
        conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0)
    };

    let counts = DashboardCounts {
        total_projects: count("SELECT COUNT(*) FROM episodes WHERE archived = 0"),
        ideas_awaiting: count(
            "SELECT COUNT(*) FROM ideas WHERE status IN ('inbox','shortlisted','developing')",
        ),
        scripts_ready: count(
            "SELECT COUNT(*) FROM scripts s JOIN episodes e ON e.id = s.episode_id
             WHERE s.word_count > 0 AND e.archived = 0",
        ),
        narrations_completed: count(
            "SELECT COUNT(*) FROM narrations WHERE status = 'approved' AND is_current = 1",
        ),
        in_production: count(
            "SELECT COUNT(*) FROM episodes WHERE archived = 0 AND status IN ('narration','editing')",
        ),
        ready_for_review: count(
            "SELECT COUNT(*) FROM episodes WHERE archived = 0 AND status = 'final_review'",
        ),
        scheduled_posts: count("SELECT COUNT(*) FROM publishing WHERE status = 'scheduled'"),
        published_posts: count("SELECT COUNT(*) FROM publishing WHERE status = 'published'"),
    };

    let mut recent_activity = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, entity_type, entity_id, message, created_at
                 FROM activity_logs ORDER BY id DESC LIMIT 12",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Activity {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    entity_type: r.get(2)?,
                    entity_id: r.get(3)?,
                    message: r.get(4)?,
                    created_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            recent_activity.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut queue = Vec::new();
    {
        let sql = format!(
            "SELECT {EPISODE_COLS} FROM episodes e
             WHERE e.archived = 0 AND e.status NOT IN ('published')
             ORDER BY CASE e.priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                      e.updated_at DESC LIMIT 8"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_episode)
            .map_err(|e| e.to_string())?;
        for row in rows {
            queue.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut upcoming = Vec::new();
    {
        let sql = format!(
            "SELECT {SCHEDULE_COLS} {SCHEDULE_JOINS}
             WHERE p.status = 'scheduled' ORDER BY p.scheduled_at ASC LIMIT 6"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_schedule)
            .map_err(|e| e.to_string())?;
        for row in rows {
            upcoming.push(row.map_err(|e| e.to_string())?);
        }
    }

    let has_analytics: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM analytics)", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0)
        == 1;

    let mut content_performance = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT substr(captured_at, 1, 10) AS d, SUM(views), SUM(likes + comments + shares)
                 FROM analytics GROUP BY d ORDER BY d DESC LIMIT 30",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SeriesPoint {
                    date: r.get(0)?,
                    value: r.get::<_, f64>(1).unwrap_or(0.0),
                    secondary: r.get::<_, f64>(2).unwrap_or(0.0),
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            content_performance.push(row.map_err(|e| e.to_string())?);
        }
        content_performance.reverse();
    }

    let mut platform_performance = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(pl.name, 'Unassigned') AS name,
                        COUNT(DISTINCT a.publishing_id) AS posts,
                        COALESCE(SUM(a.views), 0),
                        COALESCE(SUM(a.likes), 0),
                        COALESCE(SUM(a.likes + a.comments + a.shares), 0)
                 FROM analytics a LEFT JOIN platforms pl ON pl.id = a.platform_id
                 GROUP BY name ORDER BY SUM(a.views) DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let views: i64 = r.get(2)?;
                let engagements: i64 = r.get(4)?;
                Ok(PlatformPerf {
                    platform: r.get(0)?,
                    posts: r.get(1)?,
                    views,
                    likes: r.get(3)?,
                    engagement_rate: if views > 0 {
                        (engagements as f64 / views as f64) * 100.0
                    } else {
                        0.0
                    },
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            platform_performance.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut status_counts: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT status, COUNT(*) FROM episodes WHERE archived = 0 GROUP BY status")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (status, n) = row.map_err(|e| e.to_string())?;
            status_counts.insert(status, n);
        }
    }
    let production_progress = STATUSES
        .iter()
        .map(|s| LabeledValue {
            label: status_label(s).to_string(),
            value: *status_counts.get(*s).unwrap_or(&0) as f64,
        })
        .collect();

    let last_backup_at = db::get_setting(&conn, "lastBackupAt").filter(|v| !v.is_empty());

    Ok(Dashboard {
        counts,
        recent_activity,
        queue,
        upcoming,
        content_performance,
        platform_performance,
        production_progress,
        has_analytics,
        root_path: root.display().to_string(),
        database_path: state.db_path.display().to_string(),
        last_backup_at,
    })
}

#[tauri::command]
pub fn list_activity(state: State<'_, AppState>, limit: Option<i64>) -> Result<Vec<Activity>, String> {
    let conn = state.conn()?;
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, entity_type, entity_id, message, created_at
             FROM activity_logs ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(Activity {
                id: r.get(0)?,
                kind: r.get(1)?,
                entity_type: r.get(2)?,
                entity_id: r.get(3)?,
                message: r.get(4)?,
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

/// Open a path in its default Windows application. Only paths inside the
/// StoryOps library are accepted, and no shell is involved.
#[tauri::command]
pub fn open_external(state: State<'_, AppState>, path: String) -> Result<(), String> {
    crate::util::validate_path_string(&path)?;
    let root = state.root();
    let p = std::path::PathBuf::from(&path);
    if !is_within(&root, &p) {
        if let Ok(conn) = state.conn() {
            db::log_activity(
                &conn,
                "security",
                Some("file"),
                None,
                &format!("Blocked opening a path outside the library: {path}"),
            );
        }
        return Err("Only files inside the StoryOps library can be opened".into());
    }
    if !p.exists() {
        return Err(format!("File no longer exists: {path}"));
    }
    tauri_plugin_opener::open_path(&p, None::<&str>).map_err(|e| format!("Could not open file: {e}"))
}

#[tauri::command]
pub fn reveal_in_explorer(state: State<'_, AppState>, path: String) -> Result<(), String> {
    crate::util::validate_path_string(&path)?;
    let root = state.root();
    let p = std::path::PathBuf::from(&path);
    if !is_within(&root, &p) {
        return Err("Only files inside the StoryOps library can be revealed".into());
    }
    if !p.exists() {
        return Err(format!("Path no longer exists: {path}"));
    }
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer.exe");
        if p.is_dir() {
            cmd.arg(&p);
        } else {
            cmd.arg(format!("/select,{}", p.display()));
        }
        cmd.spawn().map_err(|e| format!("Could not open Explorer: {e}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    {
        tauri_plugin_opener::reveal_item_in_dir(&p)
            .map_err(|e| format!("Could not reveal the file: {e}"))
    }
}

pub fn touch_episode(conn: &rusqlite::Connection, episode_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE episodes SET updated_at = ?1 WHERE id = ?2",
        params![now(), episode_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
