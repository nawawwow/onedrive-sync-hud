use crate::models::{FileTransferState, RecentFileItem};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT_FILES: usize = 14;

pub struct FolderFileTracker {
    _watchers: Vec<RecommendedWatcher>,
    recent_by_account: Arc<Mutex<HashMap<String, VecDeque<RecentFileItem>>>>,
}

impl FolderFileTracker {
    pub fn new(account_folders: &[(String, String)]) -> Self {
        let recent_by_account: Arc<Mutex<HashMap<String, VecDeque<RecentFileItem>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let mut watchers = Vec::new();

        for (account_id, folder_str) in account_folders {
            let folder_path = PathBuf::from(folder_str);
            if !folder_path.exists() {
                continue;
            }

            let store = Arc::clone(&recent_by_account);
            let acc_id = account_id.clone();
            let root_clone = folder_path.clone();

            let watcher_res = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    let Ok(event) = res else { return };
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {}
                        _ => return,
                    }

                    for path in event.paths {
                        if should_ignore_path(&path) || !path.is_file() {
                            continue;
                        }
                        if let Some(item) = build_recent_item(&root_clone, &path) {
                            if let Ok(mut map) = store.lock() {
                                let list = map.entry(acc_id.clone()).or_default();
                                list.retain(|existing| {
                                    !existing.path.eq_ignore_ascii_case(&item.path)
                                });
                                list.push_front(item);
                                while list.len() > MAX_RECENT_FILES {
                                    list.pop_back();
                                }
                            }
                        }
                    }
                },
                Config::default(),
            );

            if let Ok(mut watcher) = watcher_res {
                if watcher
                    .watch(&folder_path, RecursiveMode::Recursive)
                    .is_ok()
                {
                    watchers.push(watcher);
                }
            }
        }

        Self {
            _watchers: watchers,
            recent_by_account,
        }
    }

    /// Re-evaluates Shell StorageProviderState on recently tracked files so syncing items
    /// transition cleanly to synced or error as OneDrive finishes processing them.
    pub fn snapshot_and_refresh(&self, account_id: &str) -> Vec<RecentFileItem> {
        let Ok(mut map) = self.recent_by_account.lock() else {
            return Vec::new();
        };
        let Some(list) = map.get_mut(account_id) else {
            return Vec::new();
        };

        for item in list.iter_mut() {
            let path = Path::new(&item.path);
            if !path.exists() {
                continue;
            }
            if let Ok(meta) = std::fs::metadata(path) {
                item.size_bytes = meta.len();
            }
            item.state = detect_file_sync_state(path);
        }

        list.iter().cloned().collect()
    }
}

fn should_ignore_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("~$")
        || lower.starts_with(".~")
        || lower.ends_with(".tmp")
        || lower.ends_with(".partial")
        || lower.ends_with(".crdownload")
        || lower == "desktop.ini"
        || lower == "thumbs.db"
        || lower == ".ds_store"
    {
        return true;
    }

    // Ignore internal .git object churn to keep recent activity meaningful
    for comp in path.components() {
        if let Some(s) = comp.as_os_str().to_str() {
            if s.eq_ignore_ascii_case(".git") || s.eq_ignore_ascii_case("node_modules") {
                return true;
            }
        }
    }
    false
}

fn build_recent_item(root: &Path, path: &Path) -> Option<RecentFileItem> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let relative_path = path
        .strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.clone());
    let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let updated_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let state = detect_file_sync_state(path);

    Some(RecentFileItem {
        path: path.to_string_lossy().to_string(),
        file_name,
        relative_path,
        size_bytes,
        state,
        updated_at_ms,
    })
}

#[cfg(windows)]
fn detect_file_sync_state(path: &Path) -> FileTransferState {
    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Storage::FileSystem::{
        GetFileAttributesW, FILE_ATTRIBUTE_PINNED, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        FILE_ATTRIBUTE_UNPINNED, INVALID_FILE_ATTRIBUTES,
    };
    use windows::Win32::UI::Shell::{IShellItem2, SHCreateItemFromParsingName};
    use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

    // PKEY_StorageProviderState = {E77E90DF-6271-4F5B-834F-2DD1F245DDA4}, pid 2
    const PKEY_STORAGE_PROVIDER_STATE: PROPERTYKEY = PROPERTYKEY {
        fmtid: GUID::from_u128(0xE77E90DF_6271_4F5B_834F_2DD1F245DDA4),
        pid: 2,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    if let Ok(item) = unsafe { SHCreateItemFromParsingName::<_, _, IShellItem2>(PCWSTR(wide.as_ptr()), None) } {
        if let Ok(state_val) = unsafe { item.GetUInt32(&PKEY_STORAGE_PROVIDER_STATE) } {
            // Windows Shell StorageProviderState values:
            // 1 = Sparse/CloudOnly, 2 = InSync, 3 = InSync (Pinned),
            // 4 = Pending/Syncing, 5 = Transferring/Syncing,
            // 6 = Error, 7 = Warning, 9 = Excluded, 10 = InSync (Partial/Mixed)
            return match state_val {
                1 => FileTransferState::CloudOnly,
                2 | 3 | 10 => FileTransferState::Synced,
                4 => FileTransferState::Pending,
                5 => FileTransferState::Syncing,
                6 | 7 => FileTransferState::Error,
                _ => FileTransferState::Synced,
            };
        }
    }

    // Fallback to cloud file attributes
    let attrs = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
    if attrs != INVALID_FILE_ATTRIBUTES {
        if (attrs & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0) != 0
            || (attrs & FILE_ATTRIBUTE_UNPINNED.0) != 0
        {
            return FileTransferState::CloudOnly;
        }
        if (attrs & FILE_ATTRIBUTE_PINNED.0) != 0 {
            return FileTransferState::Synced;
        }
    }

    FileTransferState::Synced
}

#[cfg(not(windows))]
fn detect_file_sync_state(_path: &Path) -> FileTransferState {
    FileTransferState::Synced
}
