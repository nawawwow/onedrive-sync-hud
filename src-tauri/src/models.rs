use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    InSync,
    Syncing,
    Paused,
    Warning,
    Error,
    Offline,
    NotRunning,
    Unknown,
}

impl SyncState {
    pub fn severity_rank(self) -> u8 {
        match self {
            SyncState::Error => 7,
            SyncState::Warning => 6,
            SyncState::Syncing => 5,
            SyncState::Paused => 4,
            SyncState::Offline => 3,
            SyncState::NotRunning => 2,
            SyncState::InSync => 1,
            SyncState::Unknown => 0,
        }
    }

    pub fn is_attention_required(self) -> bool {
        matches!(
            self,
            SyncState::Error
                | SyncState::Warning
                | SyncState::Paused
                | SyncState::Offline
                | SyncState::NotRunning
        )
    }

    pub fn is_active(self) -> bool {
        matches!(self, SyncState::Syncing)
    }

    pub fn default_label(self) -> &'static str {
        match self {
            SyncState::InSync => "Up to date",
            SyncState::Syncing => "Syncing",
            SyncState::Paused => "Sync paused",
            SyncState::Warning => "Sync attention needed",
            SyncState::Error => "Sync error",
            SyncState::Offline => "Offline",
            SyncState::NotRunning => "OneDrive not running",
            SyncState::Unknown => "Checking OneDrive...",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileTransferState {
    Syncing,
    Synced,
    Pending,
    Error,
    CloudOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFileItem {
    pub path: String,
    pub file_name: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub state: FileTransferState,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfo {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub used_percent: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickActionInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTelemetry {
    pub id: String,
    pub sync_root_id: Option<String>,
    pub display_name: String,
    pub is_business: bool,
    pub folder_path: String,
    pub state: SyncState,
    pub state_label: String,
    pub status_summary: String,
    pub status_detail: Option<String>,
    pub progress_percent: Option<f64>,
    pub files_to_upload: u64,
    pub files_to_download: u64,
    pub bytes_uploaded: u64,
    pub bytes_to_upload: u64,
    pub bytes_downloaded: u64,
    pub bytes_to_download: u64,
    pub changes_to_process: u64,
    pub upload_speed_bps: u64,
    pub download_speed_bps: u64,
    pub eta_seconds: Option<u64>,
    pub error_count: u64,
    pub warning_count: u64,
    pub conflict_count: u64,
    pub quota: Option<QuotaInfo>,
    pub recent_files: Vec<RecentFileItem>,
    pub provider_commands: Vec<QuickActionInfo>,
    pub diagnostics_age_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudSnapshot {
    pub onedrive_running: bool,
    pub onedrive_exe_path: Option<String>,
    pub aggregate_state: SyncState,
    pub aggregate_label: String,
    pub aggregate_summary: String,
    pub aggregate_progress_percent: Option<f64>,
    pub total_speed_bps: u64,
    pub upload_speed_bps: u64,
    pub download_speed_bps: u64,
    pub total_files_remaining: u64,
    pub total_bytes_remaining: u64,
    pub total_eta_seconds: Option<u64>,
    pub speed_history_bps: Vec<u64>,
    pub system_theme: String,
    pub is_fullscreen: bool,
    pub accounts: Vec<AccountTelemetry>,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_hide_when_idle: bool,
    pub idle_hide_delay_secs: u64,
    pub hide_in_fullscreen: bool,
    pub locked: bool,
    pub opacity: f64,
    pub theme: String,
    pub start_with_windows: bool,
    pub position_x: Option<i32>,
    pub position_y: Option<i32>,
    pub hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_hide_when_idle: true,
            idle_hide_delay_secs: 6,
            hide_in_fullscreen: true,
            locked: false,
            opacity: 1.0,
            theme: "auto".to_string(),
            start_with_windows: false,
            position_x: None,
            position_y: None,
            hotkey: "Ctrl+Alt+O".to_string(),
        }
    }
}
