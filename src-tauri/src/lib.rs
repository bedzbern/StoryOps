mod b64;
mod commands;
mod db;
mod models;
mod repo;
mod state;
mod util;

use std::path::PathBuf;
use tauri::Manager;

use state::{bootstrap_library, AppState};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = handle.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("logs"))?;
            let db_path = data_dir.join("storyops.db");

            let conn = db::open(&db_path)
                .map_err(|e| -> Box<dyn std::error::Error> { format!("Could not open the database at {}: {e}", db_path.display()).into() })?;
            db::migrate(&conn).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            let configured = db::get_setting(&conn, "projectRoot").filter(|s| !s.trim().is_empty());
            let root = match configured {
                Some(value) => PathBuf::from(value),
                None => {
                    let documents = handle
                        .path()
                        .document_dir()
                        .unwrap_or_else(|_| data_dir.clone());
                    let default_root = documents.join("StoryOps Studio");
                    db::set_setting(&conn, "projectRoot", &default_root.display().to_string())
                        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                    default_root
                }
            };
            bootstrap_library(&root).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            let has_project: i64 = conn
                .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
                .unwrap_or(0);
            if has_project == 0 {
                let ts = util::now();
                let _ = conn.execute(
                    "INSERT INTO projects (id, name, description, root_path, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    rusqlite::params![
                        util::new_id(),
                        "StoryOps Library",
                        "Local content production library",
                        root.display().to_string(),
                        ts
                    ],
                );
            }

            db::log_activity(
                &conn,
                "app",
                Some("application"),
                None,
                &format!("StoryOps started with library at {}", root.display()),
            );

            app.manage(AppState::new(data_dir, db_path, conn, root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // core
            commands::core::get_settings,
            commands::core::set_setting,
            commands::core::get_app_info,
            commands::core::get_dashboard,
            commands::core::list_activity,
            commands::core::open_external,
            commands::core::reveal_in_explorer,
            // episodes and ideas
            commands::content::list_categories,
            commands::content::list_episodes,
            commands::content::get_episode,
            commands::content::create_episode,
            commands::content::update_episode,
            commands::content::set_episode_status,
            commands::content::archive_episode,
            commands::content::delete_episode,
            commands::content::list_episode_history,
            commands::content::list_ideas,
            commands::content::create_idea,
            commands::content::update_idea,
            commands::content::delete_idea,
            commands::content::convert_idea_to_episode,
            // scripts and prompt templates
            commands::scripts::list_scripts,
            commands::scripts::get_script,
            commands::scripts::create_script,
            commands::scripts::update_script,
            commands::scripts::delete_script,
            commands::scripts::list_script_versions,
            commands::scripts::get_script_version,
            commands::scripts::restore_script_version,
            commands::scripts::import_script_file,
            commands::scripts::export_script,
            commands::scripts::list_prompt_templates,
            commands::scripts::save_prompt_template,
            commands::scripts::delete_prompt_template,
            commands::scripts::render_prompt,
            // narration
            commands::narrations::list_narrations,
            commands::narrations::import_narration,
            commands::narrations::update_narration,
            commands::narrations::replace_narration,
            commands::narrations::delete_narration,
            // files
            commands::files::list_files,
            commands::files::import_file,
            commands::files::register_file,
            commands::files::create_text_file,
            commands::files::create_folder,
            commands::files::rename_entry,
            commands::files::move_entry,
            commands::files::copy_entry,
            commands::files::delete_entry,
            commands::files::path_metadata,
            commands::files::episode_tree,
            commands::files::file_versions,
            commands::files::relink_file,
            commands::files::scan_external_changes,
            commands::files::read_file_data,
            // publishing and analytics
            commands::publishing::list_platforms,
            commands::publishing::save_platform,
            commands::publishing::list_schedules,
            commands::publishing::save_schedule,
            commands::publishing::delete_schedule,
            commands::publishing::mark_published,
            commands::publishing::list_exports,
            commands::publishing::register_export,
            commands::publishing::update_export_status,
            commands::publishing::delete_export,
            commands::publishing::list_analytics,
            commands::publishing::save_analytics,
            commands::publishing::delete_analytics,
            commands::publishing::analytics_summary,
            commands::publishing::category_performance,
            // maintenance
            commands::maintenance::backup_database,
            commands::maintenance::list_backups,
            commands::maintenance::restore_backup,
            commands::maintenance::database_integrity,
            commands::maintenance::repair_manifests,
            commands::maintenance::log_tail,
            // ArtCraft connectors
            commands::connectors::detect_applications,
            commands::connectors::list_applications,
            commands::connectors::test_connection,
            commands::connectors::launch_application,
            commands::connectors::open_project,
            commands::connectors::create_project,
            commands::connectors::execute_app_command,
            commands::connectors::list_app_commands,
            commands::connectors::inspect_project
        ])
        .run(tauri::generate_context!())
        .expect("StoryOps failed to start");
}
