use crate::models::AppSettings;
use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, WebviewWindow};

// Outer window dimensions include asymmetric transparent shadow halo padding
// (20px top, 24px left/right, 30px bottom) exceeding 1.5x Skia Gaussian blur tail.
pub const PILL_WIDTH_LOGICAL: f64 = 324.0;
pub const PILL_HEIGHT_LOGICAL: f64 = 96.0;
pub const CARD_WIDTH_LOGICAL: f64 = 416.0;
pub const CARD_DEFAULT_HEIGHT_LOGICAL: f64 = 506.0;
const EDGE_MARGIN_LOGICAL: f64 = 0.0;
const SNAP_THRESHOLD_LOGICAL: f64 = 26.0;

#[cfg(windows)]
pub fn is_left_mouse_button_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0 }
}

#[cfg(not(windows))]
pub fn is_left_mouse_button_down() -> bool {
    false
}

pub fn is_window_in_lower_half(window: &WebviewWindow) -> bool {
    let Ok(pos) = window.outer_position() else {
        return true;
    };
    let Ok(size) = window.outer_size() else {
        return true;
    };
    if let Some(work) = current_monitor_work_area(window) {
        let center_y = pos.y + (size.height as i32) / 2;
        center_y > work.top + work.height() / 2
    } else {
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WorkArea {
    pub fn width(self) -> i32 {
        (self.right - self.left).max(1)
    }

    pub fn height(self) -> i32 {
        (self.bottom - self.top).max(1)
    }
}

#[cfg(windows)]
pub fn apply_toolwindow_style(window: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0);
        unsafe {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let updated = (ex_style | WS_EX_TOOLWINDOW.0 as i32) & !(WS_EX_APPWINDOW.0 as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, updated);
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn apply_toolwindow_style(_window: &WebviewWindow) {}

#[cfg(windows)]
pub fn current_monitor_work_area(window: &WebviewWindow) -> Option<WorkArea> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    let hwnd_raw = window.hwnd().ok()?;
    let hwnd = HWND(hwnd_raw.0);
    unsafe {
        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            Some(WorkArea {
                left: info.rcWork.left,
                top: info.rcWork.top,
                right: info.rcWork.right,
                bottom: info.rcWork.bottom,
            })
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn current_monitor_work_area(_window: &WebviewWindow) -> Option<WorkArea> {
    None
}

#[cfg(windows)]
pub fn work_area_for_point(x: i32, y: i32) -> Option<WorkArea> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    unsafe {
        let hmon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            Some(WorkArea {
                left: info.rcWork.left,
                top: info.rcWork.top,
                right: info.rcWork.right,
                bottom: info.rcWork.bottom,
            })
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn work_area_for_point(_x: i32, _y: i32) -> Option<WorkArea> {
    None
}

pub fn default_bottom_right_position(window: &WebviewWindow) -> PhysicalPosition<i32> {
    let scale = window.scale_factor().unwrap_or(1.0);
    let win_w = (PILL_WIDTH_LOGICAL * scale).round() as i32;
    let win_h = (PILL_HEIGHT_LOGICAL * scale).round() as i32;
    let margin = (EDGE_MARGIN_LOGICAL * scale).round() as i32;

    if let Some(work) = current_monitor_work_area(window) {
        return PhysicalPosition::new(
            (work.right - win_w - margin).max(work.left + margin),
            (work.bottom - win_h - margin).max(work.top + margin),
        );
    }

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let pos = monitor.position();
        let size = monitor.size();
        let taskbar_est = (48.0 * scale).round() as i32;
        return PhysicalPosition::new(
            pos.x + size.width as i32 - win_w - margin,
            pos.y + size.height as i32 - win_h - taskbar_est - margin,
        );
    }

    PhysicalPosition::new(800, 600)
}

pub fn position_initial_window(window: &WebviewWindow, settings: &AppSettings) {
    apply_toolwindow_style(window);
    let _ = window.set_size(LogicalSize::new(PILL_WIDTH_LOGICAL, PILL_HEIGHT_LOGICAL));

    let pos = match (settings.position_x, settings.position_y) {
        (Some(x), Some(y)) => clamp_to_work_area(window, x, y),
        _ => default_bottom_right_position(window),
    };
    let _ = window.set_position(pos);
}

pub fn reset_window_position(window: &WebviewWindow) -> PhysicalPosition<i32> {
    let _ = window.set_size(LogicalSize::new(PILL_WIDTH_LOGICAL, PILL_HEIGHT_LOGICAL));
    let pos = default_bottom_right_position(window);
    let _ = window.set_position(pos);
    pos
}

pub fn clamp_to_work_area(window: &WebviewWindow, x: i32, y: i32) -> PhysicalPosition<i32> {
    let size = window
        .outer_size()
        .unwrap_or(PhysicalSize::new(PILL_WIDTH_LOGICAL as u32, PILL_HEIGHT_LOGICAL as u32));
    let scale = window.scale_factor().unwrap_or(1.0);
    let margin = (EDGE_MARGIN_LOGICAL * scale).round() as i32;

    let work = work_area_for_point(x + (size.width as i32) / 2, y + (size.height as i32) / 2)
        .or_else(|| current_monitor_work_area(window));

    if let Some(work) = work {
        let min_x = work.left + margin;
        let max_x = (work.right - size.width as i32 - margin).max(min_x);
        let min_y = work.top + margin;
        let max_y = (work.bottom - size.height as i32 - margin).max(min_y);
        PhysicalPosition::new(x.clamp(min_x, max_x), y.clamp(min_y, max_y))
    } else {
        PhysicalPosition::new(x, y)
    }
}

/// Snaps window to nearest monitor work-area edges when dropped close to an edge,
/// and clamps it safely inside the visible work area.
pub fn snap_and_clamp_after_drag(window: &WebviewWindow) -> PhysicalPosition<i32> {
    let Ok(pos) = window.outer_position() else {
        return default_bottom_right_position(window);
    };
    let Ok(size) = window.outer_size() else {
        return pos;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let margin = (EDGE_MARGIN_LOGICAL * scale).round() as i32;
    let snap = (SNAP_THRESHOLD_LOGICAL * scale).round() as i32;

    let Some(work) = current_monitor_work_area(window) else {
        return pos;
    };

    let min_x = work.left + margin;
    let max_x = (work.right - size.width as i32 - margin).max(min_x);
    let min_y = work.top + margin;
    let max_y = (work.bottom - size.height as i32 - margin).max(min_y);

    let mut x = pos.x.clamp(min_x, max_x);
    let mut y = pos.y.clamp(min_y, max_y);

    if (x - min_x).abs() <= snap {
        x = min_x;
    } else if (max_x - x).abs() <= snap {
        x = max_x;
    }

    if (y - min_y).abs() <= snap {
        y = min_y;
    } else if (max_y - y).abs() <= snap {
        y = max_y;
    }

    let final_pos = PhysicalPosition::new(x, y);
    if final_pos.x != pos.x || final_pos.y != pos.y {
        let _ = window.set_position(final_pos);
    }
    final_pos
}

/// Resizes between compact pill and expanded detail card while anchoring smartly to
/// the nearest screen corner (expands upward when near bottom, leftward when near right).
/// Returns whether the panel expanded upward (`true`) or downward (`false`).
pub fn resize_with_smart_anchor(
    window: &WebviewWindow,
    expanded: bool,
    requested_height_logical: Option<f64>,
) -> bool {
    let scale = window.scale_factor().unwrap_or(1.0);
    let old_pos = window
        .outer_position()
        .unwrap_or_else(|_| default_bottom_right_position(window));
    let old_size = window
        .outer_size()
        .unwrap_or(PhysicalSize::new(PILL_WIDTH_LOGICAL as u32, PILL_HEIGHT_LOGICAL as u32));

    let (target_w_log, target_h_log) = if expanded {
        let h = requested_height_logical
            .unwrap_or(CARD_DEFAULT_HEIGHT_LOGICAL)
            .clamp(340.0, 620.0);
        (CARD_WIDTH_LOGICAL, h)
    } else {
        (PILL_WIDTH_LOGICAL, PILL_HEIGHT_LOGICAL)
    };

    let new_w = (target_w_log * scale).round() as i32;
    let new_h = (target_h_log * scale).round() as i32;
    let margin = (EDGE_MARGIN_LOGICAL * scale).round() as i32;

    let work = current_monitor_work_area(window);
    let (anchor_right, anchor_bottom) = if let Some(w) = work {
        let center_x = old_pos.x + (old_size.width as i32) / 2;
        let center_y = old_pos.y + (old_size.height as i32) / 2;
        (
            center_x > w.left + w.width() / 2,
            center_y > w.top + w.height() / 2,
        )
    } else {
        (true, true)
    };

    let mut new_x = if anchor_right {
        old_pos.x + old_size.width as i32 - new_w
    } else {
        old_pos.x
    };
    let mut new_y = if anchor_bottom {
        old_pos.y + old_size.height as i32 - new_h
    } else {
        old_pos.y
    };

    if let Some(w) = work {
        let min_x = w.left + margin;
        let max_x = (w.right - new_w - margin).max(min_x);
        let min_y = w.top + margin;
        let max_y = (w.bottom - new_h - margin).max(min_y);
        new_x = new_x.clamp(min_x, max_x);
        new_y = new_y.clamp(min_y, max_y);
    }

    let _ = window.set_size(LogicalSize::new(target_w_log, target_h_log));
    let _ = window.set_position(PhysicalPosition::new(new_x, new_y));
    anchor_bottom
}
