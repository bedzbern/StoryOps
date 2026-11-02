use rusqlite::Row;

use crate::models::{status_label, Episode, Schedule};

pub const EPISODE_COLS: &str = "e.id, e.project_id, e.code, e.title, e.description, e.category, e.tags,
     e.status, e.priority, e.due_date, e.progress, e.archived, e.folder, e.created_at, e.updated_at,
     (SELECT COUNT(*) FROM scripts s WHERE s.episode_id = e.id),
     (SELECT COUNT(*) FROM files f WHERE f.episode_id = e.id),
     (SELECT COUNT(*) FROM narrations n WHERE n.episode_id = e.id)";

pub fn map_episode(row: &Row<'_>) -> rusqlite::Result<Episode> {
    let status: String = row.get(7)?;
    Ok(Episode {
        id: row.get(0)?,
        project_id: row.get(1)?,
        code: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        category: row.get(5)?,
        tags: json_strings(row.get::<_, String>(6)?),
        status_label: status_label(&status).to_string(),
        status,
        priority: row.get(8)?,
        due_date: row.get(9)?,
        progress: row.get(10)?,
        archived: row.get::<_, i64>(11)? != 0,
        folder: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        script_count: row.get(15)?,
        file_count: row.get(16)?,
        narration_count: row.get(17)?,
    })
}

pub const SCHEDULE_COLS: &str = "p.id, p.episode_id, e.title, p.platform_id, pl.name, p.title,
     p.description, p.caption, p.hashtags, p.target_audience, p.scheduled_at, p.status,
     p.export_id, ex.name, p.checklist, p.post_url, p.published_at, p.created_at, p.updated_at";

pub const SCHEDULE_JOINS: &str = "FROM publishing p
     JOIN episodes e ON e.id = p.episode_id
     JOIN platforms pl ON pl.id = p.platform_id
     LEFT JOIN exports ex ON ex.id = p.export_id";

pub fn map_schedule(row: &Row<'_>) -> rusqlite::Result<Schedule> {
    Ok(Schedule {
        id: row.get(0)?,
        episode_id: row.get(1)?,
        episode_title: row.get(2)?,
        platform_id: row.get(3)?,
        platform_name: row.get(4)?,
        title: row.get(5)?,
        description: row.get(6)?,
        caption: row.get(7)?,
        hashtags: row.get(8)?,
        target_audience: row.get(9)?,
        scheduled_at: row.get(10)?,
        status: row.get(11)?,
        export_id: row.get(12)?,
        export_name: row.get(13)?,
        checklist: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or(serde_json::json!({})),
        post_url: row.get(15)?,
        published_at: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

pub fn json_strings(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

pub fn to_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".into())
}

pub fn to_json_list(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".into())
}
