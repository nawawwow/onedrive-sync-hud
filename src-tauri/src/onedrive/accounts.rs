use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DiscoveredAccount {
    pub id: String,
    pub display_name: String,
    pub is_business: bool,
    pub folder_path: String,
    pub sync_root_id: Option<String>,
    pub status_ui_factory_clsid: Option<String>,
    pub diagnostics_log_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct OneDriveRuntime {
    pub running: bool,
    pub pids: Vec<u32>,
    pub exe_path: Option<String>,
}

#[cfg(windows)]
mod win_impl {
    use super::{DiscoveredAccount, OneDriveRuntime};
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, HKEY,
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_EXPAND_SZ, REG_SZ,
        REG_VALUE_TYPE,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn read_reg_string(hkey: HKEY, value_name: &str) -> Option<String> {
        let wide_name = to_wide(value_name);
        let mut reg_type = REG_VALUE_TYPE(0);
        let mut size_bytes: u32 = 0;

        let status = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(wide_name.as_ptr()),
                None,
                Some(&mut reg_type),
                None,
                Some(&mut size_bytes),
            )
        };

        if status != ERROR_SUCCESS || size_bytes == 0 {
            return None;
        }
        if reg_type != REG_SZ && reg_type != REG_EXPAND_SZ {
            return None;
        }

        let mut buf: Vec<u16> = vec![0; (size_bytes as usize / 2) + 1];
        let status = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(wide_name.as_ptr()),
                None,
                Some(&mut reg_type),
                Some(buf.as_mut_ptr() as *mut u8),
                Some(&mut size_bytes),
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }

        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let val = OsString::from_wide(&buf[..len])
            .to_string_lossy()
            .trim()
            .to_string();
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
    }

    fn read_reg_dword(hkey: HKEY, value_name: &str) -> Option<u32> {
        let wide_name = to_wide(value_name);
        let mut reg_type = REG_VALUE_TYPE(0);
        let mut value: u32 = 0;
        let mut size_bytes: u32 = std::mem::size_of::<u32>() as u32;

        let status = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(wide_name.as_ptr()),
                None,
                Some(&mut reg_type),
                Some((&mut value as *mut u32) as *mut u8),
                Some(&mut size_bytes),
            )
        };
        if status == ERROR_SUCCESS && reg_type == REG_DWORD {
            Some(value)
        } else {
            None
        }
    }

    fn enum_subkeys(hkey: HKEY) -> Vec<String> {
        let mut result = Vec::new();
        let mut index = 0u32;
        loop {
            let mut name_buf = vec![0u16; 512];
            let mut name_len = name_buf.len() as u32;
            let status = unsafe {
                RegEnumKeyExW(
                    hkey,
                    index,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status == ERROR_SUCCESS {
                let s = OsString::from_wide(&name_buf[..name_len as usize])
                    .to_string_lossy()
                    .to_string();
                result.push(s);
            }
            index += 1;
        }
        result
    }

    fn enum_string_values(hkey: HKEY) -> Vec<(String, String)> {
        let mut result = Vec::new();
        let mut index = 0u32;
        loop {
            let mut name_buf = vec![0u16; 256];
            let mut name_len = name_buf.len() as u32;
            let mut reg_type: u32 = 0;
            let mut data_buf = vec![0u8; 2048];
            let mut data_len = data_buf.len() as u32;

            let status = unsafe {
                RegEnumValueW(
                    hkey,
                    index,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    Some(&mut reg_type),
                    Some(data_buf.as_mut_ptr()),
                    Some(&mut data_len),
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status == ERROR_SUCCESS
                && (REG_VALUE_TYPE(reg_type) == REG_SZ || REG_VALUE_TYPE(reg_type) == REG_EXPAND_SZ)
            {
                let key_name = OsString::from_wide(&name_buf[..name_len as usize])
                    .to_string_lossy()
                    .to_string();
                let u16_slice = unsafe {
                    std::slice::from_raw_parts(
                        data_buf.as_ptr() as *const u16,
                        (data_len as usize) / 2,
                    )
                };
                let val_len = u16_slice
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(u16_slice.len());
                let val = OsString::from_wide(&u16_slice[..val_len])
                    .to_string_lossy()
                    .trim()
                    .to_string();
                if !val.is_empty() {
                    result.push((key_name, val));
                }
            }
            index += 1;
        }
        result
    }

    fn normalize_folder(path: &str) -> String {
        path.trim()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    }

    struct SyncRootMeta {
        sync_root_id: String,
        factory_clsid: Option<String>,
        account_hint: Option<String>,
    }

    fn read_sync_roots_by_folder() -> HashMap<String, SyncRootMeta> {
        let mut map = HashMap::new();
        let root_path =
            to_wide(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager");
        let mut h_root = HKEY::default();
        let opened = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(root_path.as_ptr()),
                0,
                KEY_READ,
                &mut h_root,
            )
        };
        if opened != ERROR_SUCCESS {
            return map;
        }

        for subkey_name in enum_subkeys(h_root) {
            if !subkey_name.to_ascii_lowercase().contains("onedrive") {
                continue;
            }
            let wide_sub = to_wide(&subkey_name);
            let mut h_sub = HKEY::default();
            if unsafe { RegOpenKeyExW(h_root, PCWSTR(wide_sub.as_ptr()), 0, KEY_READ, &mut h_sub) }
                != ERROR_SUCCESS
            {
                continue;
            }

            let factory_clsid = read_reg_string(h_sub, "StorageProviderStatusUISourceFactory");
            let account_hint = subkey_name
                .split('!')
                .nth(2)
                .map(|part| part.split('|').next().unwrap_or(part).to_string());

            let wide_usr = to_wide("UserSyncRoots");
            let mut h_usr = HKEY::default();
            if unsafe { RegOpenKeyExW(h_sub, PCWSTR(wide_usr.as_ptr()), 0, KEY_READ, &mut h_usr) }
                == ERROR_SUCCESS
            {
                for (_sid, folder) in enum_string_values(h_usr) {
                    map.insert(
                        normalize_folder(&folder),
                        SyncRootMeta {
                            sync_root_id: subkey_name.clone(),
                            factory_clsid: factory_clsid.clone(),
                            account_hint: account_hint.clone(),
                        },
                    );
                }
                unsafe {
                    let _ = RegCloseKey(h_usr);
                }
            }

            unsafe {
                let _ = RegCloseKey(h_sub);
            }
        }

        unsafe {
            let _ = RegCloseKey(h_root);
        }
        map
    }

    pub fn discover_accounts() -> Vec<DiscoveredAccount> {
        let sync_roots = read_sync_roots_by_folder();
        let local_app_data = std::env::var("LOCALAPPDATA").ok().map(PathBuf::from);
        let mut accounts = Vec::new();
        let mut seen_folders = HashMap::<String, ()>::new();

        let acc_path = to_wide(r"Software\Microsoft\OneDrive\Accounts");
        let mut h_acc = HKEY::default();
        if unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(acc_path.as_ptr()),
                0,
                KEY_READ,
                &mut h_acc,
            )
        } == ERROR_SUCCESS
        {
            for subkey_name in enum_subkeys(h_acc) {
                let wide_sub = to_wide(&subkey_name);
                let mut h_sub = HKEY::default();
                if unsafe {
                    RegOpenKeyExW(h_acc, PCWSTR(wide_sub.as_ptr()), 0, KEY_READ, &mut h_sub)
                } != ERROR_SUCCESS
                {
                    continue;
                }

                let folder = read_reg_string(h_sub, "UserFolder");
                let display_raw = read_reg_string(h_sub, "DisplayName");
                let business_flag = read_reg_dword(h_sub, "Business").unwrap_or(0) == 1
                    || subkey_name.to_ascii_lowercase().starts_with("business");

                unsafe {
                    let _ = RegCloseKey(h_sub);
                }

                let Some(folder_path) = folder else {
                    continue;
                };
                let norm = normalize_folder(&folder_path);
                if seen_folders.contains_key(&norm) {
                    continue;
                }
                seen_folders.insert(norm.clone(), ());

                let meta = sync_roots.get(&norm);
                let display_name = match display_raw {
                    Some(name) if !name.is_empty() => {
                        if name.to_ascii_lowercase().contains("onedrive") {
                            name
                        } else {
                            format!("OneDrive - {name}")
                        }
                    }
                    _ => {
                        if business_flag {
                            format!("OneDrive - {subkey_name}")
                        } else {
                            "OneDrive - Personal".to_string()
                        }
                    }
                };

                let diagnostics_log_path = local_app_data.as_ref().map(|base| {
                    base.join("Microsoft")
                        .join("OneDrive")
                        .join("logs")
                        .join(&subkey_name)
                        .join("SyncDiagnostics.log")
                });

                accounts.push(DiscoveredAccount {
                    id: subkey_name,
                    display_name,
                    is_business: business_flag,
                    folder_path,
                    sync_root_id: meta.map(|m| m.sync_root_id.clone()),
                    status_ui_factory_clsid: meta.and_then(|m| m.factory_clsid.clone()),
                    diagnostics_log_path,
                });
            }

            unsafe {
                let _ = RegCloseKey(h_acc);
            }
        }

        // Fallback to environment variables if registry accounts were empty
        if accounts.is_empty() {
            for (env_var, id, is_business, label) in [
                ("OneDriveConsumer", "Personal", false, "OneDrive - Personal"),
                ("OneDriveCommercial", "Business1", true, "OneDrive - Work"),
                ("OneDrive", "Personal", false, "OneDrive"),
            ] {
                if let Ok(folder_path) = std::env::var(env_var) {
                    let norm = normalize_folder(&folder_path);
                    if seen_folders.contains_key(&norm) {
                        continue;
                    }
                    seen_folders.insert(norm.clone(), ());
                    let meta = sync_roots.get(&norm);
                    let log_key = meta
                        .and_then(|m| m.account_hint.clone())
                        .unwrap_or_else(|| id.to_string());
                    let diagnostics_log_path = local_app_data.as_ref().map(|base| {
                        base.join("Microsoft")
                            .join("OneDrive")
                            .join("logs")
                            .join(&log_key)
                            .join("SyncDiagnostics.log")
                    });
                    accounts.push(DiscoveredAccount {
                        id: log_key,
                        display_name: label.to_string(),
                        is_business,
                        folder_path,
                        sync_root_id: meta.map(|m| m.sync_root_id.clone()),
                        status_ui_factory_clsid: meta.and_then(|m| m.factory_clsid.clone()),
                        diagnostics_log_path,
                    });
                }
            }
        }

        // Sort Personal first, then Business accounts alphabetically
        accounts.sort_by(|a, b| a.is_business.cmp(&b.is_business).then(a.id.cmp(&b.id)));
        accounts
    }

    pub fn detect_onedrive_runtime() -> OneDriveRuntime {
        let mut pids = Vec::new();
        if let Ok(snapshot) = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
                loop {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let exe_name = OsString::from_wide(&entry.szExeFile[..len])
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    if exe_name == "onedrive.exe" {
                        pids.push(entry.th32ProcessID);
                    }
                    if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                        break;
                    }
                }
            }
            unsafe {
                let _ = CloseHandle(snapshot);
            }
        }

        let exe_path = find_onedrive_exe();
        OneDriveRuntime {
            running: !pids.is_empty(),
            pids,
            exe_path,
        }
    }

    pub fn find_onedrive_exe() -> Option<String> {
        let mut candidates = Vec::new();
        if let Ok(pf) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(pf).join("Microsoft OneDrive").join("OneDrive.exe"));
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(
                PathBuf::from(pf86)
                    .join("Microsoft OneDrive")
                    .join("OneDrive.exe"),
            );
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local)
                    .join("Microsoft")
                    .join("OneDrive")
                    .join("OneDrive.exe"),
            );
        }

        candidates
            .into_iter()
            .find(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn detect_windows_theme() -> String {
        let path = to_wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let mut hkey = HKEY::default();
        if unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                0,
                KEY_READ,
                &mut hkey,
            )
        } == ERROR_SUCCESS
        {
            let light = read_reg_dword(hkey, "AppsUseLightTheme").unwrap_or(1);
            unsafe {
                let _ = RegCloseKey(hkey);
            }
            if light == 0 {
                return "dark".to_string();
            }
        }
        "light".to_string()
    }
}

#[cfg(windows)]
pub use win_impl::{
    detect_onedrive_runtime, detect_windows_theme, discover_accounts, find_onedrive_exe,
};

#[cfg(not(windows))]
pub fn discover_accounts() -> Vec<DiscoveredAccount> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn detect_onedrive_runtime() -> OneDriveRuntime {
    OneDriveRuntime {
        running: false,
        pids: Vec::new(),
        exe_path: None,
    }
}

#[cfg(not(windows))]
pub fn detect_windows_theme() -> String {
    "light".to_string()
}

#[cfg(not(windows))]
pub fn find_onedrive_exe() -> Option<String> {
    None
}
