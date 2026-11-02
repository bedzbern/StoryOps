use rusqlite::{params, Connection};
use std::path::Path;

use crate::util::{new_id, now};

pub const SCHEMA_VERSION: i64 = 1;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  root_path TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS episodes (
  id TEXT PRIMARY KEY,
  project_id TEXT,
  code TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  category TEXT NOT NULL DEFAULT 'Other',
  tags TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'idea',
  priority TEXT NOT NULL DEFAULT 'medium',
  due_date TEXT,
  progress INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  folder TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_episodes_status ON episodes(status);
CREATE INDEX IF NOT EXISTS idx_episodes_archived ON episodes(archived);

CREATE TABLE IF NOT EXISTS episode_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  episode_id TEXT NOT NULL,
  from_status TEXT,
  to_status TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_history_episode ON episode_history(episode_id, created_at);

CREATE TABLE IF NOT EXISTS ideas (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  body TEXT NOT NULL DEFAULT '',
  category TEXT NOT NULL DEFAULT 'Other',
  tags TEXT NOT NULL DEFAULT '[]',
  priority TEXT NOT NULL DEFAULT 'medium',
  status TEXT NOT NULL DEFAULT 'inbox',
  notes TEXT NOT NULL DEFAULT '',
  converted_episode_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ideas_status ON ideas(status);
CREATE INDEX IF NOT EXISTS idx_ideas_category ON ideas(category);

CREATE TABLE IF NOT EXISTS scripts (
  id TEXT PRIMARY KEY,
  episode_id TEXT,
  title TEXT NOT NULL,
  content TEXT NOT NULL DEFAULT '',
  word_count INTEGER NOT NULL DEFAULT 0,
  est_duration_sec INTEGER NOT NULL DEFAULT 0,
  current_version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_scripts_episode ON scripts(episode_id);

CREATE TABLE IF NOT EXISTS script_versions (
  id TEXT PRIMARY KEY,
  script_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  content TEXT NOT NULL,
  word_count INTEGER NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  FOREIGN KEY(script_id) REFERENCES scripts(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_script_version ON script_versions(script_id, version);

CREATE TABLE IF NOT EXISTS prompt_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  category TEXT NOT NULL,
  instructions TEXT NOT NULL,
  variables TEXT NOT NULL DEFAULT '[]',
  provider TEXT NOT NULL DEFAULT 'any',
  builtin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS narrations (
  id TEXT PRIMARY KEY,
  episode_id TEXT,
  title TEXT NOT NULL,
  file_path TEXT NOT NULL,
  format TEXT NOT NULL DEFAULT '',
  duration_sec REAL NOT NULL DEFAULT 0,
  sample_rate INTEGER,
  channels INTEGER,
  bitrate INTEGER,
  size_bytes INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'draft',
  notes TEXT NOT NULL DEFAULT '',
  version INTEGER NOT NULL DEFAULT 1,
  is_current INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_narrations_episode ON narrations(episode_id);

CREATE TABLE IF NOT EXISTS files (
  id TEXT PRIMARY KEY,
  episode_id TEXT,
  kind TEXT NOT NULL DEFAULT 'other',
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  ext TEXT NOT NULL DEFAULT '',
  size_bytes INTEGER NOT NULL DEFAULT 0,
  modified_at TEXT,
  hash TEXT,
  app_id TEXT,
  missing INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_files_episode ON files(episode_id);
CREATE INDEX IF NOT EXISTS idx_files_kind ON files(kind);

CREATE TABLE IF NOT EXISTS file_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_id TEXT NOT NULL,
  path TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  modified_at TEXT,
  hash TEXT,
  note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_file_versions ON file_versions(file_id, created_at);

CREATE TABLE IF NOT EXISTS applications (
  id TEXT PRIMARY KEY,
  app_key TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  version TEXT,
  install_dir TEXT,
  exe_path TEXT,
  cli_path TEXT,
  project_extensions TEXT NOT NULL DEFAULT '[]',
  capabilities TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL DEFAULT 'unknown',
  last_seen TEXT,
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS editing_projects (
  id TEXT PRIMARY KEY,
  episode_id TEXT NOT NULL,
  app_id TEXT,
  name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS exports (
  id TEXT PRIMARY KEY,
  episode_id TEXT NOT NULL,
  file_id TEXT,
  name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  format TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'draft',
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS platforms (
  id TEXT PRIMARY KEY,
  key TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  config TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS publishing (
  id TEXT PRIMARY KEY,
  episode_id TEXT NOT NULL,
  platform_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  caption TEXT NOT NULL DEFAULT '',
  hashtags TEXT NOT NULL DEFAULT '',
  target_audience TEXT NOT NULL DEFAULT '',
  scheduled_at TEXT,
  status TEXT NOT NULL DEFAULT 'draft',
  export_id TEXT,
  checklist TEXT NOT NULL DEFAULT '{}',
  post_url TEXT,
  published_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE,
  FOREIGN KEY(platform_id) REFERENCES platforms(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_publishing_status ON publishing(status);
CREATE INDEX IF NOT EXISTS idx_publishing_scheduled ON publishing(scheduled_at);

CREATE TABLE IF NOT EXISTS analytics (
  id TEXT PRIMARY KEY,
  publishing_id TEXT,
  platform_id TEXT,
  episode_id TEXT,
  captured_at TEXT NOT NULL,
  views INTEGER NOT NULL DEFAULT 0,
  likes INTEGER NOT NULL DEFAULT 0,
  comments INTEGER NOT NULL DEFAULT 0,
  shares INTEGER NOT NULL DEFAULT 0,
  watch_time_sec REAL NOT NULL DEFAULT 0,
  avg_retention_pct REAL NOT NULL DEFAULT 0,
  followers_gained INTEGER NOT NULL DEFAULT 0,
  countries TEXT NOT NULL DEFAULT '[]',
  source TEXT NOT NULL DEFAULT 'manual',
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(publishing_id) REFERENCES publishing(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_analytics_captured ON analytics(captured_at);

CREATE TABLE IF NOT EXISTS activity_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  entity_type TEXT,
  entity_id TEXT,
  message TEXT NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_logs(created_at DESC);

CREATE TABLE IF NOT EXISTS backups (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL,
  size_bytes INTEGER NOT NULL DEFAULT 0,
  kind TEXT NOT NULL DEFAULT 'manual',
  note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);
"#;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<(), String> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| format!("Could not read schema version: {e}"))?;

    if version < SCHEMA_VERSION {
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("Could not create the database schema: {e}"))?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| format!("Could not set schema version: {e}"))?;
    }
    seed(conn)?;
    Ok(())
}

pub fn default_settings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("theme", "dark"),
        ("projectRoot", ""),
        ("backupDirectory", ""),
        ("defaultExportDirectory", ""),
        ("fileMonitoring", "true"),
        ("notifications", "true"),
        ("backupReminderDays", "7"),
        ("autoBackupOnExit", "false"),
        ("logRetentionDays", "30"),
    ]
}

fn seed(conn: &Connection) -> Result<(), String> {
    let ts = now();
    for (key, value) in default_settings() {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, ts],
        )
        .map_err(|e| format!("Could not seed settings: {e}"))?;
    }

    let platforms = [
        ("youtube", "YouTube"),
        ("youtube_shorts", "YouTube Shorts"),
        ("instagram_reels", "Instagram Reels"),
        ("tiktok", "TikTok"),
        ("facebook_reels", "Facebook Reels"),
    ];
    for (key, name) in platforms {
        conn.execute(
            "INSERT OR IGNORE INTO platforms (id, key, name, enabled, config, created_at)
             VALUES (?1, ?2, ?3, 1, '{}', ?4)",
            params![format!("platform-{key}"), key, name, ts],
        )
        .map_err(|e| format!("Could not seed platforms: {e}"))?;
    }

    for template in builtin_templates() {
        conn.execute(
            "INSERT OR IGNORE INTO prompt_templates
             (id, name, category, instructions, variables, provider, builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
            params![
                template.0,
                template.1,
                template.2,
                template.3,
                template.4,
                template.5,
                ts,
                ts
            ],
        )
        .map_err(|e| format!("Could not seed prompt templates: {e}"))?;
    }
    Ok(())
}

/// Built-in prompt templates. These are text the user copies into an external
/// assistant; StoryOps never calls an AI provider itself.
fn builtin_templates() -> Vec<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "builtin-story-generation",
            "Story seed from an observation",
            "Story generation",
            "You are helping plan a single-narrator YouTube essay about everyday human experience.\n\nWork from this seed:\nTitle: {{title}}\nCategory: {{category}}\nNotes: {{notes}}\n\nProduce 5 distinct story directions. Each must:\n- be told by one narrator to the viewer,\n- start from an ordinary, recognisable moment,\n- move toward a psychological or philosophical insight,\n- avoid dialogue scenes, skits and POV roleplay,\n- be 150-250 words of prose per direction.\n\nEnd with a one-line recommendation and the emotional core of the strongest option.",
            "[\"title\",\"category\",\"notes\"]",
            "any",
        ),
        (
            "builtin-script-writing",
            "Full narration script",
            "Script writing",
            "Write a complete narration script for a single narrator.\n\nTitle: {{title}}\nCategory: {{category}}\nExisting notes: {{notes}}\n\nRules:\n- First person or close third person, spoken aloud, never a dialogue scene.\n- Open with a concrete moment, not a thesis statement.\n- 1,100-1,500 words for a 8-11 minute video.\n- Short paragraphs, natural breath points.\n- Build one clear idea; no listicle structure unless the idea demands it.\n- Close by returning to the opening image with a changed meaning.\n- No stage directions, no scene headings, no camera notes.\n\nReturn only the script text.",
            "[\"title\",\"category\",\"notes\"]",
            "any",
        ),
        (
            "builtin-script-improvement",
            "Tighten an existing script",
            "Script improvement",
            "Improve the script below without changing its meaning or voice.\n\nScript:\n{{script}}\n\nTasks:\n1. Remove any sentence that only restates the previous one.\n2. Replace abstract claims with concrete images where possible.\n3. Vary sentence length so it reads aloud naturally.\n4. Keep the opening and the closing image intact.\n5. Report the word count before and after.\n\nReturn the revised script, then a short list of what you changed and why.",
            "[\"script\"]",
            "any",
        ),
        (
            "builtin-hook-generation",
            "Opening hooks",
            "Hook generation",
            "Write 10 opening lines for this episode.\n\nTitle: {{title}}\nCategory: {{category}}\nScript summary: {{notes}}\n\nConstraints:\n- Each hook is 1-2 sentences, under 30 words.\n- No clickbait promises the video cannot keep.\n- No questions that the viewer can answer instantly.\n- Each should create a small, specific tension.\n\nRank them by how naturally they lead into the script's first paragraph.",
            "[\"title\",\"category\",\"notes\"]",
            "any",
        ),
        (
            "builtin-story-research",
            "Research brief",
            "Story research",
            "Build a research brief for a narrated essay.\n\nTopic: {{title}}\nCategory: {{category}}\nAngle: {{notes}}\n\nProvide:\n- 6 credible sources or known studies, with author/year and what each actually found,\n- 3 common claims about this topic that are overstated or unsupported,\n- 2 counterarguments a thoughtful viewer might raise,\n- concrete everyday examples that illustrate the idea without jargon.\n\nFlag anything you are not confident about instead of guessing.",
            "[\"title\",\"category\",\"notes\"]",
            "any",
        ),
        (
            "builtin-fact-checking",
            "Fact-check pass",
            "Fact checking",
            "Fact-check the script below.\n\nScript:\n{{script}}\n\nFor every factual claim:\n- quote the sentence,\n- state whether it is supported, contested or unsupported,\n- give the evidence you rely on, or say that you could not verify it.\n\nDo not rewrite the script. List the claims, then give a short list of edits that would make the unsupported ones honest.",
            "[\"script\"]",
            "any",
        ),
        (
            "builtin-title-generation",
            "Titles",
            "Title generation",
            "Generate 15 titles for this episode.\n\nScript:\n{{script}}\nCategory: {{category}}\n\nRules:\n- 4-9 words.\n- Curious but accurate; no all-caps, no emoji, no 'you won't believe'.\n- Mix direct statements, quiet questions and one paradoxical phrasing.\n\nGroup them by tone and mark your top three.",
            "[\"script\",\"category\"]",
            "any",
        ),
        (
            "builtin-description-generation",
            "Video description",
            "Description generation",
            "Write a YouTube description for this episode.\n\nTitle: {{title}}\nScript:\n{{script}}\n\nStructure:\n1. Two-sentence hook that expands on the title.\n2. One short paragraph on what the video explores.\n3. Chapters as timestamps with short titles, assuming a {{duration}} minute runtime.\n4. A closing line inviting reflection, not a demand to subscribe.\n\nKeep it under 900 characters before the chapters.",
            "[\"title\",\"script\",\"duration\"]",
            "any",
        ),
        (
            "builtin-publishing-captions",
            "Platform captions",
            "Publishing captions",
            "Write publishing captions for this episode for: YouTube, YouTube Shorts, Instagram Reels and TikTok.\n\nTitle: {{title}}\nSummary: {{notes}}\n\nFor each platform give:\n- a caption in that platform's natural register (never copy-paste the same text),\n- 5-8 relevant hashtags, no banned or spammy tags,\n- a note on the ideal length for that platform.\n\nDo not invent links or mentions.",
            "[\"title\",\"notes\"]",
            "any",
        ),
        (
            "builtin-production-guide",
            "Production guide",
            "Production guides",
            "Create a production guide for this episode.\n\nTitle: {{title}}\nScript:\n{{script}}\n\nProvide:\n- narration direction: pace, pauses, emotional beats,\n- a shot/visual plan described generically (StoryOps does not impose a visual style),\n- audio notes: music energy curve, silence moments,\n- an editing checklist in the order the work should happen,\n- what to check before export.\n\nKeep every instruction actionable.",
            "[\"title\",\"script\"]",
            "any",
        ),
    ]
}

pub fn log_activity(
    conn: &Connection,
    kind: &str,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    message: &str,
) {
    let _ = conn.execute(
        "INSERT INTO activity_logs (kind, entity_type, entity_id, message, meta, created_at)
         VALUES (?1, ?2, ?3, ?4, '{}', ?5)",
        params![kind, entity_type, entity_id, message, now()],
    );
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now()],
    )
    .map_err(|e| format!("Could not save setting {key}: {e}"))?;
    Ok(())
}

pub fn new_backup_id() -> String {
    new_id()
}
