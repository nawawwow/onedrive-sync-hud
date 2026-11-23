use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Default)]
pub struct DiagnosticsData {
    pub sync_progress_state: Option<i64>,
    pub age_secs: Option<u64>,
    pub bytes_downloaded: u64,
    pub bytes_to_download: u64,
    pub bytes_uploaded: u64,
    pub bytes_to_upload: u64,
    pub files_to_download: u64,
    pub files_to_upload: u64,
    pub changes_to_process: u64,
    pub changes_to_send: u64,
    pub download_speed_bps: u64,
    pub upload_speed_bps: u64,
    pub est_time_remaining_secs: u64,
    pub failed_uploads: u64,
    pub failed_downloads: u64,
    pub conflicts_failed: u64,
    pub files_in_warning: u64,
    pub sync_stall_detected: bool,
}

impl DiagnosticsData {
    pub fn is_fresh(&self, max_age_secs: u64) -> bool {
        self.age_secs.map(|age| age <= max_age_secs).unwrap_or(false)
    }

    pub fn total_errors(&self) -> u64 {
        self.failed_uploads
            .saturating_add(self.failed_downloads)
            .saturating_add(self.conflicts_failed)
    }

    pub fn has_pending_work(&self) -> bool {
        self.files_to_upload > 0
            || self.files_to_download > 0
            || self.changes_to_process > 0
            || self.changes_to_send > 0
            || self.bytes_to_upload > self.bytes_uploaded
            || self.bytes_to_download > self.bytes_downloaded
    }
}

pub fn read_diagnostics_file(path: &Path) -> Option<DiagnosticsData> {
    let content = fs::read_to_string(path).ok()?;
    let mtime = fs::metadata(path).ok().and_then(|m| m.modified().ok());
    Some(parse_diagnostics_content(&content, mtime, Utc::now()))
}

pub fn parse_diagnostics_content(
    content: &str,
    file_mtime: Option<SystemTime>,
    now: DateTime<Utc>,
) -> DiagnosticsData {
    let mut map: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let split_pos = trimmed.find('=').or_else(|| trimmed.find(':'));
        let Some(pos) = split_pos else {
            continue;
        };
        let key = trimmed[..pos].trim().to_ascii_lowercase();
        let val = trimmed[pos + 1..].trim().to_string();
        if !key.is_empty() {
            map.entry(key).or_insert(val);
        }
    }

    let get_u64 = |key: &str| -> u64 {
        map.get(key)
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|&v| v > 0)
            .map(|v| v as u64)
            .unwrap_or(0)
    };

    let get_i64 = |key: &str| -> Option<i64> { map.get(key).and_then(|v| v.parse::<i64>().ok()) };

    let utc_now = map
        .get("utcnow")
        .or_else(|| map.get("timeutc"))
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let age_secs = if let Some(ts) = utc_now {
        let diff = now.signed_duration_since(ts).num_seconds();
        Some(diff.max(0) as u64)
    } else if let Some(mt) = file_mtime {
        let dt: DateTime<Utc> = mt.into();
        let diff = now.signed_duration_since(dt).num_seconds();
        Some(diff.max(0) as u64)
    } else {
        None
    };

    DiagnosticsData {
        sync_progress_state: get_i64("syncprogressstate"),
        age_secs,
        bytes_downloaded: get_u64("bytesdownloaded"),
        bytes_to_download: get_u64("bytestodownload"),
        bytes_uploaded: get_u64("bytesuploaded"),
        bytes_to_upload: get_u64("bytestoupload"),
        files_to_download: get_u64("filestodownload"),
        files_to_upload: get_u64("filestoupload"),
        changes_to_process: get_u64("changestoprocess"),
        changes_to_send: get_u64("changestosend"),
        download_speed_bps: get_u64("downloadspeedbytespersec"),
        upload_speed_bps: get_u64("uploadspeedbytespersec"),
        est_time_remaining_secs: get_u64("esttimeremaininginsec"),
        failed_uploads: get_u64("numfilefaileduploads")
            .max(get_u64("numuploaderrorsreported")),
        failed_downloads: get_u64("numfilefaileddownloads")
            .max(get_u64("numdownloaderrorsreported")),
        conflicts_failed: get_u64("conflictsfailed"),
        files_in_warning: get_u64("numfileinwarning"),
        sync_stall_detected: get_u64("syncstalldetected") > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_sync_diagnostics_fields() {
        let sample = "\
SyncProgressState = 552
UtcNow = 2026-11-23T11:29:01.0000000Z
BytesDownloaded = 5242880
BytesToDownload = 10485760
BytesToUpload = 20971520
BytesUploaded = 10485760
ChangesToProcess = 42
ChangesToSend = 7
DownloadSpeedBytesPerSec = 1048576
UploadSpeedBytesPerSec = 524288
EstTimeRemainingInSec = 25
FilesToDownload = 3
FilesToUpload = 2
numFileFailedUploads = 1
conflictsFailed = 2
numFileInWarning = 1
syncStallDetected = 0
";
        let now = DateTime::parse_from_rfc3339("2026-11-23T11:29:31Z")
            .unwrap()
            .with_timezone(&Utc);
        let parsed = parse_diagnostics_content(sample, None, now);

        assert_eq!(parsed.sync_progress_state, Some(552));
        assert_eq!(parsed.age_secs, Some(30));
        assert!(parsed.is_fresh(60));
        assert!(!parsed.is_fresh(10));
        assert!(parsed.has_pending_work());
        assert_eq!(parsed.files_to_download, 3);
        assert_eq!(parsed.files_to_upload, 2);
        assert_eq!(parsed.bytes_uploaded, 10_485_760);
        assert_eq!(parsed.bytes_to_upload, 20_971_520);
        assert_eq!(parsed.upload_speed_bps, 524_288);
        assert_eq!(parsed.download_speed_bps, 1_048_576);
        assert_eq!(parsed.est_time_remaining_secs, 25);
        assert_eq!(parsed.total_errors(), 3);
        assert_eq!(parsed.files_in_warning, 1);
        assert!(!parsed.sync_stall_detected);
    }

    #[test]
    fn clamps_negative_numbers_to_zero() {
        let sample = "EstTimeRemainingInSec = -1\nUploadSpeedBytesPerSec = -5\n";
        let parsed = parse_diagnostics_content(sample, None, Utc::now());
        assert_eq!(parsed.est_time_remaining_secs, 0);
        assert_eq!(parsed.upload_speed_bps, 0);
        assert!(!parsed.has_pending_work());
    }
}
