pub mod accounts;
pub mod diagnostics;
pub mod file_watcher;
pub mod fullscreen;
pub mod status_ui;
pub mod throughput;
pub mod tray_tooltip;

use crate::models::{AccountTelemetry, FileTransferState, HudSnapshot, SyncState};
use accounts::{detect_onedrive_runtime, detect_windows_theme, discover_accounts, DiscoveredAccount};
use diagnostics::read_diagnostics_file;
use file_watcher::FolderFileTracker;
use fullscreen::is_fullscreen_app_active;
use status_ui::StatusUiClient;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use throughput::ThroughputTracker;
use tray_tooltip::{TrayTooltipEntry, TrayTooltipReader};

/// Max age in seconds for SyncDiagnostics.log numeric counters to be trusted as active
/// when live StorageProviderStatusUI / Tray Tooltip does not indicate active syncing.
const FRESH_DIAGNOSTICS_MAX_AGE_SECS: u64 = 180;

pub struct TelemetryEngine {
    accounts: Vec<DiscoveredAccount>,
    last_account_scan: Instant,
    status_client: StatusUiClient,
    tooltip_reader: TrayTooltipReader,
    throughput: ThroughputTracker,
    file_tracker: FolderFileTracker,
}

impl TelemetryEngine {
    pub fn new() -> Self {
        let accounts = discover_accounts();
        let pairs: Vec<(String, String)> = accounts
            .iter()
            .map(|a| (a.id.clone(), a.folder_path.clone()))
            .collect();
        let file_tracker = FolderFileTracker::new(&pairs);

        Self {
            accounts,
            last_account_scan: Instant::now(),
            status_client: StatusUiClient::new(),
            tooltip_reader: TrayTooltipReader::new(),
            throughput: ThroughputTracker::new(),
            file_tracker,
        }
    }

    pub fn accounts(&self) -> &[DiscoveredAccount] {
        &self.accounts
    }

    pub fn refresh_accounts_if_needed(&mut self) {
        if self.last_account_scan.elapsed().as_secs() < 30 {
            return;
        }
        self.last_account_scan = Instant::now();
        let discovered = discover_accounts();
        let old_keys: Vec<&str> = self.accounts.iter().map(|a| a.folder_path.as_str()).collect();
        let new_keys: Vec<&str> = discovered.iter().map(|a| a.folder_path.as_str()).collect();
        if old_keys != new_keys {
            let pairs: Vec<(String, String)> = discovered
                .iter()
                .map(|a| (a.id.clone(), a.folder_path.clone()))
                .collect();
            self.file_tracker = FolderFileTracker::new(&pairs);
            self.status_client.clear();
            self.accounts = discovered;
        }
    }

    pub fn poll(&mut self) -> HudSnapshot {
        self.refresh_accounts_if_needed();

        let runtime = detect_onedrive_runtime();
        if !runtime.running {
            self.status_client.clear();
        }
        let tooltip_entries = if runtime.running {
            self.tooltip_reader.read_entries()
        } else {
            Vec::new()
        };
        let system_theme = detect_windows_theme();
        let is_fullscreen = is_fullscreen_app_active();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut account_snapshots = Vec::with_capacity(self.accounts.len());
        let mut any_syncing = false;
        let mut fresh_diag_up_sum = 0u64;
        let mut fresh_diag_down_sum = 0u64;
        let mut has_fresh_diag_speed = false;

        for acc in &self.accounts {
            let recent_files = self.file_tracker.snapshot_and_refresh(&acc.id);
            let active_file_count = recent_files
                .iter()
                .filter(|f| {
                    matches!(
                        f.state,
                        FileTransferState::Syncing | FileTransferState::Pending
                    )
                })
                .count() as u64;

            let diag = acc
                .diagnostics_log_path
                .as_deref()
                .and_then(read_diagnostics_file);

            let live_ui = if runtime.running {
                acc.sync_root_id.as_deref().and_then(|srid| {
                    self.status_client
                        .query(srid, acc.status_ui_factory_clsid.as_deref())
                })
            } else {
                None
            };

            let matched_tooltip = match_tooltip_for_account(&tooltip_entries, acc, self.accounts.len());

            // Detect elevation mismatch: when HUD runs elevated (High IL) while OneDrive.exe
            // runs normally (Medium IL), FileCoAuth returns Offline / "Start OneDrive" even
            // though OneDrive.exe is alive and actively syncing.
            let com_elevation_mismatch = runtime.running
                && live_ui.as_ref().is_some_and(|u| {
                    u.state == SyncState::Offline
                        && u.state_label.to_ascii_lowercase().contains("start onedrive")
                });

            let (mut state, mut state_label, mut status_summary, mut status_detail, quota, commands) =
                if !runtime.running {
                    (
                        SyncState::NotRunning,
                        "OneDrive not running".to_string(),
                        "Launch OneDrive to resume syncing".to_string(),
                        None,
                        None,
                        Vec::new(),
                    )
                } else if com_elevation_mismatch || live_ui.is_none() {
                    if let Some(tip) = matched_tooltip {
                        (
                            tip.state,
                            tip.state_label.clone(),
                            tip.status_text.clone(),
                            None,
                            None,
                            Vec::new(),
                        )
                    } else {
                        let diag_active = diag.as_ref().is_some_and(|d| {
                            d.is_fresh(FRESH_DIAGNOSTICS_MAX_AGE_SECS) && d.has_pending_work()
                        });
                        if diag_active || active_file_count > 0 {
                            (
                                SyncState::Syncing,
                                "Syncing".to_string(),
                                "Syncing changes".to_string(),
                                None,
                                None,
                                Vec::new(),
                            )
                        } else {
                            (
                                SyncState::InSync,
                                "Up to date".to_string(),
                                "All files synced".to_string(),
                                None,
                                None,
                                Vec::new(),
                            )
                        }
                    }
                } else if let Some(ref ui) = live_ui {
                    let mut st = ui.state;
                    let mut lbl = ui.state_label.clone();
                    let mut summary = ui
                        .sync_command_label
                        .clone()
                        .unwrap_or_else(|| ui.state_label.clone());
                    // Enrich with live tray tooltip if it reports a more specific active/error state
                    if let Some(tip) = matched_tooltip {
                        if tip.state.severity_rank() > st.severity_rank() {
                            st = tip.state;
                            lbl = tip.state_label.clone();
                            summary = tip.status_text.clone();
                        } else if st == SyncState::InSync && summary.eq_ignore_ascii_case(&lbl) {
                            summary = tip.status_text.clone();
                        }
                    }
                    (
                        st,
                        lbl,
                        summary,
                        ui.sync_command_description.clone(),
                        ui.quota.clone(),
                        ui.commands.clone(),
                    )
                } else {
                    (
                        SyncState::InSync,
                        "Up to date".to_string(),
                        "All files synced".to_string(),
                        None,
                        None,
                        Vec::new(),
                    )
                };

            let diag_fresh = diag
                .as_ref()
                .map(|d| d.is_fresh(FRESH_DIAGNOSTICS_MAX_AGE_SECS))
                .unwrap_or(false);

            let parsed = if !com_elevation_mismatch {
                live_ui
                    .as_ref()
                    .map(|u| &u.parsed_text)
                    .or_else(|| matched_tooltip.map(|t| &t.parsed))
            } else {
                matched_tooltip.map(|t| &t.parsed)
            };

            let mut files_to_upload = parsed.and_then(|p| p.files_uploading).unwrap_or(0);
            let mut files_to_download = parsed.and_then(|p| p.files_downloading).unwrap_or(0);
            let mut changes_to_process = parsed.and_then(|p| p.changes_processing).unwrap_or(0);
            let mut bytes_uploaded = 0u64;
            let mut bytes_to_upload = 0u64;
            let mut bytes_downloaded = 0u64;
            let mut bytes_to_download = 0u64;
            let mut diag_up_speed = 0u64;
            let mut diag_down_speed = 0u64;
            let mut eta_seconds: Option<u64> = None;
            let mut error_count = 0u64;
            let mut warning_count = 0u64;
            let mut conflict_count = 0u64;

            if let Some(ref d) = diag {
                if diag_fresh || state == SyncState::Syncing {
                    files_to_upload = files_to_upload.max(d.files_to_upload);
                    files_to_download = files_to_download.max(d.files_to_download);
                    changes_to_process =
                        changes_to_process.max(d.changes_to_process + d.changes_to_send);
                    bytes_uploaded = d.bytes_uploaded;
                    bytes_to_upload = d.bytes_to_upload;
                    bytes_downloaded = d.bytes_downloaded;
                    bytes_to_download = d.bytes_to_download;
                    diag_up_speed = d.upload_speed_bps;
                    diag_down_speed = d.download_speed_bps;
                    if d.est_time_remaining_secs > 0 {
                        eta_seconds = Some(d.est_time_remaining_secs);
                    }
                    if diag_fresh && (diag_up_speed > 0 || diag_down_speed > 0) {
                        fresh_diag_up_sum = fresh_diag_up_sum.saturating_add(diag_up_speed);
                        fresh_diag_down_sum = fresh_diag_down_sum.saturating_add(diag_down_speed);
                        has_fresh_diag_speed = true;
                    }
                    if diag_fresh && d.sync_stall_detected {
                        warning_count = warning_count.saturating_add(1);
                        if status_detail.is_none() {
                            status_detail = Some("OneDrive sync stall detected".to_string());
                        }
                    }
                    if diag_fresh && d.sync_progress_state == Some(0) && d.has_pending_work() {
                        state = SyncState::Syncing;
                    }
                }
                if diag_fresh || matches!(state, SyncState::Error | SyncState::Warning) {
                    error_count = d.total_errors();
                    warning_count = warning_count.max(d.files_in_warning);
                    conflict_count = d.conflicts_failed;
                }
            }

            if let Some(p) = parsed {
                if let (Some(done), Some(total)) = (p.bytes_done, p.bytes_total) {
                    if bytes_to_upload == 0 && bytes_to_download == 0 {
                        if files_to_download > files_to_upload {
                            bytes_downloaded = done;
                            bytes_to_download = total;
                        } else {
                            bytes_uploaded = done;
                            bytes_to_upload = total;
                        }
                    }
                }
            }

            if runtime.running && state == SyncState::InSync && active_file_count > 0 {
                state = SyncState::Syncing;
                state_label = "Syncing".to_string();
                status_summary = format!(
                    "Syncing {} {}",
                    active_file_count,
                    if active_file_count == 1 { "file" } else { "files" }
                );
                if files_to_upload == 0 && files_to_download == 0 {
                    files_to_upload = active_file_count;
                }
            }

            if state == SyncState::Syncing {
                any_syncing = true;
            }

            let total_bytes = bytes_to_upload.saturating_add(bytes_to_download);
            let done_bytes = bytes_uploaded.saturating_add(bytes_downloaded);

            let progress_percent = if state == SyncState::InSync {
                Some(100.0)
            } else if let Some(pct) = parsed.and_then(|p| p.progress_percent) {
                Some(pct)
            } else if total_bytes > 0 {
                Some(((done_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0))
            } else {
                None
            };

            account_snapshots.push(AccountTelemetry {
                id: acc.id.clone(),
                sync_root_id: acc.sync_root_id.clone(),
                display_name: acc.display_name.clone(),
                is_business: acc.is_business,
                folder_path: acc.folder_path.clone(),
                state,
                state_label,
                status_summary,
                status_detail,
                progress_percent,
                files_to_upload,
                files_to_download,
                bytes_uploaded,
                bytes_to_upload,
                bytes_downloaded,
                bytes_to_download,
                changes_to_process,
                upload_speed_bps: diag_up_speed,
                download_speed_bps: diag_down_speed,
                eta_seconds,
                error_count,
                warning_count,
                conflict_count,
                quota,
                recent_files,
                provider_commands: commands,
                diagnostics_age_secs: diag.and_then(|d| d.age_secs),
            });
        }

        let sample = self.throughput.update(
            &runtime.pids,
            any_syncing,
            if has_fresh_diag_speed {
                Some(fresh_diag_up_sum)
            } else {
                None
            },
            if has_fresh_diag_speed {
                Some(fresh_diag_down_sum)
            } else {
                None
            },
        );

        if sample.total_bps > 0 {
            let active_indices: Vec<usize> = account_snapshots
                .iter()
                .enumerate()
                .filter(|(_, a)| a.state == SyncState::Syncing)
                .map(|(i, _)| i)
                .collect();
            if active_indices.len() == 1 {
                let idx = active_indices[0];
                if account_snapshots[idx].upload_speed_bps == 0
                    && account_snapshots[idx].download_speed_bps == 0
                {
                    account_snapshots[idx].upload_speed_bps = sample.upload_bps;
                    account_snapshots[idx].download_speed_bps = sample.download_bps;
                }
            }
        }

        build_aggregate_snapshot(
            runtime.running,
            runtime.exe_path,
            account_snapshots,
            sample.upload_bps,
            sample.download_bps,
            self.throughput.history(),
            system_theme,
            is_fullscreen,
            now_ms,
        )
    }
}

fn match_tooltip_for_account<'a>(
    entries: &'a [TrayTooltipEntry],
    acc: &DiscoveredAccount,
    total_accounts: usize,
) -> Option<&'a TrayTooltipEntry> {
    if entries.is_empty() {
        return None;
    }
    let disp_lower = acc.display_name.to_ascii_lowercase();
    if let Some(exact) = entries
        .iter()
        .find(|e| e.account_title.to_ascii_lowercase() == disp_lower)
    {
        return Some(exact);
    }
    if !acc.is_business {
        if let Some(personal) = entries
            .iter()
            .find(|e| e.account_title.to_ascii_lowercase().contains("personal"))
        {
            return Some(personal);
        }
    }
    if total_accounts == 1 && entries.len() == 1 {
        return Some(&entries[0]);
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn build_aggregate_snapshot(
    onedrive_running: bool,
    onedrive_exe_path: Option<String>,
    accounts: Vec<AccountTelemetry>,
    upload_speed_bps: u64,
    download_speed_bps: u64,
    speed_history_bps: Vec<u64>,
    system_theme: String,
    is_fullscreen: bool,
    updated_at_ms: u64,
) -> HudSnapshot {
    let total_speed_bps = upload_speed_bps.saturating_add(download_speed_bps);

    if !onedrive_running {
        return HudSnapshot {
            onedrive_running: false,
            onedrive_exe_path,
            aggregate_state: SyncState::NotRunning,
            aggregate_label: "OneDrive not running".to_string(),
            aggregate_summary: "Click to launch OneDrive".to_string(),
            aggregate_progress_percent: None,
            total_speed_bps: 0,
            upload_speed_bps: 0,
            download_speed_bps: 0,
            total_files_remaining: 0,
            total_bytes_remaining: 0,
            total_eta_seconds: None,
            speed_history_bps,
            system_theme,
            is_fullscreen,
            accounts,
            updated_at_ms,
        };
    }

    if accounts.is_empty() {
        return HudSnapshot {
            onedrive_running: true,
            onedrive_exe_path,
            aggregate_state: SyncState::Warning,
            aggregate_label: "No OneDrive account".to_string(),
            aggregate_summary: "Sign in to OneDrive to monitor sync".to_string(),
            aggregate_progress_percent: None,
            total_speed_bps: 0,
            upload_speed_bps: 0,
            download_speed_bps: 0,
            total_files_remaining: 0,
            total_bytes_remaining: 0,
            total_eta_seconds: None,
            speed_history_bps,
            system_theme,
            is_fullscreen,
            accounts,
            updated_at_ms,
        };
    }

    let primary_acc = accounts
        .iter()
        .max_by_key(|a| a.state.severity_rank())
        .expect("accounts verified non-empty");

    let aggregate_state = primary_acc.state;
    let aggregate_label = primary_acc.state_label.clone();

    let total_files_remaining: u64 = accounts
        .iter()
        .map(|a| a.files_to_upload.saturating_add(a.files_to_download))
        .sum();

    let total_bytes_all: u64 = accounts
        .iter()
        .map(|a| a.bytes_to_upload.saturating_add(a.bytes_to_download))
        .sum();
    let total_bytes_done: u64 = accounts
        .iter()
        .map(|a| a.bytes_uploaded.saturating_add(a.bytes_downloaded))
        .sum();
    let total_bytes_remaining = total_bytes_all.saturating_sub(total_bytes_done);

    let aggregate_progress_percent = if aggregate_state == SyncState::InSync {
        Some(100.0)
    } else if total_bytes_all > 0 {
        Some(((total_bytes_done as f64 / total_bytes_all as f64) * 100.0).clamp(0.0, 100.0))
    } else if accounts.len() == 1 {
        primary_acc.progress_percent
    } else {
        let determinate: Vec<f64> = accounts
            .iter()
            .filter(|a| a.state == SyncState::Syncing)
            .filter_map(|a| a.progress_percent)
            .collect();
        if determinate.is_empty() {
            None
        } else {
            Some(determinate.iter().sum::<f64>() / determinate.len() as f64)
        }
    };

    let total_eta_seconds = accounts
        .iter()
        .filter_map(|a| a.eta_seconds)
        .max()
        .or_else(|| {
            if total_bytes_remaining > 0 && total_speed_bps > 1_024 {
                Some(total_bytes_remaining / total_speed_bps)
            } else {
                None
            }
        });

    let aggregate_summary = if accounts.len() == 1 {
        primary_acc.status_summary.clone()
    } else if aggregate_state == SyncState::Syncing {
        if total_files_remaining > 0 {
            format!(
                "Syncing {} {}",
                total_files_remaining,
                if total_files_remaining == 1 {
                    "file"
                } else {
                    "files"
                }
            )
        } else {
            primary_acc.status_summary.clone()
        }
    } else {
        primary_acc.status_summary.clone()
    };

    HudSnapshot {
        onedrive_running,
        onedrive_exe_path,
        aggregate_state,
        aggregate_label,
        aggregate_summary,
        aggregate_progress_percent,
        total_speed_bps,
        upload_speed_bps,
        download_speed_bps,
        total_files_remaining,
        total_bytes_remaining,
        total_eta_seconds,
        speed_history_bps,
        system_theme,
        is_fullscreen,
        accounts,
        updated_at_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_windows_onedrive_telemetry_smoke_test() {
        let mut engine = TelemetryEngine::new();
        let snap = engine.poll();
        println!(
            "LIVE_TELEMETRY: running={} state={:?} label='{}' summary='{}' accounts={} theme={} fullscreen={}",
            snap.onedrive_running,
            snap.aggregate_state,
            snap.aggregate_label,
            snap.aggregate_summary,
            snap.accounts.len(),
            snap.system_theme,
            snap.is_fullscreen
        );
        assert!(!snap.system_theme.is_empty());
        assert_eq!(snap.speed_history_bps.len(), 30);
        assert!(!snap.is_fullscreen);
    }
}
