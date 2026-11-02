import { invoke } from "@tauri-apps/api/core";
import type {
  Activity,
  AnalyticsInput,
  AnalyticsRow,
  AppInfo,
  Application,
  BackupRow,
  ChangeReport,
  CommandResult,
  Dashboard,
  Episode,
  EpisodeInput,
  EpisodeTree,
  EpisodeUpdate,
  ExportRow,
  FileEntry,
  FileVersion,
  HistoryEntry,
  Idea,
  IdeaInput,
  IntegrityReport,
  LabeledValue,
  Narration,
  PathMeta,
  Platform,
  PromptTemplate,
  Schedule,
  ScheduleInput,
  Script,
  ScriptVersion,
  TemplateInput,
  Totals,
} from "./types";

/** Every backend error arrives as a string; normalise it for the UI. */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const message =
      typeof error === "string"
        ? error
        : error instanceof Error
          ? error.message
          : "The desktop backend returned an unexpected error.";
    throw new Error(message);
  }
}

export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export const api = {
  settings: {
    all: () => call<Record<string, string>>("get_settings"),
    set: (key: string, value: string) => call<void>("set_setting", { key, value }),
    appInfo: () => call<AppInfo>("get_app_info"),
  },
  dashboard: () => call<Dashboard>("get_dashboard"),
  activity: (limit = 100) => call<Activity[]>("list_activity", { limit }),
  openExternal: (path: string) => call<void>("open_external", { path }),
  reveal: (path: string) => call<void>("reveal_in_explorer", { path }),

  episodes: {
    list: (includeArchived = false) => call<Episode[]>("list_episodes", { includeArchived }),
    get: (id: string) => call<Episode>("get_episode", { id }),
    create: (input: EpisodeInput) => call<Episode>("create_episode", { input }),
    update: (input: EpisodeUpdate) => call<Episode>("update_episode", { input }),
    setStatus: (id: string, status: string, note?: string) =>
      call<Episode>("set_episode_status", { id, status, note: note ?? null }),
    archive: (id: string, archived: boolean) => call<void>("archive_episode", { id, archived }),
    remove: (id: string) => call<void>("delete_episode", { id }),
    history: (episodeId: string) => call<HistoryEntry[]>("list_episode_history", { episodeId }),
  },

  ideas: {
    list: () => call<Idea[]>("list_ideas"),
    create: (input: IdeaInput) => call<Idea>("create_idea", { input }),
    update: (id: string, input: IdeaInput) => call<Idea>("update_idea", { id, input }),
    remove: (id: string) => call<void>("delete_idea", { id }),
    convert: (id: string, category?: string) =>
      call<Episode>("convert_idea_to_episode", { id, category: category ?? null }),
  },

  scripts: {
    list: () => call<Script[]>("list_scripts"),
    get: (id: string) => call<Script>("get_script", { id }),
    create: (title: string, episodeId?: string | null, content?: string) =>
      call<Script>("create_script", {
        title,
        episodeId: episodeId ?? null,
        content: content ?? null,
      }),
    update: (id: string, title: string, content: string, note?: string) =>
      call<Script>("update_script", { id, title, content, note: note ?? null }),
    remove: (id: string) => call<void>("delete_script", { id }),
    versions: (scriptId: string) => call<ScriptVersion[]>("list_script_versions", { scriptId }),
    versionContent: (versionId: string) => call<string>("get_script_version", { versionId }),
    restore: (scriptId: string, versionId: string) =>
      call<Script>("restore_script_version", { scriptId, versionId }),
    importFile: (path: string, episodeId?: string | null) =>
      call<Script>("import_script_file", { path, episodeId: episodeId ?? null }),
    exportFile: (id: string, path: string) => call<string>("export_script", { id, path }),
  },

  templates: {
    list: () => call<PromptTemplate[]>("list_prompt_templates"),
    save: (id: string | null, input: TemplateInput) =>
      call<PromptTemplate>("save_prompt_template", { id, input }),
    remove: (id: string) => call<void>("delete_prompt_template", { id }),
    render: (templateId: string, episodeId?: string | null, values?: Record<string, string>) =>
      call<string>("render_prompt", {
        templateId,
        episodeId: episodeId ?? null,
        values: values ?? null,
      }),
  },

  narrations: {
    list: () => call<Narration[]>("list_narrations"),
    import: (episodeId: string, sourcePath: string, title?: string) =>
      call<Narration>("import_narration", {
        episodeId,
        sourcePath,
        title: title ?? null,
      }),
    update: (id: string, title: string, notes: string, status: string, durationSec?: number) =>
      call<Narration>("update_narration", {
        id,
        title,
        notes,
        status,
        durationSec: durationSec ?? null,
      }),
    replace: (id: string, sourcePath: string) =>
      call<Narration>("replace_narration", { id, sourcePath }),
    remove: (id: string) => call<void>("delete_narration", { id }),
  },

  files: {
    list: (filters?: {
      episodeId?: string;
      kind?: string;
      appId?: string;
      search?: string;
      includeMissing?: boolean;
    }) =>
      call<FileEntry[]>("list_files", {
        episodeId: filters?.episodeId ?? null,
        kind: filters?.kind ?? null,
        appId: filters?.appId ?? null,
        search: filters?.search ?? null,
        includeMissing: filters?.includeMissing ?? true,
      }),
    import: (episodeId: string, sourcePath: string, kind?: string) =>
      call<FileEntry>("import_file", { episodeId, sourcePath, kind: kind ?? null }),
    register: (path: string, episodeId?: string | null, kind?: string) =>
      call<FileEntry>("register_file", {
        path,
        episodeId: episodeId ?? null,
        kind: kind ?? null,
      }),
    createText: (episodeId: string, name: string, content?: string) =>
      call<FileEntry>("create_text_file", { episodeId, name, content: content ?? null }),
    createFolder: (parentPath: string, name: string) =>
      call<string>("create_folder", { parentPath, name }),
    rename: (path: string, newName: string) => call<string>("rename_entry", { path, newName }),
    move: (path: string, destinationDir: string) =>
      call<string>("move_entry", { path, destinationDir }),
    copy: (path: string, destinationDir: string) =>
      call<string>("copy_entry", { path, destinationDir }),
    remove: (path: string) => call<void>("delete_entry", { path }),
    meta: (path: string) => call<PathMeta>("path_metadata", { path }),
    tree: (episodeId: string) => call<EpisodeTree>("episode_tree", { episodeId }),
    versions: (fileId: string) => call<FileVersion[]>("file_versions", { fileId }),
    relink: (fileId: string, newPath: string) => call<FileEntry>("relink_file", { fileId, newPath }),
    scan: () => call<ChangeReport>("scan_external_changes"),
    data: (path: string) => call<string>("read_file_data", { path }),
  },

  publishing: {
    platforms: () => call<Platform[]>("list_platforms"),
    savePlatform: (id: string | null, name: string, enabled: boolean) =>
      call<Platform>("save_platform", { id, name, enabled }),
    schedules: (episodeId?: string | null) =>
      call<Schedule[]>("list_schedules", { episodeId: episodeId ?? null }),
    saveSchedule: (input: ScheduleInput) => call<Schedule>("save_schedule", { input }),
    removeSchedule: (id: string) => call<void>("delete_schedule", { id }),
    markPublished: (id: string, postUrl?: string | null) =>
      call<Schedule>("mark_published", { id, postUrl: postUrl ?? null }),
    exports: (episodeId?: string | null) =>
      call<ExportRow[]>("list_exports", { episodeId: episodeId ?? null }),
    registerExport: (episodeId: string, path: string, notes?: string) =>
      call<ExportRow>("register_export", { episodeId, path, notes: notes ?? null }),
    setExportStatus: (id: string, status: string) =>
      call<void>("update_export_status", { id, status }),
    removeExport: (id: string) => call<void>("delete_export", { id }),
  },

  analytics: {
    list: (filters?: { from?: string; to?: string; platformId?: string; episodeId?: string }) =>
      call<AnalyticsRow[]>("list_analytics", {
        from: filters?.from ?? null,
        to: filters?.to ?? null,
        platformId: filters?.platformId ?? null,
        episodeId: filters?.episodeId ?? null,
      }),
    save: (input: AnalyticsInput) => call<AnalyticsRow>("save_analytics", { input }),
    remove: (id: string) => call<void>("delete_analytics", { id }),
    summary: (from?: string, to?: string) =>
      call<Totals>("analytics_summary", { from: from ?? null, to: to ?? null }),
    categories: () => call<LabeledValue[]>("category_performance"),
  },

  maintenance: {
    backup: (note?: string) => call<BackupRow>("backup_database", { note: note ?? null }),
    backups: () => call<BackupRow[]>("list_backups"),
    restore: (id: string) => call<string>("restore_backup", { id }),
    integrity: () => call<IntegrityReport>("database_integrity"),
    repairManifests: () => call<number>("repair_manifests"),
    logs: (lines = 200) => call<string[]>("log_tail", { lines }),
  },

  connectors: {
    detect: () => call<Application[]>("detect_applications"),
    list: () => call<Application[]>("list_applications"),
    test: (id: string) => call<Application>("test_connection", { id }),
    launch: (id: string) => call<number>("launch_application", { id }),
    openProject: (appId: string, path: string) => call<number>("open_project", { appId, path }),
    createProject: (appId: string, episodeId: string, name: string) =>
      call<{ path: string; fileId: string; editingProjectId: string; stdout: string }>(
        "create_project",
        { appId, episodeId, name },
      ),
    run: (appId: string, args: string[]) =>
      call<CommandResult>("execute_app_command", { appId, args }),
    commands: (appId: string) => call<{ app: string; raw: string; commands: unknown }>(
      "list_app_commands",
      { appId },
    ),
    inspect: (appId: string, path: string) =>
      call<unknown>("inspect_project", { appId, path }),
  },
};

export type Api = typeof api;
