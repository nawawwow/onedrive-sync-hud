use crate::models::AppSettings;
use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "OneDriveSyncHUD";
const SETTINGS_FILE_NAME: &str = "settings.json";
const RUN_VALUE_NAME: &str = "OneDriveSyncHUD";

pub fn settings_file_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join(APP_DIR_NAME).join(SETTINGS_FILE_NAME)
}

pub fn load_settings() -> AppSettings {
    let path = settings_file_path();
    let mut settings = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<AppSettings>(&raw).ok())
        .unwrap_or_default();

    settings.opacity = settings.opacity.clamp(0.55, 1.0);
    if !matches!(settings.theme.as_str(), "auto" | "light" | "dark") {
        settings.theme = "auto".to_string();
    }
    settings.start_with_windows = is_windows_startup_enabled();
    settings
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {e}"))?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write settings file: {e}"))?;
    Ok(())
}

#[cfg(windows)]
pub fn is_windows_startup_enabled() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    };

    let subkey: Vec<u16> = r"Software\Microsoft\Windows\CurrentVersion\Run"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let val_name: Vec<u16> = RUN_VALUE_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut hkey = HKEY::default();
    if unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
    } != ERROR_SUCCESS
    {
        return false;
    }

    let exists = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(val_name.as_ptr()),
            None,
            None,
            None,
            None,
        ) == ERROR_SUCCESS
    };
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    exists
}

#[cfg(windows)]
pub fn set_windows_startup(enabled: bool) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let subkey: Vec<u16> = r"Software\Microsoft\Windows\CurrentVersion\Run"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let val_name: Vec<u16> = RUN_VALUE_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut hkey = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("Failed to open Windows Run registry key: {status:?}"));
    }

    let result = if enabled {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Unable to resolve current executable path: {e}"))?;
        let quoted = format!("\"{}\"", exe.to_string_lossy());
        let wide_val: Vec<u16> = quoted.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = unsafe {
            std::slice::from_raw_parts(wide_val.as_ptr() as *const u8, wide_val.len() * 2)
        };
        let set_res = unsafe {
            RegSetValueExW(
                hkey,
                PCWSTR(val_name.as_ptr()),
                0,
                REG_SZ,
                Some(bytes),
            )
        };
        if set_res == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("Failed to set Run startup value: {set_res:?}"))
        }
    } else {
        let del_res = unsafe { RegDeleteValueW(hkey, PCWSTR(val_name.as_ptr())) };
        if del_res == ERROR_SUCCESS || del_res == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(format!("Failed to remove Run startup value: {del_res:?}"))
        }
    };

    unsafe {
        let _ = RegCloseKey(hkey);
    }
    result
}

#[cfg(not(windows))]
pub fn is_windows_startup_enabled() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_windows_startup(_enabled: bool) -> Result<(), String> {
    Ok(())
}
