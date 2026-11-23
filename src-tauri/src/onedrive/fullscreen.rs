#[cfg(windows)]
pub fn is_fullscreen_app_active() -> bool {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect, IsWindowVisible,
    };

    unsafe {
        let Ok(state) = SHQueryUserNotificationState() else {
            return false;
        };

        if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE {
            return true;
        }

        if state != QUNS_BUSY {
            return false;
        }

        // QUNS_BUSY can trigger on Focus Assist, lock screen, or idle sessions.
        // Confirm an actual visible non-shell foreground window covers the full monitor.
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() || !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let mut class_buf = [0u16; 128];
        let len = GetClassNameW(hwnd, &mut class_buf);
        if len > 0 {
            let class_name = OsString::from_wide(&class_buf[..len as usize])
                .to_string_lossy()
                .to_ascii_lowercase();
            if matches!(
                class_name.as_str(),
                "progman"
                    | "workerw"
                    | "shell_traywnd"
                    | "shell_secondarytraywnd"
                    | "windows.ui.core.corewindow"
                    | "applicationframewindow"
            ) {
                return false;
            }
        }

        let mut win_rect = RECT::default();
        if GetWindowRect(hwnd, &mut win_rect).is_err() {
            return false;
        }

        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mon_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(hmon, &mut mon_info).as_bool() {
            return false;
        }

        let mon = mon_info.rcMonitor;
        win_rect.left <= mon.left
            && win_rect.top <= mon.top
            && win_rect.right >= mon.right
            && win_rect.bottom >= mon.bottom
    }
}

#[cfg(not(windows))]
pub fn is_fullscreen_app_active() -> bool {
    false
}
