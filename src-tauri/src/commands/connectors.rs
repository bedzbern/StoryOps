use rusqlite::params;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::State;

use crate::db;
use crate::models::{Application, CommandResult};
use crate::state::AppState;
use crate::util::{is_within, new_id, now, slugify, validate_path_string};

/// Read-only subcommands StoryOps is willing to invoke. Anything that writes,
/// deletes, renders or runs arbitrary scripts is deliberately excluded: those
/// belong in the app's own UI where the user sees the result.
const ALLOWED_SUBCOMMANDS: [&str; 12] = [
    "info",
    "inspect",
    "text",
    "commands",
    "describe",
    "probe",
    "cat",
    "eval",
    "tools",
    "version",
    "--version",
    "--help",
];

struct CatalogEntry {
    key: &'static str,
    name: &'static str,
    exts: &'static [&'static str],
    /// Whether the CLI publishes a documented way to create a native project.
    create: bool,
    /// Whether the GUI documents opening a file passed as an argument.
    open: bool,
    /// Whether the CLI documents structured project inspection.
    inspect: bool,
    notes: &'static str,
}

/// Catalogue assembled from each application's own `--help` output. Entries
/// only claim what the CLI documents; everything else reports unsupported.
fn catalog() -> Vec<CatalogEntry> {
    vec![
        CatalogEntry {
            key: "filmcraft",
            name: "FilmCraft",
            exts: &["fcproj"],
            create: true,
            open: true,
            inspect: true,
            notes: "Video editing. Verified: file.newProject creates a .fcproj; the GUI opens a project path argument.",
        },
        CatalogEntry {
            key: "wordcraft",
            name: "WordCraft",
            exts: &["docx", "doc", "odt", "rtf", "md", "txt", "pdf", "html", "tex", "json", "png", "xml"],
            create: false,
            open: false,
            inspect: true,
            notes: "Documents. CLI documented: convert, info, text, inspect, render, run.",
        },
        CatalogEntry {
            key: "pdfcraft",
            name: "PdfCraft",
            exts: &["pdf"],
            create: false,
            open: false,
            inspect: true,
            notes: "PDF. CLI documented: info, render, text, edit, combine, extract, split, check, audit, batch, tools, run.",
        },
        CatalogEntry {
            key: "photocraft",
            name: "PhotoCraft",
            exts: &["pcraft", "psd", "png", "jpg", "jpeg", "tif", "tiff", "webp", "exr"],
            create: false,
            open: false,
            inspect: true,
            notes: "Raster images. CLI documented: convert, info, run, batch, droplet, commands, serve.",
        },
        CatalogEntry {
            key: "soundcraft",
            name: "SoundCraft",
            exts: &["scraft"],
            create: false,
            open: false,
            inspect: true,
            notes: "Audio. CLI documented: info, convert, run, script, commands, describe, app, parity, plugins.",
        },
        CatalogEntry {
            key: "effectcraft",
            name: "EffectCraft",
            exts: &[],
            create: false,
            open: false,
            inspect: false,
            notes: "Motion graphics. CLI documented: info, commands, exec, run, props, get, set, render-frame, render, script.",
        },
        CatalogEntry {
            key: "deckcraft",
            name: "DeckCraft",
            exts: &["deckcraft", "pptx"],
            create: false,
            open: false,
            inspect: true,
            notes: "Presentations. CLI documented: info, render, render-transition, convert, run, commands, describe, app.",
        },
        CatalogEntry {
            key: "designcraft",
            name: "DesignCraft",
            exts: &[],
            create: false,
            open: false,
            inspect: false,
            notes: "Page design. CLI documented: run, commands, describe, script, app, perf, bench, validate-pdf.",
        },
        CatalogEntry {
            key: "figcraft",
            name: "FigCraft",
            exts: &["figcraft", "penpot", "json", "fig", "sketch", "svg"],
            create: false,
            open: false,
            inspect: true,
            notes: "Interface design. CLI documented: info, export, convert, render, run, commands, plugin, connect.",
        },
        CatalogEntry {
            key: "cadcraft",
            name: "CADCraft",
            exts: &["dwg", "dxf", "dwt", "svg"],
            create: false,
            open: false,
            inspect: true,
            notes: "CAD drawings. CLI documented: info, convert, run, commands, sample, perf.",
        },
        CatalogEntry {
            key: "gridcraft",
            name: "GridCraft",
            exts: &["xlsx", "csv", "tsv", "json", "html", "pdf"],
            create: false,
            open: false,
            inspect: true,
            notes: "Spreadsheets. CLI documented: info, convert, eval, cat, check, run, commands, functions.",
        },
        CatalogEntry {
            key: "vectorcraft",
            name: "VectorCraft",
            exts: &["ait"],
            create: false,
            open: false,
            inspect: false,
            notes: "Vector illustration. CLI documented: run (--cmd/--commands/--export), mcp.",
        },
        CatalogEntry {
            key: "lightcraft",
            name: "LightCraft",
            exts: &[],
            create: false,
            open: false,
            inspect: false,
            notes: "Photo library and raw developer. CLI documented: run, mcp; headless runs need --library DIR.",
        },
        CatalogEntry {
            key: "solidcraft",
            name: "SolidCraft",
            exts: &["sc", "step", "stp"],
            create: false,
            open: false,
            inspect: true,
            notes: "Parametric CAD. CLI documented: run, eval, script, snapshot, bench-open.",
        },
    ]
}

fn find_catalog(key: &str) -> Option<&'static CatalogEntry> {
    static CACHE: std::sync::OnceLock<Vec<CatalogEntry>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(catalog)
        .iter()
        .find(|e| e.key == key)
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for letter in ["C", "D", "E", "F", "G"] {
        let p = PathBuf::from(format!("{letter}:\\"));
        if p.exists() {
            roots.push(p);
        }
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        roots.push(PathBuf::from(pf));
    }
    if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(pf));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Programs"));
    }
    roots
}

struct Detected {
    key: String,
    name: String,
    dir: PathBuf,
    exe: PathBuf,
    cli: Option<PathBuf>,
}

fn detect_on_disk() -> Vec<Detected> {
    let mut found: Vec<Detected> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for root in candidate_roots() {
        let Ok(read) = fs::read_dir(&root) else { continue };
        for entry in read.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let dir_name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let lower = dir_name.to_lowercase();
            if !lower.ends_with("craft") || lower == "craft anything" {
                continue;
            }
            if seen.contains(&lower) {
                continue;
            }
            let Ok(files) = fs::read_dir(&dir) else { continue };
            let mut exe: Option<PathBuf> = None;
            let mut cli: Option<PathBuf> = None;
            for f in files.flatten() {
                let p = f.path();
                if !p.is_file() {
                    continue;
                }
                let Some(ext) = p.extension() else { continue };
                if !ext.to_string_lossy().eq_ignore_ascii_case("exe") {
                    continue;
                }
                let stem = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if stem == format!("{lower}-cli") {
                    cli = Some(p);
                } else if stem == lower || stem == "artcraft-launcher" {
                    exe = Some(p);
                }
            }
            let Some(exe) = exe else { continue };
            seen.insert(lower.clone());
            found.push(Detected {
                key: lower,
                name: dir_name,
                dir,
                exe,
                cli,
            });
        }
    }
    found
}

fn run_capture(exe: &Path, args: &[String]) -> Result<(i32, String, String), String> {
    let output = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Could not run {}: {e}", exe.display()))?;
    let code = output.status.code().unwrap_or(-1);
    let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stdout.len() > 2 * 1024 * 1024 {
        stdout.truncate(2 * 1024 * 1024);
        stdout.push_str("\n…output truncated");
    }
    if stderr.len() > 64 * 1024 {
        stderr.truncate(64 * 1024);
    }
    Ok((code, stdout, stderr))
}

fn read_version(cli: &Option<PathBuf>) -> Option<String> {
    let exe = cli.as_ref()?;
    let (code, stdout, _) = run_capture(exe, &["--version".to_string()]).ok()?;
    if code != 0 {
        return None;
    }
    let line = stdout.lines().next()?.trim().to_string();
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

fn read_help_head(cli: &Option<PathBuf>) -> Option<String> {
    let exe = cli.as_ref()?;
    let (_, stdout, stderr) = run_capture(exe, &["--help".to_string()]).ok()?;
    let text = if stdout.trim().is_empty() { stderr } else { stdout };
    let mut head: String = text.chars().take(4000).collect();
    if text.chars().count() > 4000 {
        head.push_str("\n…");
    }
    Some(head)
}

fn advertised_subcommands(help: &str) -> Vec<String> {
    let tokens = [
        "exec", "run", "commands", "inspect", "info", "probe", "convert", "render", "export",
        "text", "describe", "cat", "eval", "props", "get", "set", "mcp", "script", "batch",
        "combine", "extract", "split", "check", "audit", "tools", "droplet", "serve", "app",
        "snapshot", "perf", "bench",
    ];
    let lower = help.to_lowercase();
    let mut out = Vec::new();
    for token in tokens {
        if lower.contains(&format!(" {token}"))
            || lower.contains(&format!("\n  {token}"))
            || lower.contains(&format!("{token} "))
            || lower.contains(&format!("|{token}"))
        {
            out.push(token.to_string());
        }
    }
    out
}

fn capabilities_json(
    entry: Option<&CatalogEntry>,
    cli: &Option<PathBuf>,
    help: &Option<String>,
) -> serde_json::Value {
    let advertised = help
        .as_ref()
        .map(|h| advertised_subcommands(h))
        .unwrap_or_default();
    let has_cli = cli.as_ref().map(|p| p.exists()).unwrap_or(false);
    json!({
        "launch": true,
        "openProject": entry.map(|e| e.open).unwrap_or(false),
        "createProject": entry.map(|e| e.create).unwrap_or(false),
        "inspectProject": entry.map(|e| e.inspect).unwrap_or(false),
        "executeCommand": has_cli,
        "listCommands": has_cli,
        "mcp": advertised.iter().any(|c| c == "mcp"),
        "cliPath": cli.as_ref().map(|p| p.display().to_string()),
        "advertisedSubcommands": advertised,
        "allowedSubcommands": ALLOWED_SUBCOMMANDS,
        "helpHead": help,
        "notes": entry.map(|e| e.notes).unwrap_or("Discovered application without a documented automation catalogue entry."),
        "verified": entry.map(|e| e.key == "filmcraft").unwrap_or(false),
    })
}

fn upsert(conn: &rusqlite::Connection, detected: &Detected) -> Result<String, String> {
    let entry = find_catalog(&detected.key);
    let exts: Vec<String> = entry
        .map(|e| e.exts.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let version = read_version(&detected.cli);
    let help = read_help_head(&detected.cli);
    let capabilities = capabilities_json(entry, &detected.cli, &help);
    let name = entry.map(|e| e.name.to_string()).unwrap_or_else(|| detected.name.clone());
    let ts = now();

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM applications WHERE app_key = ?1",
            params![detected.key],
            |r| r.get(0),
        )
        .ok();

    let id = existing.unwrap_or_else(new_id);
    conn.execute(
        "INSERT INTO applications (id, app_key, name, version, install_dir, exe_path, cli_path,
         project_extensions, capabilities, status, last_seen, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'connected', ?10, ?11, ?12, ?12)
         ON CONFLICT(app_key) DO UPDATE SET
           name = excluded.name,
           version = excluded.version,
           install_dir = excluded.install_dir,
           exe_path = excluded.exe_path,
           cli_path = excluded.cli_path,
           project_extensions = excluded.project_extensions,
           capabilities = excluded.capabilities,
           status = excluded.status,
           last_seen = excluded.last_seen,
           updated_at = excluded.updated_at",
        params![
            id,
            detected.key,
            name,
            version,
            detected.dir.display().to_string(),
            detected.exe.display().to_string(),
            detected.cli.as_ref().map(|p| p.display().to_string()),
            serde_json::to_string(&exts).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&capabilities).unwrap_or_else(|_| "{}".into()),
            ts,
            entry.map(|e| e.notes).unwrap_or_default()
        ],
    )
    .map_err(|e| format!("Could not store {}: {e}", detected.key))?;

    let _id: String = conn
        .query_row(
            "SELECT id FROM applications WHERE app_key = ?1",
            params![detected.key],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(_id)
}

fn map_application(row: &rusqlite::Row<'_>) -> rusqlite::Result<Application> {
    Ok(Application {
        id: row.get(0)?,
        app_key: row.get(1)?,
        name: row.get(2)?,
        version: row.get(3)?,
        install_dir: row.get(4)?,
        exe_path: row.get(5)?,
        cli_path: row.get(6)?,
        project_extensions: crate::repo::json_strings(row.get::<_, String>(7)?),
        capabilities: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or(json!({})),
        status: row.get(9)?,
        last_seen: row.get(10)?,
        notes: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

const APP_COLS: &str = "id, app_key, name, version, install_dir, exe_path, cli_path,
    project_extensions, capabilities, status, last_seen, notes, created_at, updated_at";

fn fetch_app(conn: &rusqlite::Connection, id: &str) -> Result<Application, String> {
    conn.query_row(
        &format!("SELECT {APP_COLS} FROM applications WHERE id = ?1"),
        params![id],
        map_application,
    )
    .map_err(|e| format!("Application not found: {e}"))
}

#[tauri::command]
pub fn detect_applications(state: State<'_, AppState>) -> Result<Vec<Application>, String> {
    let conn = state.conn()?;
    let detected = detect_on_disk();
    let mut ids = Vec::new();
    for app in &detected {
        match upsert(&conn, app) {
            Ok(id) => ids.push(id),
            Err(e) => {
                db::log_activity(&conn, "connector", Some("application"), None, &e);
            }
        }
    }

    // Anything previously known but no longer on disk becomes "missing".
    let known: Vec<(String, Option<String>)> = {
        let mut stmt = conn
            .prepare("SELECT id, exe_path FROM applications")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    for (id, exe) in known {
        let exists = exe
            .as_ref()
            .map(|p| PathBuf::from(p).exists())
            .unwrap_or(false);
        if !exists && !ids.contains(&id) {
            conn.execute(
                "UPDATE applications SET status = 'missing', updated_at = ?1 WHERE id = ?2",
                params![now(), id],
            )
            .ok();
        }
    }

    db::log_activity(
        &conn,
        "connector",
        Some("application"),
        None,
        &format!("Detected {} installed ArtCraft applications", detected.len()),
    );

    let mut stmt = conn
        .prepare(&format!(
            "SELECT {APP_COLS} FROM applications WHERE status != 'missing' ORDER BY name"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_application)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn list_applications(state: State<'_, AppState>) -> Result<Vec<Application>, String> {
    let conn = state.conn()?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {APP_COLS} FROM applications ORDER BY status = 'missing', name"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_application)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn test_connection(state: State<'_, AppState>, id: String) -> Result<Application, String> {
    let conn = state.conn()?;
    let app = fetch_app(&conn, &id)?;
    let exe_ok = app
        .exe_path
        .as_ref()
        .map(|p| PathBuf::from(p).exists())
        .unwrap_or(false);
    let status = if exe_ok { "connected" } else { "missing" };
    conn.execute(
        "UPDATE applications SET status = ?1, last_seen = ?2, updated_at = ?3 WHERE id = ?4",
        params![status, now(), now(), id],
    )
    .map_err(|e| e.to_string())?;
    db::log_activity(
        &conn,
        "connector",
        Some("application"),
        Some(&id),
        &format!("Connection test for {}: {status}", app.name),
    );
    fetch_app(&conn, &id)
}

#[tauri::command]
pub fn launch_application(state: State<'_, AppState>, id: String) -> Result<u32, String> {
    let conn = state.conn()?;
    let app = fetch_app(&conn, &id)?;
    let exe = app
        .exe_path
        .clone()
        .ok_or("This application has no executable path registered")?;
    let path = PathBuf::from(&exe);
    if !path.exists() {
        return Err(format!("{} is no longer installed at {exe}", app.name));
    }
    let child = Command::new(&path)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not launch {}: {e}", app.name))?;
    db::log_activity(
        &conn,
        "connector",
        Some("application"),
        Some(&id),
        &format!("Launched {}", app.name),
    );
    Ok(child.id())
}

#[tauri::command]
pub fn open_project(
    state: State<'_, AppState>,
    app_id: String,
    path: String,
) -> Result<u32, String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    if !is_within(&root, &p) {
        return Err("Only project files inside the StoryOps library can be opened".into());
    }
    if !p.is_file() {
        return Err("That project file does not exist".into());
    }
    let conn = state.conn()?;
    let app = fetch_app(&conn, &app_id)?;
    let supported = app
        .capabilities
        .get("openProject")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !supported {
        return Err(format!(
            "{} does not document opening a project from a file argument. Launch the app and open the file from its own File menu.",
            app.name
        ));
    }
    let exe = app.exe_path.clone().ok_or("No executable registered")?;
    let child = Command::new(&exe)
        .arg(&p)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not open {} in {}: {e}", p.display(), app.name))?;
    db::log_activity(
        &conn,
        "connector",
        Some("application"),
        Some(&app_id),
        &format!("Opened {} in {}", p.display(), app.name),
    );
    Ok(child.id())
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    app_id: String,
    episode_id: String,
    name: String,
) -> Result<serde_json::Value, String> {
    if name.trim().is_empty() {
        return Err("A project name is required".into());
    }
    if name.contains(['\n', '\r']) {
        return Err("The project name contains invalid characters".into());
    }
    let root = state.root();
    let conn = state.conn()?;
    let app = fetch_app(&conn, &app_id)?;
    let supported = app
        .capabilities
        .get("createProject")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !supported {
        return Err(format!(
            "{} does not document headless project creation. Create the project in the app, then register the file in StoryOps.",
            app.name
        ));
    }
    let cli = app.cli_path.clone().ok_or("No automation CLI registered")?;
    let folder: String = conn
        .query_row(
            "SELECT folder FROM episodes WHERE id = ?1",
            params![episode_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Episode not found: {e}"))?;
    let dir = root.join("Projects").join(&folder).join("Editing");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut target = dir.join(format!("{}.fcproj", slugify(&name)));
    let mut i = 2;
    while target.exists() {
        target = dir.join(format!("{}-{i}.fcproj", slugify(&name)));
        i += 1;
    }

    let args = vec![
        "--save-as".to_string(),
        target.display().to_string(),
        "exec".to_string(),
        "file.newProject".to_string(),
        format!("name={name}"),
    ];
    let (code, stdout, stderr) = run_capture(Path::new(&cli), &args)?;
    if code != 0 {
        return Err(format!(
            "{} could not create the project (exit {code}): {}",
            app.name,
            stderr.lines().next().unwrap_or("no error text")
        ));
    }

    let file_id = crate::commands::files::register_path_public(
        &conn,
        &root,
        Some(&episode_id),
        &target,
    )?;
    let editing_id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO editing_projects (id, episode_id, app_id, name, file_path, status, notes,
         created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'active', '', ?6, ?7)",
        params![
            editing_id,
            episode_id,
            app_id,
            name.trim(),
            target.display().to_string(),
            ts,
            ts
        ],
    )
    .map_err(|e| format!("Could not record the editing project: {e}"))?;
    db::log_activity(
        &conn,
        "connector",
        Some("editing_project"),
        Some(&editing_id),
        &format!("Created {} project {}", app.name, target.display()),
    );

    Ok(json!({
        "path": target.display().to_string(),
        "fileId": file_id,
        "editingProjectId": editing_id,
        "stdout": stdout,
    }))
}

#[tauri::command]
pub fn execute_app_command(
    state: State<'_, AppState>,
    app_id: String,
    args: Vec<String>,
) -> Result<CommandResult, String> {
    let conn = state.conn()?;
    let app = fetch_app(&conn, &app_id)?;
    let allowed = app
        .capabilities
        .get("executeCommand")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !allowed {
        return Err(format!("{} exposes no automation CLI", app.name));
    }
    let cli = app.cli_path.clone().ok_or("No automation CLI registered")?;
    if args.is_empty() {
        return Err("No command was supplied".into());
    }
    for arg in &args {
        if arg.contains('\n') || arg.contains('\r') || arg.contains('\0') {
            return Err("An argument contained an invalid character".into());
        }
        if arg.len() > 4096 {
            return Err("An argument is too long".into());
        }
    }
    let first = args[0].trim_start_matches('-').to_lowercase();
    let known = ALLOWED_SUBCOMMANDS
        .iter()
        .any(|s| s.trim_start_matches('-') == first);
    if !known {
        return Err(format!(
            "\"{}\" is not one of StoryOps' allow-listed read-only subcommands",
            args[0]
        ));
    }
    let root = state.root();
    for arg in args.iter().skip(1) {
        let looks_like_path = arg.contains(":\\") || arg.starts_with("\\\\") || arg.starts_with('/');
        if looks_like_path {
            let p = PathBuf::from(arg);
            if !is_within(&root, &p) {
                return Err(format!(
                    "Arguments may only reference files inside the StoryOps library: {arg}"
                ));
            }
        }
    }

    let (code, stdout, stderr) = run_capture(Path::new(&cli), &args)?;
    let data = serde_json::from_str::<serde_json::Value>(&stdout).ok();
    db::log_activity(
        &conn,
        "connector",
        Some("application"),
        Some(&app_id),
        &format!("Ran {} {}", app.name, args.join(" ")),
    );
    Ok(CommandResult {
        ok: code == 0,
        stdout,
        stderr,
        code: Some(code),
        data,
    })
}

#[tauri::command]
pub fn list_app_commands(
    state: State<'_, AppState>,
    app_id: String,
) -> Result<serde_json::Value, String> {
    let conn = state.conn()?;
    let app = fetch_app(&conn, &app_id)?;
    let allowed = app
        .capabilities
        .get("listCommands")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !allowed {
        return Err(format!("{} exposes no automation CLI", app.name));
    }
    let cli = app.cli_path.clone().ok_or("No automation CLI registered")?;

    let (mut code, mut stdout, mut stderr) =
        run_capture(Path::new(&cli), &["commands".to_string(), "--json".to_string()])?;
    if code != 0 || stdout.trim().is_empty() {
        let fallback = run_capture(Path::new(&cli), &["commands".to_string()])?;
        code = fallback.0;
        stdout = fallback.1;
        stderr = fallback.2;
    }
    if code != 0 {
        return Err(format!(
            "{} could not list its commands: {}",
            app.name,
            stderr.lines().next().unwrap_or("no error text")
        ));
    }
    let parsed = serde_json::from_str::<serde_json::Value>(&stdout).ok();
    Ok(json!({
        "app": app.name,
        "raw": stdout,
        "commands": parsed,
    }))
}

#[tauri::command]
pub fn inspect_project(
    state: State<'_, AppState>,
    app_id: String,
    path: String,
) -> Result<serde_json::Value, String> {
    validate_path_string(&path)?;
    let root = state.root();
    let p = PathBuf::from(&path);
    if !is_within(&root, &p) {
        return Err("Only projects inside the StoryOps library can be inspected".into());
    }
    if !p.is_file() {
        return Err("That project file does not exist".into());
    }
    let conn = state.conn()?;
    let app = fetch_app(&conn, &app_id)?;
    let supported = app
        .capabilities
        .get("inspectProject")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !supported {
        return Err(format!(
            "{} does not document structured project inspection",
            app.name
        ));
    }
    let cli = app.cli_path.clone().ok_or("No automation CLI registered")?;

    let args = if app.app_key == "filmcraft" {
        vec![
            "--project".to_string(),
            p.display().to_string(),
            "inspect".to_string(),
            "project".to_string(),
        ]
    } else {
        vec!["info".to_string(), p.display().to_string()]
    };
    let (code, stdout, stderr) = run_capture(Path::new(&cli), &args)?;
    if code != 0 {
        return Err(format!(
            "{} could not inspect the project: {}",
            app.name,
            stderr.lines().next().unwrap_or("no error text")
        ));
    }
    Ok(serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap_or_else(|_| json!({ "raw": stdout })))
}
