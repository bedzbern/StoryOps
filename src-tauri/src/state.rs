use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub root: Mutex<PathBuf>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf, db_path: PathBuf, conn: Connection, root: PathBuf) -> Self {
        Self {
            db: Mutex::new(conn),
            root: Mutex::new(root),
            data_dir,
            db_path,
        }
    }

    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.db
            .lock()
            .map_err(|_| "Database lock is poisoned; restart StoryOps".to_string())
    }

    pub fn root(&self) -> PathBuf {
        self.root
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    pub fn set_root(&self, path: PathBuf) -> Result<(), String> {
        bootstrap_library(&path)?;
        let mut guard = self
            .root
            .lock()
            .map_err(|_| "Root lock is poisoned".to_string())?;
        *guard = path;
        Ok(())
    }

    pub fn log_path(&self) -> PathBuf {
        self.data_dir.join("logs").join("storyops.log")
    }
}

/// Create the library directory structure. Idempotent.
pub fn bootstrap_library(root: &Path) -> Result<(), String> {
    let dirs = [
        root.to_path_buf(),
        root.join("Projects"),
        root.join("Templates"),
        root.join("Templates").join("Scripts"),
        root.join("Templates").join("Documents"),
        root.join("Templates").join("Production"),
        root.join("Shared-Assets"),
        root.join("Backups"),
        root.join("Logs"),
        root.join("database"),
    ];
    for dir in dirs {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    }
    Ok(())
}

pub const EPISODE_SUBDIRS: [&str; 6] = [
    "Documents",
    "Narration",
    "Media",
    "Editing",
    "Subtitles",
    "Exports",
];

pub fn create_episode_folders(root: &Path, folder_name: &str) -> Result<PathBuf, String> {
    let base = root.join("Projects").join(folder_name);
    fs::create_dir_all(&base)
        .map_err(|e| format!("Could not create episode folder {}: {e}", base.display()))?;
    for sub in EPISODE_SUBDIRS {
        let dir = base.join(sub);
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    }
    Ok(base)
}

pub fn append_log(state: &AppState, level: &str, message: &str) {
    use std::io::Write;
    let path = state.log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(
            file,
            "{} [{}] {}",
            crate::util::now(),
            level,
            message.replace('\n', " ")
        );
    }
}
