export const STATUSES = [
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
] as const;

export type Status = (typeof STATUSES)[number];

export const STATUS_LABELS: Record<Status, string> = {
  idea: "Idea",
  research: "Research",
  writing: "Writing",
  review_script: "Review Script",
  narration: "Narration",
  editing: "Editing",
  final_review: "Final Review",
  ready: "Ready",
  scheduled: "Scheduled",
  published: "Published",
};

export const CATEGORIES = [
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
] as const;

export type Priority = "high" | "medium" | "low";

export interface Episode {
  id: string;
  projectId: string | null;
  code: string;
  title: string;
  description: string;
  category: string;
  tags: string[];
  status: Status;
  statusLabel: string;
  priority: Priority;
  dueDate: string | null;
  progress: number;
  archived: boolean;
  folder: string;
  createdAt: string;
  updatedAt: string;
  scriptCount: number;
  fileCount: number;
  narrationCount: number;
}

export interface EpisodeInput {
  title: string;
  description?: string;
  category?: string;
  tags?: string[];
  priority?: string;
  dueDate?: string | null;
  status?: string;
}

export interface EpisodeUpdate {
  id: string;
  title: string;
  description?: string;
  category?: string;
  tags?: string[];
  priority?: string;
  dueDate?: string | null;
  archived?: boolean;
}

export interface HistoryEntry {
  id: number;
  episodeId: string;
  fromStatus: string | null;
  toStatus: string;
  note: string;
  createdAt: string;
}

export interface Idea {
  id: string;
  title: string;
  body: string;
  category: string;
  tags: string[];
  priority: Priority;
  status: string;
  notes: string;
  convertedEpisodeId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface IdeaInput {
  title: string;
  body?: string;
  category?: string;
  tags?: string[];
  priority?: string;
  status?: string;
  notes?: string;
}

export interface Script {
  id: string;
  episodeId: string | null;
  episodeTitle: string | null;
  title: string;
  content: string;
  wordCount: number;
  estDurationSec: number;
  currentVersion: number;
  createdAt: string;
  updatedAt: string;
}

export interface ScriptVersion {
  id: string;
  scriptId: string;
  version: number;
  wordCount: number;
  note: string;
  createdAt: string;
}

export interface PromptTemplate {
  id: string;
  name: string;
  category: string;
  instructions: string;
  variables: string[];
  provider: string;
  builtin: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface TemplateInput {
  name: string;
  category: string;
  instructions: string;
  variables?: string[];
  provider?: string;
}

export interface Narration {
  id: string;
  episodeId: string | null;
  episodeTitle: string | null;
  title: string;
  filePath: string;
  format: string;
  durationSec: number;
  sampleRate: number | null;
  channels: number | null;
  bitrate: number | null;
  sizeBytes: number;
  status: string;
  notes: string;
  version: number;
  isCurrent: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface FileEntry {
  id: string;
  episodeId: string | null;
  episodeTitle: string | null;
  kind: string;
  name: string;
  path: string;
  ext: string;
  sizeBytes: number;
  modifiedAt: string | null;
  appId: string | null;
  appName: string | null;
  missing: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface FileVersion {
  id: number;
  fileId: string;
  path: string;
  sizeBytes: number;
  modifiedAt: string | null;
  hash: string | null;
  note: string;
  createdAt: string;
}

export interface TreeEntry {
  name: string;
  path: string;
  isDir: boolean;
  sizeBytes: number;
  modifiedAt: string | null;
  ext: string;
  registeredId: string | null;
  children: TreeEntry[];
}

export interface EpisodeTree {
  path: string;
  entries: TreeEntry[];
}

export interface PathMeta {
  path: string;
  name: string;
  isDir: boolean;
  sizeBytes: number;
  modifiedAt: string | null;
  createdAt: string | null;
  ext: string;
  registeredId: string | null;
  episodeId: string | null;
  exists: boolean;
}

export interface Capabilities {
  launch?: boolean;
  openProject?: boolean;
  createProject?: boolean;
  inspectProject?: boolean;
  executeCommand?: boolean;
  listCommands?: boolean;
  mcp?: boolean;
  cliPath?: string | null;
  advertisedSubcommands?: string[];
  allowedSubcommands?: string[];
  helpHead?: string | null;
  notes?: string;
  verified?: boolean;
}

export interface Application {
  id: string;
  appKey: string;
  name: string;
  version: string | null;
  installDir: string | null;
  exePath: string | null;
  cliPath: string | null;
  projectExtensions: string[];
  capabilities: Capabilities;
  status: string;
  lastSeen: string | null;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface Platform {
  id: string;
  key: string;
  name: string;
  enabled: boolean;
  config: Record<string, unknown>;
  createdAt: string;
}

export interface Checklist {
  scriptFinal?: boolean;
  narrationApproved?: boolean;
  editExported?: boolean;
  thumbnail?: boolean;
  captions?: boolean;
  description?: boolean;
  hashtags?: boolean;
  finalReview?: boolean;
  [key: string]: boolean | undefined;
}

export interface Schedule {
  id: string;
  episodeId: string;
  episodeTitle: string;
  platformId: string;
  platformName: string;
  title: string;
  description: string;
  caption: string;
  hashtags: string;
  targetAudience: string;
  scheduledAt: string | null;
  status: string;
  exportId: string | null;
  exportName: string | null;
  checklist: Checklist;
  postUrl: string | null;
  publishedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ScheduleInput {
  id?: string | null;
  episodeId: string;
  platformId: string;
  title: string;
  description?: string;
  caption?: string;
  hashtags?: string;
  targetAudience?: string;
  scheduledAt?: string | null;
  status?: string;
  exportId?: string | null;
  checklist?: Checklist;
  postUrl?: string | null;
}

export interface ExportRow {
  id: string;
  episodeId: string;
  episodeTitle: string;
  name: string;
  filePath: string;
  format: string;
  status: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface AnalyticsRow {
  id: string;
  publishingId: string | null;
  platformId: string | null;
  platformName: string | null;
  episodeId: string | null;
  episodeTitle: string | null;
  capturedAt: string;
  views: number;
  likes: number;
  comments: number;
  shares: number;
  watchTimeSec: number;
  avgRetentionPct: number;
  followersGained: number;
  countries: string[];
  source: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface AnalyticsInput {
  id?: string | null;
  publishingId?: string | null;
  platformId?: string | null;
  episodeId?: string | null;
  capturedAt?: string | null;
  views?: number;
  likes?: number;
  comments?: number;
  shares?: number;
  watchTimeSec?: number;
  avgRetentionPct?: number;
  followersGained?: number;
  countries?: string[];
  source?: string;
  notes?: string;
}

export interface Totals {
  views: number;
  likes: number;
  comments: number;
  shares: number;
  watchTimeSec: number;
  avgRetentionPct: number;
  followersGained: number;
  engagementRate: number;
}

export interface Activity {
  id: number;
  kind: string;
  entityType: string | null;
  entityId: string | null;
  message: string;
  createdAt: string;
}

export interface BackupRow {
  id: string;
  path: string;
  sizeBytes: number;
  kind: string;
  note: string;
  createdAt: string;
  exists: boolean;
}

export interface DashboardCounts {
  totalProjects: number;
  ideasAwaiting: number;
  scriptsReady: number;
  narrationsCompleted: number;
  inProduction: number;
  readyForReview: number;
  scheduledPosts: number;
  publishedPosts: number;
}

export interface SeriesPoint {
  date: string;
  value: number;
  secondary: number;
}

export interface LabeledValue {
  label: string;
  value: number;
}

export interface PlatformPerf {
  platform: string;
  posts: number;
  views: number;
  likes: number;
  engagementRate: number;
}

export interface Dashboard {
  counts: DashboardCounts;
  recentActivity: Activity[];
  queue: Episode[];
  upcoming: Schedule[];
  contentPerformance: SeriesPoint[];
  platformPerformance: PlatformPerf[];
  productionProgress: LabeledValue[];
  hasAnalytics: boolean;
  rootPath: string;
  databasePath: string;
  lastBackupAt: string | null;
}

export interface AppInfo {
  name: string;
  version: string;
  tauriVersion: string;
  dataDir: string;
  databasePath: string;
  rootPath: string;
  os: string;
  settings: Record<string, string>;
}

export interface CommandResult {
  ok: boolean;
  stdout: string;
  stderr: string;
  code: number | null;
  data: unknown;
}

export interface IntegrityReport {
  databaseOk: boolean;
  databaseMessage: string;
  episodesChecked: number;
  missingFolders: string[];
  missingFiles: string[];
  orphanFiles: number;
  manifestIssues: string[];
  checkedAt: string;
}

export interface ChangeReport {
  checked: number;
  new: FileEntry[];
  modified: FileEntry[];
  missing: FileEntry[];
  checkedAt: string;
}
