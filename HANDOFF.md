# StoryOps Desktop — Handoff

> **Read this first if you are a new agent or a new session.**
> The full original specification is in
> `.cloud-attachments/imported-conversation-e5643927368a3a792d5eeb1cbd6252389862f61c0a20771343a32d08761f4e41.json`
> (message 1, role `user`). It is ~20 KB; read it before designing anything.

Last updated: 2026-11-02 (mid-implementation, nothing has been compiled yet).

---

## 1. What is being built

**StoryOps Desktop** — a real Tauri 2 + React/TypeScript + SQLite desktop app that
manages the whole content-production workflow (ideas → scripts → narration → editing →
publish → analytics) for a single-narrator YouTube channel, and integrates with the
ArtCraft suite of creative apps installed on this machine.

Requirements that must not be violated:

- No mockups, no fake analytics, no fabricated connectors. Every number comes from SQLite.
- Unsupported operations must return a clear "unsupported" error, never pretend.
- 11 working pages: Dashboard, Content Pipeline, Ideas, Scripts, Narration, Production
  Studio, File Library, Publishing, Analytics, App Connections, Settings.
- Project root directory is configurable; every episode has a permanent folder.
- Docs listed in §16 of the spec must be written and must describe reality.

---

## 2. Environment (verified facts — do not re-litigate)

| Item | Value |
|---|---|
| OS | Windows 11, MSYS/Git Bash shell (`uname` = MINGW64_NT-10.0-26200) |
| Node / npm | v24.18.0 / 11.16.0 |
| Rust | rustup installed at `~/.cargo` (installed during session) |
| Default toolchain | `stable-x86_64-pc-windows-gnu` (rustc 1.99.1) |
| MinGW | portable winlibs at `~/toolchain/mingw64` (gcc 16.2.0) |
| MSVC | **unusable**: VS 2026 has no C++ workload and no Windows SDK (`kernel32.lib` missing) |
| Tauri | `tauri` crate 2.12.5; **a real Tauri binary was built and linked successfully with GNU** |

### Mandatory PATH for every Rust command

```bash
export PATH="$HOME/toolchain/mingw64/bin:$HOME/.cargo/bin:$PATH"
```

Without that prefix, `cargo` is not found and/or the MSVC host linker is used and fails.

### Tool-call limits

`run_terminal_command` rejects `timeout_seconds > 300`. A first Tauri build takes ~3.5
minutes. Use `process_type: "BACKGROUND"` with a log file and poll:

```bash
export PATH="$HOME/toolchain/mingw64/bin:$HOME/.cargo/bin:$PATH"
cd src-tauri && nohup cargo check > /tmp/check.log 2>&1 & echo "pid $!"
# later:
tail -40 /tmp/check.log
```

Prefer `cargo check` while iterating; use `cargo build` only to prove linking.

---

## 3. ArtCraft apps — verified integration surface

All 16 apps are installed at `D:\<Name>Craft\` with exactly two executables:
`<lowercase>.exe` (GUI) and `<lowercase>-cli.exe` (headless automation).

Discovered apps: CADCraft, DeckCraft, DesignCraft, EffectCraft, FigCraft, FilmCraft,
GridCraft, LightCraft, PdfCraft, PhotoCraft, SolidCraft, SoundCraft, VectorCraft,
WordCraft (**plus** `C:\Program Files\Craft Anything`, which is an unrelated binary-analysis
tool — do **not** register it as an ArtCraft editor).

CLI versions verified: filmcraft 0.16.0, wordcraft 0.14.0, soundcraft 0.16.0,
pdfcraft 0.17.0.

### Commands actually executed (ground truth)

```bash
D:/FilmCraft/filmcraft-cli.exe --version                 # filmcraft-cli 0.16.0
D:/FilmCraft/filmcraft-cli.exe commands --json           # JSON array of {id,label,params,...}
D:/FilmCraft/filmcraft-cli.exe --save-as /tmp/x.fcproj exec file.newProject name=StoryOpsTest
D:/FilmCraft/filmcraft-cli.exe --project /tmp/x.fcproj inspect project
D:/FilmCraft/filmcraft.exe --help
# -> "Usage: filmcraft [options] [project.fcproj | media files...]"
```

Results: `file.newProject` **created** a 727-byte `.fcproj`; `inspect project` returned
project JSON; the GUI documents opening a project by passing the path as an argument.

Everything else in the catalogue below was read from each CLI's own `--help` output, not
guessed:

| App | Project ext | Documented headless surface |
|---|---|---|
| FilmCraft | `.fcproj` | `exec/run/commands/inspect/import/export/render/probe/mcp`, `--bridge <host:port>` to a running app |
| WordCraft | `.docx .doc .odt .rtf .md .txt .pdf .html .tex .json .png .xml` | `convert/info/text/inspect/render/run` |
| PdfCraft | `.pdf` | `info/render/text/edit/combine/extract/split/check/audit/batch/tools/run/mcp` |
| PhotoCraft | `.pcraft .psd .png .jpg .tif .webp .exr` | `convert/info/run/batch/droplet/commands/mcp/serve` |
| SoundCraft | `.scraft` | `info/convert/run/script/commands/describe/app/mcp` |
| EffectCraft | (none advertised) | `info/commands/exec/run/props/get/set/render-frame/render/script` |
| DeckCraft | `.deckcraft .pptx` | `info/render/render-transition/convert/run/commands/describe/app/mcp` |
| DesignCraft | (none advertised) | `run/commands/describe/script/app/mcp` |
| FigCraft | `.figcraft .penpot .json .fig .sketch .svg` | `info/export/convert/render/run/commands/mcp/plugin/connect` |
| CADCraft | `.dwg .dxf .dwt .svg .png .pdf` | `info/convert/run/commands/mcp` |
| GridCraft | `.xlsx .csv .tsv .json .html .pdf` | `info/convert/eval/cat/check/run/commands/functions/mcp` |
| VectorCraft | `.ait` | `run` (`--cmd/--commands/--export`), `mcp` |
| LightCraft | (library folder) | `run`, `mcp`; needs `--library DIR` for headless |
| SolidCraft | `.sc .step` | `run/eval/script/snapshot/bench-*` |

Connector rules: invoke the CLI **directly** with an argument array (never a shell),
validate the subcommand against the app's advertised set, confine any output path to the
episode folder, and surface the process exit code + stderr. Capability model:
`launch`, `openProject`, `createProject`, `executeCommand`, `getProjectStatus`,
`listCommands`, `mcp`, each marked supported/unsupported **per app**.

---

## 4. Repository state (what exists on disk)

```
package.json  package-lock.json  tsconfig.json  vite.config.ts  index.html
node_modules/                      (installed)
src/styles.css                     Tailwind 4 design system, light + dark tokens
src-tauri/Cargo.toml               tauri 2, rusqlite(bundled, backup), chrono, uuid, sha2
src-tauri/build.rs
src-tauri/tauri.conf.json          window 1480x940, identifier ai.storyops.desktop
src-tauri/capabilities/default.json
src-tauri/src/util.rs              ids, timestamps, path normalisation/containment, sha256, slugify
src-tauri/src/b64.rs               base64 encoder for media previews
src-tauri/src/models.rs            all serde camelCase DTOs
src-tauri/src/db.rs                full schema + migrations + seeding (platforms, 10 prompt templates)
src-tauri/src/repo.rs              row mappers + shared column lists
src-tauri/src/state.rs             AppState (Mutex<Connection>, root), library bootstrap
src-tauri/src/commands/{core,content,scripts,narrations,files,publishing}.rs
```

`src-tauri/src/commands/mod.rs` declares: content, core, connectors, files, maintenance,
narrations, publishing, scripts. **`connectors.rs` and `maintenance.rs` do not exist yet.**

**Nothing has been compiled.** Expect normal first-pass errors.

---

## 5. Remaining work, in order

### Step 1 — make the backend compile (highest priority)

1. Create `src-tauri/src/commands/maintenance.rs`:
   - `backup_database(note)` → `VACUUM INTO <root>/Backups/storyops-YYYYmmdd-HHMMSS.db`,
     insert into `backups`, set `lastBackupAt` in settings, log activity.
   - `list_backups()`, `restore_backup(id)` → make a `pre-restore` backup first, then use
     `rusqlite::backup::Backup` to copy the chosen file into the live connection.
   - `database_integrity()` → `PRAGMA integrity_check`, missing episode folders, missing
     registered files, orphan files, manifest drift → `IntegrityReport`.
   - `write_missing_manifests()` / `repair_manifests()`.
   - `log_tail(lines)` reading `state.log_path()`.
2. Create `src-tauri/src/commands/connectors.rs`:
   - `detect_applications()` scans `C:\`, `D:\`, `E:\`, `C:\Program Files`,
     `C:\Program Files (x86)`, `%LOCALAPPDATA%\Programs` for directories ending in `Craft`
     that contain `<lower(dir)>.exe`; reads version via `<cli> --version`; captures
     `--help` head; upserts `applications`.
   - `list_applications()`, `test_connection(id)`, `launch_application(id)`,
     `open_project(app_id, path)`, `create_project(app_id, episode_id, name)`,
     `run_app_command(app_id, subcommand, args[])` (allowlist + arg validation),
     `inspect_project(app_id, path)`, `list_app_commands(app_id)`.
3. Create `src-tauri/src/lib.rs` (build `AppState`, `manage`, `invoke_handler` with every
   `#[tauri::command]`) and `src-tauri/src/main.rs`
   (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`,
   `storyops_lib::run()`), plus `mod b64; mod db; mod models; mod repo; mod state; mod util; mod commands;`.
4. Generate icons: write a square SVG and run `npx tauri icon src-tauri/icons/logo.svg`.
5. `cargo check` → fix. Then `cargo build` to prove the GNU link.

Startup behaviour to implement: data dir = `app.path().app_data_dir()`, DB at
`<data_dir>/storyops.db`; default library root = `<Documents>/StoryOps Studio` (never
inside this repository), created with `Projects/Templates/{Scripts,Documents,Production}/Shared-Assets/Backups/Logs/database`.
Load `projectRoot` from settings when present.

Likely compile fixes: `State` is not `Clone` (already avoided); borrow conflicts when a
`MutexGuard` is held across calls that re-lock; `params_from_iter` needs owned args;
`open_path` signature in `tauri-plugin-opener` 2.x.

### Step 2 — frontend

Files not yet written: `src/main.tsx`, `src/App.tsx`, `src/lib/{bridge,types,format}.ts`,
`src/components/{ui,layout,charts,EpisodeDrawer}.tsx`, `src/pages/*.tsx` (11 pages).

- `src/lib/bridge.ts`: thin typed wrapper over `invoke`; every backend type mirrored in
  `types.ts`.
- Router: local state + hash, no extra dependency. Episode workspace is an overlay opened
  from Pipeline/Studio.
- Layout: fixed 216 px sidebar, top bar, content scroll area. Dark default, working
  light/dark switch persisted via `set_setting("theme", ...)`.
- Kanban: native HTML5 drag events across the 10 statuses; call `set_episode_status`
  (records history).
- Charts: hand-rolled SVG line/bar/donut fed by real data; empty state when
  `hasAnalytics === false`.
- Audio: `read_file_data(path)` → data URL → `<audio>`; save reported `duration` back
  with `update_narration`; playback rate control.
- Clipboard: `navigator.clipboard.writeText` for prompt copying.
- File dialogs: `@tauri-apps/plugin-dialog` `open`/`save`.

### Step 3 — verification

- `npm run build` (tsc + vite), `cargo check`, `cargo test`.
- Write Rust unit tests for: status progress, slugify, `is_within` containment, WAV
  probe, manifest round-trip, backup + restore, scheduler validation.
- Launch once (`npm run desktop:dev`) and confirm the window opens and the DB initialises;
  a native window cannot be screenshotted with the preview tools, so record this honestly.
- Database survives restart; app survives missing ArtCraft apps.

### Step 4 — documentation

`README.md`, `docs/PROJECT_OVERVIEW.md`, `ARCHITECTURE.md`, `DATABASE_SCHEMA.md`,
`ARTCRAFT_INTEGRATION.md`, `APPLICATION_CONNECTORS.md`, `FILE_MANAGEMENT.md`,
`USER_GUIDE.md`, `TESTING.md`, `KNOWN_LIMITATIONS.md`, `DEVELOPMENT_PROGRESS.md`.
State plainly what is verified vs. implemented-but-untested.

---

## 6. Decisions already made (keep them)

- **rusqlite over sqlx**: synchronous, no async runtime, `bundled` compiles SQLite with gcc.
- **No asset protocol / no `convertFileSrc`**: media reaches the webview as a base64 data
  URL from a validated Rust command (`read_file_data`, 80 MB cap, type allowlist).
- **No shell**: every external process is `Command::new(exe).args([..])`; frontend cannot
  pass arbitrary commands.
- **Deletes are recoverable**: files/episodes move to `<root>/Backups/Deleted/<stamp>-…`.
- **Imports never overwrite**: collisions get ` (2)`, ` (3)`.
- **Progress is derived** from status index (`idea`=0 … `published`=100).
- **Publishing is manual only**: `mark_published` is the single place that sets
  `published_at`; it never fabricates a URL.
- **Prompt templates are copy/paste only** — no AI API is called anywhere.

## 7. DB schema (already written in `db.rs`)

`settings, projects, episodes, episode_history, ideas, scripts, script_versions,
prompt_templates, narrations, files, file_versions, applications, editing_projects,
exports, platforms, publishing, analytics, activity_logs, backups`
— FK cascades on, WAL, `PRAGMA user_version = 1`, indexes on the obvious lookup columns.

Statuses: `idea, research, writing, review_script, narration, editing, final_review,
ready, scheduled, published`.
Idea statuses: `inbox, shortlisted, developing, converted, archived`.

## 8. Command surface (already written)

`get_settings/set_setting/get_app_info/get_dashboard/list_activity/open_external/
reveal_in_explorer`
`list_categories/list_episodes/get_episode/create_episode/update_episode/
set_episode_status/archive_episode/delete_episode/list_episode_history`
`list_ideas/create_idea/update_idea/delete_idea/convert_idea_to_episode`
`list_scripts/get_script/create_script/update_script/delete_script/list_script_versions/
get_script_version/restore_script_version/import_script_file/export_script`
`list_prompt_templates/save_prompt_template/delete_prompt_template/render_prompt`
`list_narrations/import_narration/update_narration/replace_narration/delete_narration`
`list_files/import_file/register_file/create_text_file/create_folder/rename_entry/
move_entry/copy_entry/delete_entry/path_metadata/episode_tree/file_versions/relink_file/
scan_external_changes/read_file_data`
`list_platforms/save_platform/list_schedules/save_schedule/delete_schedule/
mark_published/list_exports/register_export/update_export_status/delete_export/
list_analytics/save_analytics/delete_analytics/analytics_summary/category_performance`
plus (to write) the maintenance and connector commands from Step 1.

---

## 9. Gotchas

- `npm install` warns that `esbuild`'s postinstall was not approved; the binary is present
  and works, so ignore it.
- Rust toolchain installs were interrupted once; if you see
  `missing manifest in toolchain`, reinstall with
  `rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal`.
- `cargo` picks up MSYS `link.exe` (coreutils) if the GNU toolchain is not the default.
  Confirm with `rustc -vV | grep host` → `x86_64-pc-windows-gnu`.
- The repository itself is the code project; do **not** use it as the StoryOps library
  root at runtime.
- Test data may be created under `%TEMP%` / the library root only. The 727-byte
  `/tmp/crafttest/Demo.fcproj` from connector testing is disposable.
