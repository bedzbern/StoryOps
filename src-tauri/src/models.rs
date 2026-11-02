use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const STATUSES: [&str; 10] = [
    "idea",
    "research",
    "writing",
    "review_script",
    "narration",
    "editing",
    "final_review",
    "ready",
    "scheduled",
    "published",
];

pub const CATEGORIES: [&str; 11] = [
    "Psychology",
    "Philosophy",
    "Human Nature",
    "Relationships",
    "Emotional Experiences",
    "Daily Life",
    "Personal Growth",
    "Social Behavior",
    "Deep Conversations",
    "Observations",
    "Other",
];

pub fn status_label(status: &str) -> &'static str {
    match status {
        "idea" => "Idea",
        "research" => "Research",
        "writing" => "Writing",
        "review_script" => "Review Script",
        "narration" => "Narration",
        "editing" => "Editing",
        "final_review" => "Final Review",
        "ready" => "Ready",
        "scheduled" => "Scheduled",
        "published" => "Published",
        _ => "Unknown",
    }
}

pub fn status_progress(status: &str) -> i64 {
    match STATUSES.iter().position(|s| *s == status) {
        Some(i) => ((i as f64 / (STATUSES.len() - 1) as f64) * 100.0).round() as i64,
        None => 0,
    }
}

pub fn is_valid_status(status: &str) -> bool {
    STATUSES.contains(&status)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: String,
    pub project_id: Option<String>,
    pub code: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub status: String,
    pub status_label: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub progress: i64,
    pub archived: bool,
    pub folder: String,
    pub created_at: String,
    pub updated_at: String,
    pub script_count: i64,
    pub file_count: i64,
    pub narration_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub episode_id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeUpdate {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub archived: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    pub id: String,
    pub title: String,
    pub body: String,
    pub category: String,
    pub tags: Vec<String>,
    pub priority: String,
    pub status: String,
    pub notes: String,
    pub converted_episode_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub id: String,
    pub episode_id: Option<String>,
    pub episode_title: Option<String>,
    pub title: String,
    pub content: String,
    pub word_count: i64,
    pub est_duration_sec: i64,
    pub current_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptVersion {
    pub id: String,
    pub script_id: String,
    pub version: i64,
    pub word_count: i64,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub instructions: String,
    pub variables: Vec<String>,
    pub provider: String,
    pub builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInput {
    pub name: String,
    pub category: String,
    pub instructions: String,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default)]
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Narration {
    pub id: String,
    pub episode_id: Option<String>,
    pub episode_title: Option<String>,
    pub title: String,
    pub file_path: String,
    pub format: String,
    pub duration_sec: f64,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub bitrate: Option<i64>,
    pub size_bytes: i64,
    pub status: String,
    pub notes: String,
    pub version: i64,
    pub is_current: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: String,
    pub episode_id: Option<String>,
    pub episode_title: Option<String>,
    pub kind: String,
    pub name: String,
    pub path: String,
    pub ext: String,
    pub size_bytes: i64,
    pub modified_at: Option<String>,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub missing: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersion {
    pub id: i64,
    pub file_id: String,
    pub path: String,
    pub size_bytes: i64,
    pub modified_at: Option<String>,
    pub hash: Option<String>,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: String,
    pub app_key: String,
    pub name: String,
    pub version: Option<String>,
    pub install_dir: Option<String>,
    pub exe_path: Option<String>,
    pub cli_path: Option<String>,
    pub project_extensions: Vec<String>,
    pub capabilities: serde_json::Value,
    pub status: String,
    pub last_seen: Option<String>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub id: String,
    pub key: String,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: String,
    pub episode_id: String,
    pub episode_title: String,
    pub platform_id: String,
    pub platform_name: String,
    pub title: String,
    pub description: String,
    pub caption: String,
    pub hashtags: String,
    pub target_audience: String,
    pub scheduled_at: Option<String>,
    pub status: String,
    pub export_id: Option<String>,
    pub export_name: Option<String>,
    pub checklist: serde_json::Value,
    pub post_url: Option<String>,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInput {
    pub id: Option<String>,
    pub episode_id: String,
    pub platform_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub caption: String,
    #[serde(default)]
    pub hashtags: String,
    #[serde(default)]
    pub target_audience: String,
    #[serde(default)]
    pub scheduled_at: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub export_id: Option<String>,
    #[serde(default)]
    pub checklist: serde_json::Value,
    #[serde(default)]
    pub post_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRow {
    pub id: String,
    pub episode_id: String,
    pub episode_title: String,
    pub name: String,
    pub file_path: String,
    pub format: String,
    pub status: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsRow {
    pub id: String,
    pub publishing_id: Option<String>,
    pub platform_id: Option<String>,
    pub platform_name: Option<String>,
    pub episode_id: Option<String>,
    pub episode_title: Option<String>,
    pub captured_at: String,
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub watch_time_sec: f64,
    pub avg_retention_pct: f64,
    pub followers_gained: i64,
    pub countries: Vec<String>,
    pub source: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsInput {
    pub id: Option<String>,
    #[serde(default)]
    pub publishing_id: Option<String>,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub episode_id: Option<String>,
    #[serde(default)]
    pub captured_at: Option<String>,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub likes: i64,
    #[serde(default)]
    pub comments: i64,
    #[serde(default)]
    pub shares: i64,
    #[serde(default)]
    pub watch_time_sec: f64,
    #[serde(default)]
    pub avg_retention_pct: f64,
    #[serde(default)]
    pub followers_gained: i64,
    #[serde(default)]
    pub countries: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub watch_time_sec: f64,
    pub avg_retention_pct: f64,
    pub followers_gained: i64,
    pub engagement_rate: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: i64,
    pub kind: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRow {
    pub id: String,
    pub path: String,
    pub size_bytes: i64,
    pub kind: String,
    pub note: String,
    pub created_at: String,
    pub exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCounts {
    pub total_projects: i64,
    pub ideas_awaiting: i64,
    pub scripts_ready: i64,
    pub narrations_completed: i64,
    pub in_production: i64,
    pub ready_for_review: i64,
    pub scheduled_posts: i64,
    pub published_posts: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPoint {
    pub date: String,
    pub value: f64,
    pub secondary: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabeledValue {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformPerf {
    pub platform: String,
    pub posts: i64,
    pub views: i64,
    pub likes: i64,
    pub engagement_rate: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub counts: DashboardCounts,
    pub recent_activity: Vec<Activity>,
    pub queue: Vec<Episode>,
    pub upcoming: Vec<Schedule>,
    pub content_performance: Vec<SeriesPoint>,
    pub platform_performance: Vec<PlatformPerf>,
    pub production_progress: Vec<LabeledValue>,
    pub has_analytics: bool,
    pub root_path: String,
    pub database_path: String,
    pub last_backup_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub tauri_version: String,
    pub data_dir: String,
    pub database_path: String,
    pub root_path: String,
    pub os: String,
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub code: Option<i32>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub database_ok: bool,
    pub database_message: String,
    pub episodes_checked: i64,
    pub missing_folders: Vec<String>,
    pub missing_files: Vec<String>,
    pub orphan_files: i64,
    pub manifest_issues: Vec<String>,
    pub checked_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeReport {
    pub checked: i64,
    pub new: Vec<FileEntry>,
    pub modified: Vec<FileEntry>,
    pub missing: Vec<FileEntry>,
    pub checked_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeTree {
    pub path: String,
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: i64,
    pub modified_at: Option<String>,
    pub ext: String,
    pub registered_id: Option<String>,
    pub children: Vec<TreeEntry>,
}
