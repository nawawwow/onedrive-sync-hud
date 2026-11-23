export type SyncState =
  | "in_sync"
  | "syncing"
  | "paused"
  | "warning"
  | "error"
  | "offline"
  | "not_running"
  | "unknown";

export type FileTransferState =
  | "syncing"
  | "synced"
  | "pending"
  | "error"
  | "cloud_only";

export interface RecentFileItem {
  path: string;
  fileName: string;
  relativePath: string;
  sizeBytes: number;
  state: FileTransferState;
  updatedAtMs: number;
}

export interface QuotaInfo {
  usedBytes: number;
  totalBytes: number;
  usedPercent: number;
  label: string;
}

export interface QuickActionInfo {
  id: string;
  label: string;
  description: string;
  enabled: boolean;
  kind: string;
}

export interface AccountTelemetry {
  id: string;
  syncRootId: string | null;
  displayName: string;
  isBusiness: boolean;
  folderPath: string;
  state: SyncState;
  stateLabel: string;
  statusSummary: string;
  statusDetail: string | null;
  progressPercent: number | null;
  filesToUpload: number;
  filesToDownload: number;
  bytesUploaded: number;
  bytesToUpload: number;
  bytesDownloaded: number;
  bytesToDownload: number;
  changesToProcess: number;
  uploadSpeedBps: number;
  downloadSpeedBps: number;
  etaSeconds: number | null;
  errorCount: number;
  warningCount: number;
  conflictCount: number;
  quota: QuotaInfo | null;
  recentFiles: RecentFileItem[];
  providerCommands: QuickActionInfo[];
  diagnosticsAgeSecs: number | null;
}

export interface HudSnapshot {
  onedriveRunning: boolean;
  onedriveExePath: string | null;
  aggregateState: SyncState;
  aggregateLabel: string;
  aggregateSummary: string;
  aggregateProgressPercent: number | null;
  totalSpeedBps: number;
  uploadSpeedBps: number;
  downloadSpeedBps: number;
  totalFilesRemaining: number;
  totalBytesRemaining: number;
  totalEtaSeconds: number | null;
  speedHistoryBps: number[];
  systemTheme: string;
  isFullscreen: boolean;
  accounts: AccountTelemetry[];
  updatedAtMs: number;
}

export interface AppSettings {
  autoHideWhenIdle: boolean;
  idleHideDelaySecs: number;
  hideInFullscreen: boolean;
  locked: boolean;
  opacity: number;
  theme: "auto" | "light" | "dark";
  startWithWindows: boolean;
  positionX: number | null;
  positionY: number | null;
  hotkey: string;
}

export type SettingsPatch = Partial<
  Pick<
    AppSettings,
    | "autoHideWhenIdle"
    | "idleHideDelaySecs"
    | "hideInFullscreen"
    | "locked"
    | "opacity"
    | "theme"
    | "startWithWindows"
  >
>;
