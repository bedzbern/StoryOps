# StoryOps Desktop — Technical Handoff for External Review

**Document status:** factual snapshot of the repository at commit `e0ced33`.
**Scope:** describes what exists on disk and what was actually executed. Planned or
partial work is labelled as such. No functionality is claimed that has not been verified.

**Review boundary:** this assessment covers the repository only. It does not assert that
the application launches, renders, or performs end-to-end workflows, because those steps
have not yet been run.

---

## 1. PROJECT STRUCTURE

### 1.1 Source tree

Paths below exclude `node_modules/`, `src-tauri/target/` (build output) and `.git/`.

```
StoryOps/
├── .gitignore
├── HANDOFF.md                     # working notes for the implementing agent
├── index.html                     # Vite entry document; loads /src/main.tsx
├── package.json                   # frontend scripts and dependency pins
├── package-lock.json
├── tsconfig.json                  # strict TS, bundler resolution, noEmit
├── vite.config.ts                 # React + Tailwind 4 plugins, port 1420
├── docs/
│   └── HANDOFF_FOR_REVIEW.md      # this document
├── src/                           # React frontend (incomplete)
│   ├── main.tsx                   # entry point — currently malformed (see §5.5)
│   ├── styles.css                 # Tailwind import + design tokens + component classes
│   ├── components/
│   │   ├── charts.tsx             # SVG line chart, bar list, donut, mini bars
│   │   └── ui.tsx                 # buttons, cards, modal, dialog, tabs, toasts, states
│   └── lib/
│       ├── bridge.ts              # typed wrappers over every Tauri command
│       ├── format.ts              # date/number/size/duration helpers
│       └── types.ts               # TS mirrors of all Rust DTOs
└── src-tauri/                     # Rust desktop backend (complete, compiles)
    ├── Cargo.toml                 # tauri 2, rusqlite (bundled), chrono, uuid, sha2
    ├── Cargo.lock
    ├── build.rs                   # tauri_build::build()
    ├── tauri.conf.json            # product metadata, window, CSP, bundle config
    ├── capabilities/
    │   └── default.json           # core:default, dialog:default, opener:default
    ├── icons/                     # generated from icons/logo.svg via `tauri icon`
    └── src/
        ├── main.rs                # binary entry; calls storyops_lib::run()
        ├── lib.rs                 # app bootstrap, state wiring, invoke_handler list
        ├── util.rs                # ids, timestamps, path normalisation, slugify, hashing
        ├── b64.rs                 # base64 encoder for media previews
        ├── models.rs              # all serde DTOs (camelCase on the wire)
        ├── repo.rs                # row mappers and shared SQL column lists
        ├── db.rs                  # schema, migrations, seeding, activity logging
        ├── state.rs               # AppState, library bootstrap, log file
        └── commands/
            ├── mod.rs
            ├── core.rs            # settings, app info, dashboard, activity, open/reveal
            ├── content.rs         # episodes, history, ideas, manifests
            ├── scripts.rs         # scripts, versions, prompt templates
            ├── narrations.rs      # narration import, versions, approval
            ├── files.rs           # file registry, tree, move/copy/delete, monitoring
            ├── publishing.rs      # platforms, schedules, exports, analytics
            ├── maintenance.rs     # backup, restore, integrity, manifest repair, logs
            └── connectors.rs      # ArtCraft detection and capability-gated automation
```

### 1.2 Purpose of each major area

| Area | Purpose |
|---|---|
| `src/` | React UI. Only shared primitives and the typed bridge exist; no page components. |
| `src-tauri/src/db.rs` | Single source of truth for schema, seeding and migration versioning. |
| `src-tauri/src/commands/` | One module per domain; every public function annotated `#[tauri::command]`. |
| `src-tauri/src/connectors.rs` | Discovery of installed ArtCraft apps plus a curated capability catalogue. |
| `capabilities/default.json` | Tauri permission set exposed to the webview. |
| `docs/` | Review and handoff documentation. |

### 1.3 Current entry points

- **Backend binary entry:** `src-tauri/src/main.rs` → `storyops_lib::run()`.
- **Backend application entry:** `src-tauri/src/lib.rs::run()` — builds the Tauri app,
  opens and migrates SQLite in `.setup`, resolves the library root, registers 85 commands.
- **Frontend entry (declared):** `index.html` loads `/src/main.tsx`.
- **Frontend entry (actual state):** `src/main.tsx` is a single malformed line that does
  not parse; see §5.5. There is no `App` component and no router.

---

## 2. RUST BACKEND

Approximately 6,000 lines of Rust across 17 files.

### 2.1 Modules

| File | Lines | Responsibility |
|---|---:|---|
| `main.rs` | 5 | Windows subsystem attribute; delegates to the library. |
| `lib.rs` | 171 | Tauri builder, plugins, startup sequence, command registration. |
| `util.rs` | 122 | `now()`, `new_id()`, `slugify()`, `count_words()`, duration estimate, `norm()`/`is_within()` path containment, `sha256_file()`, `human_size()`, `validate_path_string()`, `friendly_err()`. Several helpers are currently unused (dead-code warnings). |
| `b64.rs` | 26 | Dependency-free base64 encoder used to hand media bytes to the webview. |
| `models.rs` | 578 | DTOs with `#[serde(rename_all = "camelCase")]`; status list and progress mapping; validation predicates. |
| `repo.rs` | 79 | `map_episode`, `map_schedule`, shared SQL column/join constants, JSON helpers. |
| `db.rs` | 489 | 19-table schema, `migrate()`, default settings, platform and prompt-template seeding, `log_activity()`, setting accessors. |
| `state.rs` | 108 | `AppState { db: Mutex<Connection>, root: Mutex<PathBuf>, data_dir, db_path }`, library bootstrap, episode subfolder creation, log append. |
| `commands/core.rs` | 393 | Settings, app info, dashboard aggregation, activity feed, opening/revealing files. |
| `commands/content.rs` | 626 | Episode CRUD, status transitions with history, archive, recoverable delete, idea CRUD, idea→episode conversion, `episode.json` manifest writer. |
| `commands/scripts.rs` | 592 | Script CRUD with automatic version rows, version listing/restore, import/export, prompt-template CRUD and variable rendering. |
| `commands/narrations.rs` | 412 | Audio import with WAV header probing, metadata updates, versioned replacement, delete-to-recycle. |
| `commands/files.rs` | 1062 | File registry, import/register, folder and text-file creation, rename/move/copy, recoverable delete, tree view, metadata, relink, external-change scan, data-URL reads. |
| `commands/publishing.rs` | 654 | Platform CRUD, publishing records, publish marking, exports, analytics CRUD and aggregates. |
| `commands/maintenance.rs` | 305 | `VACUUM INTO` backups, restore with safety copy, integrity report, manifest repair, log tail. |
| `commands/connectors.rs` | 887 | Application discovery, version/help probing, capability model, launch/open/create/execute/inspect. |

### 2.2 Registered Tauri commands (85)

**core.rs (7):** `get_settings`, `set_setting`, `get_app_info`, `get_dashboard`,
`list_activity`, `open_external`, `reveal_in_explorer`.

**content.rs (14):** `list_categories`, `list_episodes`, `get_episode`, `create_episode`,
`update_episode`, `set_episode_status`, `archive_episode`, `delete_episode`,
`list_episode_history`, `list_ideas`, `create_idea`, `update_idea`, `delete_idea`,
`convert_idea_to_episode`.

**scripts.rs (14):** `list_scripts`, `get_script`, `create_script`, `update_script`,
`delete_script`, `list_script_versions`, `get_script_version`, `restore_script_version`,
`import_script_file`, `export_script`, `list_prompt_templates`, `save_prompt_template`,
`delete_prompt_template`, `render_prompt`.

**narrations.rs (5):** `list_narrations`, `import_narration`, `update_narration`,
`replace_narration`, `delete_narration`.

**files.rs (15):** `list_files`, `import_file`, `register_file`, `create_text_file`,
`create_folder`, `rename_entry`, `move_entry`, `copy_entry`, `delete_entry`,
`path_metadata`, `episode_tree`, `file_versions`, `relink_file`, `scan_external_changes`,
`read_file_data`.

**publishing.rs (15):** `list_platforms`, `save_platform`, `list_schedules`,
`save_schedule`, `delete_schedule`, `mark_published`, `list_exports`, `register_export`,
`update_export_status`, `delete_export`, `list_analytics`, `save_analytics`,
`delete_analytics`, `analytics_summary`, `category_performance`.

**maintenance.rs (6):** `backup_database`, `list_backups`, `restore_backup`,
`database_integrity`, `repair_manifests`, `log_tail`.

**connectors.rs (9):** `detect_applications`, `list_applications`, `test_connection`,
`launch_application`, `open_project`, `create_project`, `execute_app_command`,
`list_app_commands`, `inspect_project`.

All 85 are declared in the `tauri::generate_handler!` list in `lib.rs`. No command is
defined but unregistered, and no registration references a missing function — this was
confirmed by the compiler.

### 2.3 Database initialisation and migrations

Implemented in `src-tauri/src/lib.rs` (setup closure) and `src-tauri/src/db.rs`.

Startup sequence:

1. Resolve the Tauri app data directory and create `logs/` beneath it.
2. Open `<app_data_dir>/storyops.db` with `foreign_keys=ON`, `journal_mode=WAL`,
   `synchronous=NORMAL`, and a 5-second busy timeout.
3. `db::migrate()` reads `PRAGMA user_version`. If it is below `SCHEMA_VERSION` (currently
   `1`), the entire `SCHEMA` SQL batch executes and `user_version` is set to 1.
4. `seed()` inserts `INSERT OR IGNORE` defaults: nine settings keys, five platforms, and
   ten built-in prompt templates.
5. The library root is read from `settings.projectRoot`; if empty, it defaults to
   `<Documents>/StoryOps Studio` and is persisted.
6. `bootstrap_library()` creates `Projects/`, `Templates/{Scripts,Documents,Production}/`,
   `Shared-Assets/`, `Backups/`, `Logs/`, `database/`.
7. A single `projects` row is inserted if the table is empty.
8. `AppState` is registered with the open connection.

**Migration status:** schema version 1 only. There is a single forward migration; there
are **no incremental migrations, no down migrations, and no per-version upgrade steps**.
Because every statement is `CREATE TABLE IF NOT EXISTS`, re-running is safe, but altering
an existing table in a future version would require adding a new migration branch that
does not currently exist.

### 2.4 Filesystem operations

All operations live in `commands/files.rs`, `commands/narrations.rs`,
`commands/content.rs` and `commands/state.rs`.

- **Library bootstrap:** fixed folder set under the configured root (`state.rs`).
- **Episode folders:** `Projects/<code>-<slug>/` with `Documents`, `Narration`, `Media`,
  `Editing`, `Subtitles`, `Exports`. The folder name is permanent; renaming an episode
  title does not move files.
- **Manifests:** `write_manifest()` regenerates `episode.json` from the database after
  episode create/update/status change, file import, narration changes, export
  registration and publish marking. Contents include IDs, metadata, scripts, audio, media,
  native projects, exports, target platforms and timestamps.
- **Import:** files are copied into the episode's type-appropriate subfolder; name
  collisions receive ` (2)`, ` (3)` suffixes. Nothing is overwritten.
- **Registration:** only paths inside the library are accepted for `register_file`.
  Import and narration sources may be outside the library because they are user-picked.
- **Rename / move / copy:** new names are validated against Windows-reserved characters;
  destinations must be inside the library; existing targets are refused. SQL path
  records are updated, including prefix rewrites for nested entries.
- **Delete:** entries are moved to `Backups/Deleted/<timestamp>-<name>`; database rows are
  then removed. Deleting an episode moves its whole folder there before the row is deleted.
- **Read for preview:** `read_file_data` returns a `data:` URL, capped at 80 MB, restricted
  to an extension allowlist (audio, images, plain text). No custom asset protocol is
  enabled.
- **Reveal/open:** `open_external` uses `tauri-plugin-opener`; `reveal_in_explorer` spawns
  `explorer.exe` with `/select,`. Both validate containment first.

### 2.5 Connector architecture

Implemented in `commands/connectors.rs`.

- **Discovery:** scans drive roots `C:`–`G:`, `%ProgramFiles%`, `%ProgramFiles(x86)%` and
  `%LOCALAPPDATA%\Programs` one level deep for directories whose name ends in `craft`
  (case-insensitive), excluding `Craft Anything`. A candidate is accepted only if it
  contains `<dir>.exe` or `artcraft-launcher.exe`; `<dir>-cli.exe` is captured when
  present.
- **Capability model:** a static catalogue (14 entries) supplies the display name, project
  extensions and three booleans — `create`, `open`, `inspect` — derived from the app's own
  documented CLI help. At detection time the backend additionally runs
  `<cli> --version` and `<cli> --help`, stores a 4,000-character help excerpt, extracts
  advertised subcommands by token matching, and writes a capability JSON object:
  `launch`, `openProject`, `createProject`, `inspectProject`, `executeCommand`,
  `listCommands`, `mcp`, `cliPath`, `advertisedSubcommands`, `allowedSubcommands`,
  `helpHead`, `notes`, `verified`.
- **Persistence:** upsert into `applications` on `app_key`; previously known apps whose
  executable is gone are marked `status='missing'`.
- **Execution:** `Command::new(exe).args([...])` only — never a shell.
  `execute_app_command` accepts a subcommand allowlist of twelve read-only verbs
  (`info`, `inspect`, `text`, `commands`, `describe`, `probe`, `cat`, `eval`, `tools`,
  `version`, `--version`, `--help`), rejects newlines/NUL and over-long arguments, and
  requires any path-like argument to resolve inside the library.
- **Writing operations** (render, export, batch, script) are deliberately **not** exposed;
  they are reported as unsupported rather than attempted.

### 2.6 Backup and restore

Implemented in `commands/maintenance.rs`.

- **Backup:** `VACUUM INTO <backup_dir>/storyops-YYYYmmdd-HHMMSS.db` (unique name; the
  default directory is `<root>/Backups`, overridable through `backupDirectory`). Records
  size, kind, note and timestamp in `backups`, updates `lastBackupAt`, logs activity.
- **Restore:** validates the backup path is inside the library, takes an automatic
  `pre-restore` backup of the live database, then uses `rusqlite::backup::Backup` with
  `run_to_completion(200, 5ms, None)` to copy the file into the open connection. Foreign
  keys are re-enabled and `migrate()` runs again. The live file is never replaced on disk
  behind SQLite's back, and the pre-restore copy is returned in the success message.
- **Integrity:** `PRAGMA integrity_check`, episode folder existence, registered-file
  existence, `episode.json` presence/ownership, and orphan counting by walking
  `Projects/` three levels deep.
- **Manifest repair:** rewrites every episode manifest and reports the count.

---

## 3. DATABASE

SQLite, embedded through `rusqlite` 0.32 with the `bundled` and `backup` features.
WAL journaling, foreign keys enforced.

### 3.1 Tables (19)

| Table | Purpose | Key columns |
|---|---|---|
| `settings` | Key/value application settings | `key` PK |
| `projects` | Library container | `id` PK |
| `episodes` | Content projects | `id` PK, `code` UNIQUE |
| `episode_history` | Status transition log | `id` PK, `episode_id` FK |
| `ideas` | Idea repository | `id` PK |
| `scripts` | Script documents | `id` PK, `episode_id` FK |
| `script_versions` | Immutable script snapshots | `id` PK, `(script_id, version)` UNIQUE |
| `prompt_templates` | Reusable prompt text | `id` PK, `builtin` flag |
| `narrations` | Audio recordings and versions | `id` PK, `episode_id` FK |
| `files` | Central file registry | `id` PK, `path` UNIQUE, `episode_id` FK |
| `file_versions` | Detected external changes | `id` PK, `file_id` FK |
| `applications` | Detected ArtCraft apps | `id` PK, `app_key` UNIQUE |
| `editing_projects` | Native projects per episode | `id` PK, `episode_id` FK |
| `exports` | Rendered outputs | `id` PK, `episode_id` FK |
| `platforms` | Publishing destinations | `id` PK, `key` UNIQUE |
| `publishing` | Publishing records/schedules | `id` PK, `episode_id` FK, `platform_id` FK |
| `analytics` | Performance snapshots | `id` PK, `publishing_id` FK |
| `activity_logs` | Human-readable audit trail | `id` PK |
| `backups` | Backup history | `id` PK |

Indexes exist on episode status/archived, history by episode, idea status/category, script
episode, file episode/kind, file-version lookup, publishing status/scheduled date,
analytics capture date, and activity timestamp.

### 3.2 Relationships and foreign keys

| Child | Parent | On delete |
|---|---|---|
| `episodes.project_id` | `projects.id` | `SET NULL` |
| `episode_history.episode_id` | `episodes.id` | `CASCADE` |
| `scripts.episode_id` | `episodes.id` | `SET NULL` |
| `script_versions.script_id` | `scripts.id` | `CASCADE` |
| `narrations.episode_id` | `episodes.id` | `SET NULL` |
| `files.episode_id` | `episodes.id` | `SET NULL` |
| `file_versions.file_id` | `files.id` | `CASCADE` |
| `editing_projects.episode_id` | `episodes.id` | `CASCADE` |
| `exports.episode_id` | `episodes.id` | `CASCADE` |
| `publishing.episode_id` | `episodes.id` | `CASCADE` |
| `publishing.platform_id` | `platforms.id` | `CASCADE` |
| `analytics.publishing_id` | `publishing.id` | `CASCADE` |

**Deliberately unconstrained columns:** `editing_projects.app_id`, `exports.file_id`,
`publishing.export_id`, `analytics.platform_id`, `analytics.episode_id`,
`ideas.converted_episode_id`. These hold IDs but declare no foreign key, so deleting the
referenced row leaves a dangling value. This is a data-integrity risk, not a crash risk.

### 3.3 Tables that are implemented but underused

- **`projects`** — one row is inserted at startup. No command reads or updates it; there is
  no project-management UI. It exists as a container placeholder for the `episodes`
  foreign key.
- **`editing_projects`** — written by `create_project` (connector) but **no command reads
  it**. Native projects therefore do not appear anywhere in the UI yet.
- **`file_versions`** — written by `scan_external_changes` and `relink_file`; read by
  `file_versions`. Rows are only created for detected size/mtime changes, so a fresh
  install has none.

All other tables are read and written by at least one command.

### 3.4 Migration status

Version `1`, applied once. Seeding is idempotent. No migration tests exist. There is no
mechanism for schema evolution beyond bumping `SCHEMA_VERSION` and appending DDL — the
current code always runs the same batch, so a future `ALTER TABLE` would need code changes
that are not yet present.

---

## 4. ARTCRAFT INTEGRATIONS

### 4.1 Applications in the catalogue

The catalogue is a static list of 14 applications. Detection is dynamic, so the set of
`applications` rows depends on what is installed.

| App key | Catalogue name | Project extensions | create | open | inspect |
|---|---|---|---|---|---|
| `filmcraft` | FilmCraft | `fcproj` | yes | yes | yes |
| `wordcraft` | WordCraft | docx, doc, odt, rtf, md, txt, pdf, html, tex, json, png, xml | no | no | yes |
| `pdfcraft` | PdfCraft | pdf | no | no | yes |
| `photocraft` | PhotoCraft | pcraft, psd, png, jpg, jpeg, tif, tiff, webp, exr | no | no | yes |
| `soundcraft` | SoundCraft | scraft | no | no | yes |
| `effectcraft` | EffectCraft | — | no | no | no |
| `deckcraft` | DeckCraft | deckcraft, pptx | no | no | yes |
| `designcraft` | DesignCraft | — | no | no | no |
| `figcraft` | FigCraft | figcraft, penpot, json, fig, sketch, svg | no | no | yes |
| `cadcraft` | CADCraft | dwg, dxf, dwt, svg | no | no | yes |
| `gridcraft` | GridCraft | xlsx, csv, tsv, json, html, pdf | no | no | yes |
| `vectorcraft` | VectorCraft | ait | no | no | no |
| `lightcraft` | LightCraft | — | no | no | no |
| `solidcraft` | SolidCraft | sc, step, stp | no | no | yes |

Only `filmcraft` is marked `"verified": true` in the stored capability JSON.

**Installed applications observed on this machine (outside the app):** 14 directories
under `D:\` — CADCraft, DeckCraft, DesignCraft, EffectCraft, FigCraft, FilmCraft,
GridCraft, LightCraft, PdfCraft, PhotoCraft, SolidCraft, SoundCraft, VectorCraft,
WordCraft. An unrelated binary-analysis tool, Craft Anything, is installed separately
under `C:\Program Files`. The in-app detector has **not** been executed, so the persisted
`applications` table has never been populated.

### 4.2 Detection mechanism

1. Build candidate roots: every existing drive `C:`–`G:`, `%ProgramFiles%`,
   `%ProgramFiles(x86)%`, `%LOCALAPPDATA%\Programs`.
2. Read one directory level; keep entries that are directories and whose lowercased name
   ends with `craft`; skip `craft anything`.
3. Require an executable whose stem equals the folder name (or `artcraft-launcher`).
4. Capture `<stem>-cli.exe` as the automation binary when present.
5. Run `--version` and `--help` through the CLI (never the GUI) and derive advertised
   subcommands by token matching.
6. Upsert into `applications`; mark previously known apps `missing`.

Limitations of this mechanism: only one directory level is scanned (no nested install
layouts); an app installed under a name that does not end in `Craft` is invisible; the
launcher-managed copies under `%LOCALAPPDATA%\Programs\ArtCraft\Launcher` are not
editors and are not registered.

### 4.3 Per-connector supported commands

All connectors with a CLI expose the same allow-listed read surface through
`execute_app_command`; the table below records what each app's own help documents and what
the connector therefore advertises.

| App | Documented CLI surface (from `--help`) | Connector operations |
|---|---|---|
| FilmCraft | `exec`, `run`, `commands`, `inspect`, `import`, `export`, `render`, `probe`, `mcp`, `--bridge` control port | launch, open project by path, create project, inspect project JSON, run allow-listed subcommands, list commands |
| WordCraft | `convert`, `info`, `text`, `inspect`, `render`, `run` | launch, info/inspect/text via allowlist, list commands |
| PdfCraft | `info`, `render`, `text`, `edit`, `combine`, `extract`, `split`, `check`, `audit`, `batch`, `tools`, `run`, `mcp` | launch, info/text via allowlist, list commands |
| PhotoCraft | `convert`, `info`, `run`, `batch`, `droplet`, `commands`, `mcp`, `serve` | launch, info via allowlist, list commands |
| SoundCraft | `info`, `convert`, `run`, `script`, `commands`, `describe`, `app`, `mcp`, `plugins`, `parity` | launch, info/describe via allowlist, list commands |
| EffectCraft | `info`, `commands`, `exec`, `run`, `props`, `get`, `set`, `render-frame`, `render`, `script` | launch, info via allowlist, list commands |
| DeckCraft | `info`, `render`, `render-transition`, `convert`, `run`, `commands`, `describe`, `app`, `mcp` | launch, info via allowlist, list commands |
| DesignCraft | `run`, `commands`, `describe`, `script`, `app`, `mcp`, `perf`, `bench`, `validate-pdf` | launch, describe via allowlist, list commands |
| FigCraft | `info`, `export`, `convert`, `render`, `run`, `commands`, `mcp`, `plugin`, `connect` | launch, info via allowlist, list commands |
| CADCraft | `info`, `convert`, `run`, `commands`, `sample`, `perf`, `mcp` | launch, info via allowlist, list commands |
| GridCraft | `info`, `convert`, `eval`, `cat`, `check`, `run`, `commands`, `functions`, `mcp` | launch, info/cat/eval via allowlist, list commands |
| VectorCraft | `run`, `mcp` | launch, info via allowlist, list commands |
| LightCraft | `run`, `mcp` | launch, info via allowlist, list commands |
| SolidCraft | `run`, `eval`, `script`, `snapshot`, `bench-open` | launch, eval via allowlist, list commands |

Writing subcommands are intentionally absent from `ALLOWED_SUBCOMMANDS`. Rendering,
exporting and batch processing must be done in the application itself.

### 4.4 Tested versus untested

**Tested directly against the installed CLIs (outside the app, before the backend existed):**

- `filmcraft-cli --version` → `filmcraft-cli 0.16.0`.
- `filmcraft-cli commands --json` → JSON array of command objects with `id`, `label`,
  `params`.
- `filmcraft-cli --save-as <path> exec file.newProject name=StoryOpsTest` → created a
  727-byte `.fcproj`; returned `{ "name": "StoryOpsTest", "revision": 2 }`.
- `filmcraft-cli --project <path> inspect project` → returned project JSON with `name`,
  `path`, `revision`, `root`.
- `filmcraft.exe --help` → `Usage: filmcraft [options] [project.fcproj | media files...]`,
  confirming a positional project argument.
- Version checks only: `wordcraft-cli 0.14.0`, `soundcraft-cli 0.16.0`,
  `pdfcraft-cli 0.17.0`.

**Untested (implemented, compiled, never executed):**

- The detector (`detect_applications`) against this machine.
- `launch_application`, `open_project`, `create_project`, `execute_app_command`,
  `list_app_commands`, `inspect_project` through the Tauri command layer.
- Every other application's CLI beyond `--version`.
- Any AK/Junction/symlink edge case in discovery.

**Not verified at all:** whether opening a `.fcproj` from StoryOps brings an already
running FilmCraft instance to the foreground, and whether any app tolerates being passed a
project path when it is already running.

### 4.5 Native project creation and opening

- **Creation (FilmCraft only):** `create_project` writes
  `--save-as <episode>/Editing/<slug>.fcproj exec file.newProject name=<name>`, verifies
  exit code 0, registers the file, inserts an `editing_projects` row and logs activity.
  Other applications return an explicit unsupported message directing the user to create
  the project in the app and register it.
- **Opening (FilmCraft only):** `open_project` validates the path is inside the library
  and spawns `filmcraft.exe <project>` detached. Other applications return an unsupported
  message rather than guessing.

### 4.6 Integration limitations

- No MCP client and no control-bridge client. `mcp` and `--bridge` appear only as
  advertised capability flags; nothing connects to them.
- No project metadata reading beyond FilmCraft `inspect`; the other apps' `info`
  subcommands are reachable only through the generic allow-listed runner, which returns
  raw stdout, not a normalised model.
- No file-watching integration with running apps; external changes are found by the
  manual scan described in §2.4/§8.
- Capability catalogue is code, not data; adding an app requires editing Rust.
- No timeout on child processes. A hung CLI would block the invoking thread.

---

## 5. FRONTEND

### 5.1 Implemented components

`src/components/ui.tsx` (388 lines): `Button`, `Card`, `PanelTitle`, `Field`, `Input`,
`Textarea`, `Select`, `Badge`, `ProgressBar`, `StatCard`, `EmptyState`, `Loading`,
`ErrorNote`, `Modal`, `ConfirmDialog`, `Tabs`, `Toaster`, `toast()`, `useAsyncAction()`,
`cx()`.

`src/components/charts.tsx` (180 lines): `LineChart` (area + line, per-point tooltips),
`BarList`, `Donut`, `MiniBars` — all hand-written SVG, no chart dependency.

`src/lib/format.ts` (96 lines): relative time, date/date-time, duration, bytes, compact
numbers, percent, date-input conversion, days-until, overdue check.

`src/styles.css` (328 lines): Tailwind 4 import, light/dark CSS variables (charcoal,
near-black, restrained coral `#e05a43`), `@theme inline` token mapping, and shared classes
(`.card`, `.btn`, `.input`, `.table`, `.nav-item`, `.kpi-value`, `.badge`, scrollbars,
skeleton shimmer).

### 5.2 Pages

**None are implemented.** There is no `App.tsx`, no router, no layout/sidebar component,
and no page module. The eleven required destinations — Dashboard, Content Pipeline,
Ideas Library, Scripts, Narration, Production Studio, File Library, Publishing, Analytics,
App Connections, Settings — do not exist as React code.

### 5.3 Typed bridge

`src/lib/bridge.ts` (248 lines) wraps `invoke` from `@tauri-apps/api/core` in `call<T>()`,
which normalises thrown strings/Errors into `Error`. The exported `api` object mirrors the
backend surface by domain:

`settings`, `dashboard`, `activity`, `openExternal`, `reveal`, `episodes`, `ideas`,
`scripts`, `templates`, `narrations`, `files`, `publishing`, `analytics`, `maintenance`,
`connectors`.

Every method names its Rust command and passes camelCase argument keys, which Tauri maps
to the snake_case Rust parameters. Return types are the interfaces in `src/lib/types.ts`
(495 lines), which mirror the `serde` DTOs field-for-field, including `Status`,
`STATUS_LABELS`, `CATEGORIES`, `Checklist` and `Capabilities`.

`isDesktop()` checks for `__TAURI_INTERNALS__`. Nothing yet consumes it.

### 5.4 Incomplete frontend functionality

- No page components, no navigation, no application shell.
- No theme switch wired to `set_setting("theme", …)`, despite tokens supporting both modes.
- No drag-and-drop Kanban implementation.
- No audio player wiring (`read_file_data` → Blob URL is designed but unused).
- No dialogs wired to `@tauri-apps/plugin-dialog`.
- No data loading, caching, refresh or error surface beyond the primitives.
- `noUnusedLocals`/`noUnusedParameters` are enabled in `tsconfig.json`; unreferenced
  exports in the current files may still typecheck, but this has not been run.

### 5.5 Known frontend breakage

`src/main.tsx` is a **single line of malformed text** (untracked in git at the time of this
review):

```
﻿-import { createRoot } from 'react-dom/client'; -import { App } from ./components/ui; ...
```

It contains a BOM, literal `-` characters instead of newlines, an unquoted import path,
and imports an `App` export that `components/ui.tsx` does not provide. It cannot compile.
`index.html` points at this file, so `npm run build` and `npm run dev` will fail until it
is replaced.

---

## 6. BUILD AND EXECUTION

### 6.1 Required software

| Requirement | Version used | Notes |
|---|---|---|
| Node.js | 24.18.0 | npm 11.16.0 |
| Rust | 1.99.1 | `stable-x86_64-pc-windows-gnu` is the default toolchain |
| MinGW-w64 | GCC 16.2.0 (winlibs, portable) | at `~/toolchain/mingw64`; must be on `PATH` |
| WebView2 | present on Windows 11 | runtime dependency for the Tauri window |
| Visual Studio C++ tools | **not usable** | VS 2026 installed without the C++ workload or Windows SDK |

The MSVC toolchain cannot link this project on the current machine: `kernel32.lib` and the
rest of the Windows SDK import libraries are absent. The build therefore targets
`x86_64-pc-windows-gnu`.

### 6.2 Development commands

```bash
export PATH="$HOME/toolchain/mingw64/bin:$HOME/.cargo/bin:$PATH"

npm install            # frontend dependencies (already installed)
npm run dev            # Vite dev server on http://localhost:1420
npm run desktop:dev    # tauri dev: Vite + native window
```

### 6.3 Build commands

```bash
npm run build                  # tsc -b && vite build  (currently fails; see §5.5)
cd src-tauri && cargo build    # builds target/debug/storyops.exe
npm run desktop:build          # tauri build (NSIS bundle) — not yet attempted
```

### 6.4 Current build status

| Artifact | Status |
|---|---|
| `cargo check` | **exit 0**, no errors |
| `cargo build` | **exit 0**, `src-tauri/target/debug/storyops.exe` produced (244,795,775 bytes, debug) |
| `dist/` frontend bundle | **absent** — never built |
| `storyops.exe` launch | **not attempted** |
| NSIS installer | **not attempted** |

Rust warnings at build time: seven unused-function warnings (`touch_episode`,
`now_stamp`, `append_log`, `today`, `sha256_file`, `human_size`, `friendly_err`) and one
GNU linker notice: `.rsrc merge failure: multiple non-default manifests`. The latter is
non-fatal on this toolchain.

### 6.5 Known environment issues

- `cargo` resolves MSYS `link.exe` (coreutils) unless the GNU toolchain is default; the
  build fails with "link: extra operand". Verify with `rustc -vV | grep host`.
- Building the crate as a `cdylib` fails on MinGW with
  `error: export ordinal too large: 100344`. `Cargo.toml` therefore declares
  `crate-type = ["staticlib", "rlib"]`.
- `npm install` warns that esbuild's postinstall was not approved; the platform binary is
  present and functional.
- The ArtCraft Launcher under `%LOCALAPPDATA%\Programs\ArtCraft\Launcher` is a separate
  product and is not an integration target.

---

## 7. TESTING

### 7.1 Tests actually executed

| Check | Command | Result |
|---|---|---|
| Type/borrow check | `cargo check --message-format short` | exit 0, no errors |
| Debug build | `cargo build --message-format short` | exit 0, binary produced |
| Isolated Tauri link probe | throwaway crate depending on `tauri` 2.12.5 | linked successfully (used to validate the toolchain before writing the app) |

No unit tests, integration tests, or end-to-end tests exist. `cargo test` has never been
run because no `#[test]` functions are defined.

### 7.2 Build verification results

The Rust library and binary compile and link. The frontend has never been built: `dist/`
does not exist, and `src/main.tsx` is syntactically invalid, so `npm run build` would fail
immediately.

### 7.3 Features tested against real applications

Only the FilmCraft CLI interactions listed in §4.4, executed directly from the shell.
They prove the underlying CLI semantics (project creation, inspection, version/help) — not
that StoryOps' command wrappers work. The wordcraft/soundcraft/pdfcraft checks were
version-only.

### 7.4 Compiled but never runtime-tested

Everything else:

- Database creation, migration, seeding and every SQL statement.
- All 85 Tauri commands.
- Episode/idea/script/narration/file/publishing/analytics workflows.
- Backup, restore, integrity report, manifest repair.
- Application detection and every connector operation through the app.
- File import/copy/move/delete/rename and path containment in practice.
- External-change detection.
- The entire React layer.

---

## 8. SECURITY AND RELIABILITY

### 8.1 Filesystem restrictions

- A single configurable library root; `norm()` canonicalises when possible and resolves
  `..` lexically otherwise; `is_within()` compares normalised paths.
- Enforced on: `open_external`, `reveal_in_explorer`, `register_file`, `import_file`
  destinations, `create_folder`, `rename_entry`, `move_entry`, `copy_entry`,
  `delete_entry`, `path_metadata`, `episode_tree`, `relink_file`, `read_file_data`,
  connector `open_project`/`inspect_project`, and path-like connector arguments.
- `import_script_file` reads a user-chosen path outside the library but only `.md`,
  `.markdown`, `.txt`, `.text`, `.fountain` under 5 MB; `import_file`/`import_narration`
  likewise only copy externally chosen sources inward.
- Windows-reserved characters are rejected in new names; existing targets are never
  overwritten.

### 8.2 Shell execution permissions

- No shell is used anywhere. External programs are spawned with
  `std::process::Command::new(exe).args([...])`.
- The only executables that can be started are: the app executables registered in
  `applications`, their CLIs, and `explorer.exe`/`/select,`.
- CLI execution is gated by `ALLOWED_SUBCOMMANDS` (twelve read-only verbs). Anything else
  returns an error. Newlines, NULs and >4 KB arguments are rejected.
- `create_project` builds its argument vector itself; the user-supplied project name is
  passed as a single `name=<value>` argument and cannot inject flags.
- There is no generic "run this command" entry point exposed to the webview.

### 8.3 Database backup safety

- Backups use `VACUUM INTO`, producing a consistent snapshot while the DB is live.
- Backup filenames are timestamped and uniquified; existing files are never overwritten.
- Restore always creates a `pre-restore` safety copy first and copies through SQLite's
  backup API rather than replacing the file on disk.
- Restore refuses paths outside the library.
- Deletions of files and episodes move content to `Backups/Deleted` instead of unlinking.

### 8.4 Error handling

- Every command returns `Result<T, String>`; messages are user-facing sentences.
- Integrity and connector failures are also written to `activity_logs`, and startup
  writes to `<app_data>/logs/storyops.log`.
- `append_log` exists but is not called by any command, so most runtime errors are **not**
  written to the log file — only activity-log entries.
- Frontend normalization is implemented in `bridge.ts` but has no consumer yet.

### 8.5 Known security and reliability risks

1. **TOCTOU:** containment is checked before the filesystem operation; a path swapped
   between check and use (e.g. a junction created concurrently) could escape the root.
2. **Junction/symlink traversal:** `canonicalize` is used when the path exists, but a
   directory junction placed inside the library can still redirect writes outside it.
3. **Subprocess timeouts:** none. A hung ArtCraft CLI blocks the command thread.
4. **Memory:** `read_file_data` base64-encodes up to 80 MB into memory, roughly 107 MB as
   a string, and the webview then holds a Blob copy.
5. **Webview permissions:** `core:default`, `dialog:default` and `opener:default` are
   granted. `opener:default` permits the webview to open URLs/paths directly, bypassing
   the Rust containment checks. If the renderer is ever compromised (e.g. by pasted
   assistant output rendered as HTML), this widens impact.
6. **No content sanitisation:** pasted AI responses are intended to be stored as plain
   text. No Markdown/HTML renderer exists yet, so no XSS path is currently reachable, but
   any future renderer must sanitise.
7. **Unencrypted database:** SQLite holds titles, notes and paths in clear text. No
   secrets are stored, and no credential or API-key fields exist.
8. **Dangling IDs:** the unconstrained columns in §3.2 can leave orphan references that
   the UI must tolerate.
9. **No concurrency control:** one `Mutex<Connection>` serialises all DB work; a long
   operation blocks every other command.
10. **Foreign function surface:** 85 synchronous commands can block Tauri worker threads;
    none are async.

---

## 9. CURRENT LIMITATIONS

### 9.1 Incomplete functionality

- **The application cannot start usefully.** There is no `dist/`, no `App`, no pages, and
  `main.tsx` does not parse. A launch today would produce a window with no interface.
- Eleven required pages are entirely absent.
- No navigation, sidebar, routing, or theme switch UI.
- No Kanban drag-and-drop, calendar view, or search/filter UI.
- No audio playback or duration reporting from the frontend.
- No prompt-template management UI (backend is complete).
- No connector management UI (backend is complete).
- `editing_projects` has no read command, so native projects are invisible to any future
  UI.
- No `docs/` set beyond this report; the specification's README and architecture documents
  are not written.

### 9.2 Temporary workarounds

- MinGW/GNU toolchain instead of the documented MSVC requirement, forced by the missing
  Windows SDK.
- `crate-type` reduced to `staticlib` + `rlib` to avoid the GNU cdylib ordinal overflow.
- Media previews use base64 data URLs rather than Tauri's asset protocol.
- External modification detection is an on-demand scan, not a filesystem watcher.
- WAV duration/sample-rate is parsed in Rust; other formats rely on a future frontend
  report.

### 9.3 Known bugs

- `src/main.tsx` is malformed (BOM, literal `-` separators, unquoted path, missing `App`).
- `set_setting("backupDirectory", …)` requires the directory to be inside the library,
  which contradicts the idea of an arbitrary backup location; the restriction is enforced
  deliberately but limits the setting.
- `update_narration` calls `write_manifest` with `unwrap_or_default()` on the episode ID,
  so a narration without an episode writes a manifest for an empty ID and fails silently.
- Seven unused functions generate warnings.
- The linker emits a duplicate-manifest warning on GNU; output still links.

### 9.4 Unverified integrations

- Application detection on this machine.
- All 13 non-FilmCraft connectors.
- FilmCraft launch/open/create/inspect **through StoryOps** (the underlying CLI was
  tested directly).
- MCP and control-bridge features (advertised only, not implemented).
- Any behaviour when an app is running and receives a second project argument.

---

## 10. RECOMMENDED NEXT ACTION

**The single blocking item is the frontend.** The backend is complete enough to support
the required workflows and demonstrably compiles and links; what prevents the initial
application from running is that no React application exists.

Minimum path to a runnable application, in order:

1. **Replace `src/main.tsx`** with a valid entry that imports `./styles.css`, mounts React
   via `createRoot`, and renders an `App` component.
2. **Create `src/App.tsx`** with the eleven-item sidebar, page switching, theme toggle
   persisted through `settings.set("theme", …)`, and the `Toaster`.
3. **Implement the eleven pages** against the existing `api` bridge, starting with
   Dashboard (real counts and empty states), Content Pipeline (Kanban over the ten
   statuses), and Settings (root directory, backups, integrity).
4. **Run `npm run build`**, fix type errors, then `npm run desktop:dev` and confirm the
   window opens, the database initialises at the configured root, navigation works, and a
   create-episode → move-status → restart cycle persists.
5. Only after that, exercise the connectors through the UI and correct the capability
   flags against observed behaviour.

Until steps 1–4 are complete, the project should be described as **a compiled backend with
a component library and typed bridge, not a runnable desktop application**.
