use crate::models::{QuickActionInfo, QuotaInfo, SyncState};

#[derive(Debug, Clone, Default)]
pub struct ParsedStatusText {
    pub progress_percent: Option<f64>,
    pub bytes_done: Option<u64>,
    pub bytes_total: Option<u64>,
    pub files_uploading: Option<u64>,
    pub files_downloading: Option<u64>,
    pub changes_processing: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct StatusUiSnapshot {
    pub state: SyncState,
    pub state_label: String,
    pub sync_command_label: Option<String>,
    pub sync_command_description: Option<String>,
    pub quota: Option<QuotaInfo>,
    pub commands: Vec<QuickActionInfo>,
    pub parsed_text: ParsedStatusText,
}

/// Parses human-readable OneDrive status strings such as:
/// - "Uploading 5 files (12.4 MB of 40.0 MB)"
/// - "Downloading 2 files (1.2 GB of 2.5 GB)"
/// - "Processing 1,245 changes"
/// - "Syncing 45%"
pub fn parse_status_text(label: &str, description: Option<&str>) -> ParsedStatusText {
    let mut result = ParsedStatusText::default();
    let combined = match description {
        Some(desc) if !desc.trim().is_empty() => format!("{label} {desc}"),
        _ => label.to_string(),
    };
    let lower = combined.to_ascii_lowercase();

    if let Some(pct) = extract_percentage(&lower) {
        result.progress_percent = Some(pct.clamp(0.0, 100.0));
    }

    if let Some((done, total)) = extract_byte_pair(&lower) {
        result.bytes_done = Some(done);
        result.bytes_total = Some(total);
        if total > 0 && result.progress_percent.is_none() {
            result.progress_percent = Some(((done as f64 / total as f64) * 100.0).clamp(0.0, 100.0));
        }
    }

    if let Some(count) = extract_count_after_keyword(&lower, &["uploading", "mengunggah"]) {
        result.files_uploading = Some(count);
    }
    if let Some(count) = extract_count_after_keyword(&lower, &["downloading", "mengunduh"]) {
        result.files_downloading = Some(count);
    }
    if let Some(count) =
        extract_count_after_keyword(&lower, &["processing", "memproses", "looking for"])
    {
        result.changes_processing = Some(count);
    }

    result
}

fn extract_percentage(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b == b'%' && idx > 0 {
            let mut start = idx;
            while start > 0 {
                let c = bytes[start - 1];
                if c.is_ascii_digit() || c == b'.' || c == b',' {
                    start -= 1;
                } else {
                    break;
                }
            }
            if start < idx {
                let num_str = text[start..idx].replace(',', ".");
                if let Ok(val) = num_str.parse::<f64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

fn parse_size_unit(unit: &str) -> Option<f64> {
    match unit.trim_end_matches(|c: char| !c.is_ascii_alphabetic()) {
        "b" | "bytes" | "byte" => Some(1.0),
        "kb" | "kib" => Some(1024.0),
        "mb" | "mib" => Some(1024.0 * 1024.0),
        "gb" | "gib" => Some(1024.0 * 1024.0 * 1024.0),
        "tb" | "tib" => Some(1024.0 * 1024.0 * 1024.0 * 1024.0),
        _ => None,
    }
}

fn parse_number_token(tok: &str) -> Option<f64> {
    let cleaned = tok.trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != ',');
    if cleaned.is_empty() {
        return None;
    }
    let normalized = if cleaned.contains('.') && cleaned.contains(',') {
        cleaned.replace(',', "")
    } else if cleaned.contains(',') {
        let parts: Vec<&str> = cleaned.split(',').collect();
        if parts.len() == 2 && parts[1].len() <= 2 {
            cleaned.replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else {
        cleaned.to_string()
    };
    normalized.parse::<f64>().ok()
}

fn extract_byte_pair(text: &str) -> Option<(u64, u64)> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for i in 0..tokens.len() {
        let connector = tokens[i].trim_matches(|c: char| !c.is_ascii_alphabetic());
        if connector != "of" && connector != "dari" && connector != "/" {
            continue;
        }
        if i >= 2 && i + 2 < tokens.len() {
            if let (Some(n1), Some(u1), Some(n2), Some(u2)) = (
                parse_number_token(tokens[i - 2]),
                parse_size_unit(tokens[i - 1]),
                parse_number_token(tokens[i + 1]),
                parse_size_unit(tokens[i + 2]),
            ) {
                let b1 = (n1 * u1).round().max(0.0) as u64;
                let b2 = (n2 * u2).round().max(0.0) as u64;
                if b2 > 0 {
                    return Some((b1.min(b2), b2));
                }
            }
        }
    }
    None
}

fn extract_count_after_keyword(text: &str, keywords: &[&str]) -> Option<u64> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        if keywords.iter().any(|kw| tok.contains(kw)) {
            for next in tokens.iter().skip(i + 1).take(2) {
                let cleaned = next
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != ',' && c != '.')
                    .replace([',', '.'], "");
                if let Ok(n) = cleaned.parse::<u64>() {
                    let is_unit = tokens
                        .get(i + 2)
                        .and_then(|u| parse_size_unit(u))
                        .is_some();
                    if !is_unit && n > 0 {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

#[cfg(windows)]
mod win_impl {
    use super::{parse_status_text, StatusUiSnapshot};
    use crate::models::{QuickActionInfo, QuotaInfo, SyncState};
    use std::collections::HashMap;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Storage::Provider::{
        IStorageProviderStatusUISource, IStorageProviderStatusUISourceFactory,
        StorageProviderState, StorageProviderStatusUI, StorageProviderUICommandState,
    };
    use windows::Win32::System::Com::{
        CLSIDFromString, CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER,
        CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED,
    };

    const DEFAULT_ONEDRIVE_FACTORY_CLSID: &str = "{0827D883-485C-4D62-BA2C-A332DBF3D4B0}";

    pub fn ensure_com_initialized() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    fn map_provider_state(state: StorageProviderState) -> SyncState {
        match state {
            StorageProviderState::InSync => SyncState::InSync,
            StorageProviderState::Syncing => SyncState::Syncing,
            StorageProviderState::Paused => SyncState::Paused,
            StorageProviderState::Error => SyncState::Error,
            StorageProviderState::Warning => SyncState::Warning,
            StorageProviderState::Offline => SyncState::Offline,
            _ => SyncState::Unknown,
        }
    }

    fn create_factory(clsid_str: Option<&str>) -> Option<IStorageProviderStatusUISourceFactory> {
        ensure_com_initialized();
        let raw = clsid_str.unwrap_or(DEFAULT_ONEDRIVE_FACTORY_CLSID);
        let wide: Vec<u16> = raw.encode_utf16().chain(std::iter::once(0)).collect();
        let clsid = unsafe { CLSIDFromString(PCWSTR(wide.as_ptr())).ok()? };
        unsafe {
            CoCreateInstance::<_, IStorageProviderStatusUISourceFactory>(
                &clsid,
                None,
                CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER,
            )
            .ok()
        }
    }

    fn extract_snapshot(ui: &StorageProviderStatusUI) -> StatusUiSnapshot {
        let state = ui
            .ProviderState()
            .map(map_provider_state)
            .unwrap_or(SyncState::Unknown);

        let state_label = ui
            .ProviderStateLabel()
            .ok()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| state.default_label().to_string());

        let (sync_command_label, sync_command_description) = match ui.SyncStatusCommand() {
            Ok(cmd) => {
                let lbl = cmd
                    .Label()
                    .ok()
                    .map(|s| s.to_string())
                    .filter(|s| !s.trim().is_empty());
                let desc = cmd
                    .Description()
                    .ok()
                    .map(|s| s.to_string())
                    .filter(|s| !s.trim().is_empty());
                (lbl, desc)
            }
            Err(_) => (None, None),
        };

        let quota = ui.QuotaUI().ok().and_then(|q| {
            let total = q.QuotaTotalInBytes().ok().unwrap_or(0);
            let used = q.QuotaUsedInBytes().ok().unwrap_or(0);
            let label = q
                .QuotaUsedLabel()
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_default();
            if total == 0 && label.trim().is_empty() {
                None
            } else {
                let pct = if total > 0 {
                    ((used as f64 / total as f64) * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };
                Some(QuotaInfo {
                    used_bytes: used,
                    total_bytes: total,
                    used_percent: pct,
                    label,
                })
            }
        });

        let mut commands = Vec::new();
        if let Ok(primary) = ui.ProviderPrimaryCommand() {
            let cmd_state = primary
                .State()
                .unwrap_or(StorageProviderUICommandState::Disabled);
            if cmd_state != StorageProviderUICommandState::Hidden {
                if let Ok(lbl) = primary.Label() {
                    let lbl_str = lbl.to_string();
                    if !lbl_str.trim().is_empty() {
                        commands.push(QuickActionInfo {
                            id: "primary".to_string(),
                            label: lbl_str,
                            description: primary
                                .Description()
                                .ok()
                                .map(|s| s.to_string())
                                .unwrap_or_default(),
                            enabled: cmd_state == StorageProviderUICommandState::Enabled,
                            kind: "primary".to_string(),
                        });
                    }
                }
            }
        }

        if let Ok(secondary_list) = ui.ProviderSecondaryCommands() {
            if let Ok(count) = secondary_list.Size() {
                for idx in 0..count {
                    if let Ok(cmd) = secondary_list.GetAt(idx) {
                        let cmd_state = cmd
                            .State()
                            .unwrap_or(StorageProviderUICommandState::Disabled);
                        if cmd_state == StorageProviderUICommandState::Hidden {
                            continue;
                        }
                        if let Ok(lbl) = cmd.Label() {
                            let lbl_str = lbl.to_string();
                            if !lbl_str.trim().is_empty() {
                                commands.push(QuickActionInfo {
                                    id: format!("secondary:{idx}"),
                                    label: lbl_str,
                                    description: cmd
                                        .Description()
                                        .ok()
                                        .map(|s| s.to_string())
                                        .unwrap_or_default(),
                                    enabled: cmd_state == StorageProviderUICommandState::Enabled,
                                    kind: "secondary".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let parsed_text = parse_status_text(
            sync_command_label.as_deref().unwrap_or(&state_label),
            sync_command_description.as_deref(),
        );

        StatusUiSnapshot {
            state,
            state_label,
            sync_command_label,
            sync_command_description,
            quota,
            commands,
            parsed_text,
        }
    }

    /// Persistent COM session caching factories and per-root StatusUISources to avoid
    /// repeatedly activating FileCoAuth.exe on every polling tick.
    #[derive(Default)]
    pub struct StatusUiClient {
        factories: HashMap<String, IStorageProviderStatusUISourceFactory>,
        sources: HashMap<String, IStorageProviderStatusUISource>,
    }

    impl StatusUiClient {
        pub fn new() -> Self {
            ensure_com_initialized();
            Self::default()
        }

        pub fn clear(&mut self) {
            self.sources.clear();
            self.factories.clear();
        }

        pub fn query(
            &mut self,
            sync_root_id: &str,
            clsid_str: Option<&str>,
        ) -> Option<StatusUiSnapshot> {
            // Try cached source first
            if let Some(source) = self.sources.get(sync_root_id) {
                if let Ok(ui) = source.GetStatusUI() {
                    return Some(extract_snapshot(&ui));
                }
                // Evict stale COM proxy if OneDrive/FileCoAuth restarted
                self.sources.remove(sync_root_id);
            }

            let clsid_key = clsid_str
                .unwrap_or(DEFAULT_ONEDRIVE_FACTORY_CLSID)
                .to_string();
            if !self.factories.contains_key(&clsid_key) {
                let factory = create_factory(Some(&clsid_key))?;
                self.factories.insert(clsid_key.clone(), factory);
            }

            let source = match self
                .factories
                .get(&clsid_key)
                .and_then(|f| f.GetStatusUISource(&HSTRING::from(sync_root_id)).ok())
            {
                Some(s) => s,
                None => {
                    // Retry once with a fresh factory
                    self.factories.remove(&clsid_key);
                    let factory = create_factory(Some(&clsid_key))?;
                    let s = factory
                        .GetStatusUISource(&HSTRING::from(sync_root_id))
                        .ok()?;
                    self.factories.insert(clsid_key, factory);
                    s
                }
            };

            let ui = source.GetStatusUI().ok()?;
            let snap = extract_snapshot(&ui);
            self.sources.insert(sync_root_id.to_string(), source);
            Some(snap)
        }
    }

    pub fn invoke_provider_command(
        sync_root_id: &str,
        clsid_str: Option<&str>,
        command_id: &str,
    ) -> Result<(), String> {
        let factory = create_factory(clsid_str)
            .ok_or_else(|| "Unable to connect to OneDrive Status UI provider".to_string())?;
        let source = factory
            .GetStatusUISource(&HSTRING::from(sync_root_id))
            .map_err(|e| format!("Failed to resolve sync root status source: {e}"))?;
        let ui = source
            .GetStatusUI()
            .map_err(|e| format!("Failed to read OneDrive status UI: {e}"))?;

        if command_id == "sync_status" {
            let cmd = ui
                .SyncStatusCommand()
                .map_err(|e| format!("Sync status command unavailable: {e}"))?;
            cmd.Invoke()
                .map_err(|e| format!("Failed to invoke sync status command: {e}"))?;
            return Ok(());
        }

        if command_id == "primary" {
            let cmd = ui
                .ProviderPrimaryCommand()
                .map_err(|e| format!("Primary command unavailable: {e}"))?;
            cmd.Invoke()
                .map_err(|e| format!("Failed to invoke primary command: {e}"))?;
            return Ok(());
        }

        if let Some(idx_str) = command_id.strip_prefix("secondary:") {
            let idx: u32 = idx_str
                .parse()
                .map_err(|_| format!("Invalid secondary command index: {idx_str}"))?;
            let list = ui
                .ProviderSecondaryCommands()
                .map_err(|e| format!("Secondary commands unavailable: {e}"))?;
            let cmd = list
                .GetAt(idx)
                .map_err(|e| format!("Secondary command #{idx} not found: {e}"))?;
            cmd.Invoke()
                .map_err(|e| format!("Failed to invoke secondary command #{idx}: {e}"))?;
            return Ok(());
        }

        Err(format!("Unknown provider command id: {command_id}"))
    }
}

#[cfg(windows)]
pub use win_impl::{invoke_provider_command, StatusUiClient};

#[cfg(not(windows))]
#[derive(Default)]
pub struct StatusUiClient;

#[cfg(not(windows))]
impl StatusUiClient {
    pub fn new() -> Self {
        Self
    }

    pub fn clear(&mut self) {}

    pub fn query(
        &mut self,
        _sync_root_id: &str,
        _clsid_str: Option<&str>,
    ) -> Option<StatusUiSnapshot> {
        None
    }
}

#[cfg(not(windows))]
pub fn invoke_provider_command(
    _sync_root_id: &str,
    _clsid_str: Option<&str>,
    _command_id: &str,
) -> Result<(), String> {
    Err("Windows StorageProvider API is only available on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_upload_progress_bytes_and_files() {
        let p = parse_status_text("Uploading 5 files (12.5 MB of 50.0 MB)", None);
        assert_eq!(p.files_uploading, Some(5));
        assert!(p.bytes_done.unwrap() > 13_000_000);
        assert!(p.bytes_total.unwrap() > 52_000_000);
        let pct = p.progress_percent.unwrap();
        assert!((pct - 25.0).abs() < 0.2);
    }

    #[test]
    fn parses_processing_changes_and_percentage() {
        let p = parse_status_text("Processing 1,245 changes", Some("62% complete"));
        assert_eq!(p.changes_processing, Some(1245));
        assert_eq!(p.progress_percent, Some(62.0));
    }

    #[test]
    fn parses_download_gb_pair() {
        let p = parse_status_text("Downloading 2 files", Some("1.5 GB of 3.0 GB"));
        assert_eq!(p.files_downloading, Some(2));
        let pct = p.progress_percent.unwrap();
        assert!((pct - 50.0).abs() < 0.2);
    }
}
