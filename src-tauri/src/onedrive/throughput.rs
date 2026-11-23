use std::collections::VecDeque;
use std::time::Instant;

const HISTORY_LEN: usize = 30;

#[derive(Debug, Default)]
pub struct ThroughputTracker {
    prev_read_bytes: u64,
    prev_write_bytes: u64,
    prev_sample_at: Option<Instant>,
    ema_upload_bps: f64,
    ema_download_bps: f64,
    history_bps: VecDeque<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ThroughputSample {
    pub upload_bps: u64,
    pub download_bps: u64,
    pub total_bps: u64,
}

impl ThroughputTracker {
    pub fn new() -> Self {
        let mut history_bps = VecDeque::with_capacity(HISTORY_LEN);
        for _ in 0..HISTORY_LEN {
            history_bps.push_back(0);
        }
        Self {
            history_bps,
            ..Default::default()
        }
    }

    /// Samples cumulative process I/O bytes across all running OneDrive.exe PIDs
    /// and merges with explicit diagnostics speeds when fresh.
    pub fn update(
        &mut self,
        pids: &[u32],
        is_syncing: bool,
        diag_upload_bps: Option<u64>,
        diag_download_bps: Option<u64>,
    ) -> ThroughputSample {
        let now = Instant::now();
        let (read_total, write_total) = sum_process_io(pids);

        let mut io_read_rate = 0.0;
        let mut io_write_rate = 0.0;

        if let Some(prev_at) = self.prev_sample_at {
            let dt = now.duration_since(prev_at).as_secs_f64();
            if dt >= 0.25 {
                let d_read = read_total.saturating_sub(self.prev_read_bytes);
                let d_write = write_total.saturating_sub(self.prev_write_bytes);
                io_read_rate = d_read as f64 / dt;
                io_write_rate = d_write as f64 / dt;
            }
        }

        self.prev_read_bytes = read_total;
        self.prev_write_bytes = write_total;
        self.prev_sample_at = Some(now);

        // When OneDrive reads local files to upload them, ReadTransferCount rises;
        // when it downloads cloud files to disk, WriteTransferCount rises.
        // Filter out tiny background DB/log churn (< 8 KB/s) when idle.
        let raw_up = if let Some(du) = diag_upload_bps {
            du as f64
        } else if is_syncing && io_read_rate >= 4_096.0 {
            io_read_rate
        } else {
            0.0
        };

        let raw_down = if let Some(dd) = diag_download_bps {
            dd as f64
        } else if is_syncing && io_write_rate >= 4_096.0 {
            io_write_rate
        } else {
            0.0
        };

        // Smooth with exponential moving average (alpha = 0.45)
        let alpha = 0.45;
        self.ema_upload_bps = if raw_up == 0.0 && !is_syncing {
            0.0
        } else {
            alpha * raw_up + (1.0 - alpha) * self.ema_upload_bps
        };
        self.ema_download_bps = if raw_down == 0.0 && !is_syncing {
            0.0
        } else {
            alpha * raw_down + (1.0 - alpha) * self.ema_download_bps
        };

        let upload_bps = self.ema_upload_bps.round().max(0.0) as u64;
        let download_bps = self.ema_download_bps.round().max(0.0) as u64;
        let total_bps = upload_bps.saturating_add(download_bps);

        if self.history_bps.len() >= HISTORY_LEN {
            self.history_bps.pop_front();
        }
        self.history_bps.push_back(total_bps);

        ThroughputSample {
            upload_bps,
            download_bps,
            total_bps,
        }
    }

    pub fn history(&self) -> Vec<u64> {
        self.history_bps.iter().copied().collect()
    }
}

#[cfg(windows)]
fn sum_process_io(pids: &[u32]) -> (u64, u64) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        GetProcessIoCounters, OpenProcess, IO_COUNTERS, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let mut total_read = 0u64;
    let mut total_write = 0u64;

    for &pid in pids {
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                let mut counters = IO_COUNTERS::default();
                if GetProcessIoCounters(handle, &mut counters).is_ok() {
                    total_read = total_read.saturating_add(counters.ReadTransferCount);
                    total_write = total_write.saturating_add(counters.WriteTransferCount);
                }
                let _ = CloseHandle(handle);
            }
        }
    }

    (total_read, total_write)
}

#[cfg(not(windows))]
fn sum_process_io(_pids: &[u32]) -> (u64, u64) {
    (0, 0)
}
