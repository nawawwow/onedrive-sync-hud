use crate::models::{AppSettings, SyncState};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

pub const TRAY_ID: &str = "onedrive-sync-hud-tray";

pub mod menu_ids {
    pub const TOGGLE_OVERLAY: &str = "toggle_overlay";
    pub const AUTO_HIDE_IDLE: &str = "auto_hide_idle";
    pub const HIDE_FULLSCREEN: &str = "hide_fullscreen";
    pub const LOCK_POSITION: &str = "lock_position";
    pub const THEME_AUTO: &str = "theme_auto";
    pub const THEME_LIGHT: &str = "theme_light";
    pub const THEME_DARK: &str = "theme_dark";
    pub const OPACITY_100: &str = "opacity_100";
    pub const OPACITY_90: &str = "opacity_90";
    pub const OPACITY_80: &str = "opacity_80";
    pub const OPACITY_70: &str = "opacity_70";
    pub const START_WITH_WINDOWS: &str = "start_with_windows";
    pub const RESET_POSITION: &str = "reset_position";
    pub const OPEN_ONEDRIVE_FOLDER: &str = "open_onedrive_folder";
    pub const QUIT: &str = "quit";
}

/// Generates a crisp 32x32 RGBA tray icon with a OneDrive cloud glyph and live status badge.
pub fn render_tray_icon(state: SyncState) -> Image<'static> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    let set_px = |buf: &mut [u8], x: i32, y: i32, color: [u8; 4]| {
        if x < 0 || y < 0 || x >= SIZE as i32 || y >= SIZE as i32 {
            return;
        }
        let idx = ((y as u32 * SIZE + x as u32) * 4) as usize;
        let alpha = color[3] as f32 / 255.0;
        let inv = 1.0 - alpha;
        buf[idx] = (color[0] as f32 * alpha + buf[idx] as f32 * inv) as u8;
        buf[idx + 1] = (color[1] as f32 * alpha + buf[idx + 1] as f32 * inv) as u8;
        buf[idx + 2] = (color[2] as f32 * alpha + buf[idx + 2] as f32 * inv) as u8;
        buf[idx + 3] = (color[3] as f32 + buf[idx + 3] as f32 * inv).min(255.0) as u8;
    };

    let fill_circle = |buf: &mut [u8], cx: f32, cy: f32, r: f32, color: [u8; 4]| {
        for y in 0..SIZE as i32 {
            for x in 0..SIZE as i32 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= r + 0.6 {
                    let cov = ((r + 0.6 - dist) / 1.2).clamp(0.0, 1.0);
                    let mut c = color;
                    c[3] = (c[3] as f32 * cov) as u8;
                    set_px(buf, x, y, c);
                }
            }
        }
    };

    // Cloud base palette (muted slate if offline/not running, OneDrive azure otherwise)
    let (cloud_back, cloud_front) = match state {
        SyncState::Offline | SyncState::NotRunning => ([120, 132, 148, 255], [164, 174, 188, 255]),
        _ => ([0, 103, 192, 255], [40, 168, 234, 255]),
    };

    // Back lobe
    fill_circle(&mut rgba, 11.5, 14.5, 6.2, cloud_back);
    // Main upper lobe
    fill_circle(&mut rgba, 18.5, 13.0, 6.8, cloud_front);
    // Side lobes
    fill_circle(&mut rgba, 8.2, 18.2, 4.7, cloud_front);
    fill_circle(&mut rgba, 23.0, 18.2, 4.7, cloud_front);
    // Flat bottom fill
    for y in 16..=22 {
        for x in 8..=23 {
            set_px(&mut rgba, x, y, cloud_front);
        }
    }

    // Status indicator dot (bottom-right) with dark ring for contrast
    let badge_color = match state {
        SyncState::InSync => [16, 185, 129, 255],       // Emerald green
        SyncState::Syncing => [56, 189, 248, 255],      // Bright sky blue
        SyncState::Paused => [245, 158, 11, 255],       // Amber
        SyncState::Warning => [245, 158, 11, 255],      // Amber
        SyncState::Error => [239, 68, 68, 255],         // Red
        SyncState::Offline | SyncState::NotRunning | SyncState::Unknown => [148, 163, 184, 255],
    };

    fill_circle(&mut rgba, 24.5, 24.5, 5.6, [18, 22, 28, 235]);
    fill_circle(&mut rgba, 24.5, 24.5, 4.1, badge_color);

    Image::new_owned(rgba, SIZE, SIZE)
}

pub fn build_tray_menu(app: &AppHandle, settings: &AppSettings) -> tauri::Result<Menu<Wry>> {
    let toggle_item = MenuItem::with_id(
        app,
        menu_ids::TOGGLE_OVERLAY,
        "Show / Hide HUD (Ctrl+Alt+O)",
        true,
        None::<&str>,
    )?;

    let auto_hide = CheckMenuItem::with_id(
        app,
        menu_ids::AUTO_HIDE_IDLE,
        "Auto-hide when idle",
        true,
        settings.auto_hide_when_idle,
        None::<&str>,
    )?;

    let hide_fs = CheckMenuItem::with_id(
        app,
        menu_ids::HIDE_FULLSCREEN,
        "Auto-hide in fullscreen",
        true,
        settings.hide_in_fullscreen,
        None::<&str>,
    )?;

    let lock_pos = CheckMenuItem::with_id(
        app,
        menu_ids::LOCK_POSITION,
        "Lock position",
        true,
        settings.locked,
        None::<&str>,
    )?;

    let theme_auto = CheckMenuItem::with_id(
        app,
        menu_ids::THEME_AUTO,
        "Auto (Windows)",
        true,
        settings.theme == "auto",
        None::<&str>,
    )?;
    let theme_light = CheckMenuItem::with_id(
        app,
        menu_ids::THEME_LIGHT,
        "Light",
        true,
        settings.theme == "light",
        None::<&str>,
    )?;
    let theme_dark = CheckMenuItem::with_id(
        app,
        menu_ids::THEME_DARK,
        "Dark",
        true,
        settings.theme == "dark",
        None::<&str>,
    )?;
    let theme_submenu = Submenu::with_items(
        app,
        "Theme",
        true,
        &[&theme_auto, &theme_light, &theme_dark],
    )?;

    let op100 = CheckMenuItem::with_id(
        app,
        menu_ids::OPACITY_100,
        "100%",
        true,
        settings.opacity >= 0.95,
        None::<&str>,
    )?;
    let op90 = CheckMenuItem::with_id(
        app,
        menu_ids::OPACITY_90,
        "90%",
        true,
        (0.85..0.95).contains(&settings.opacity),
        None::<&str>,
    )?;
    let op80 = CheckMenuItem::with_id(
        app,
        menu_ids::OPACITY_80,
        "80%",
        true,
        (0.75..0.85).contains(&settings.opacity),
        None::<&str>,
    )?;
    let op70 = CheckMenuItem::with_id(
        app,
        menu_ids::OPACITY_70,
        "70%",
        true,
        settings.opacity < 0.75,
        None::<&str>,
    )?;
    let opacity_submenu = Submenu::with_items(
        app,
        "Opacity",
        true,
        &[&op100, &op90, &op80, &op70],
    )?;

    let startup = CheckMenuItem::with_id(
        app,
        menu_ids::START_WITH_WINDOWS,
        "Start with Windows",
        true,
        settings.start_with_windows,
        None::<&str>,
    )?;

    let reset_pos = MenuItem::with_id(
        app,
        menu_ids::RESET_POSITION,
        "Reset position",
        true,
        None::<&str>,
    )?;

    let open_folder = MenuItem::with_id(
        app,
        menu_ids::OPEN_ONEDRIVE_FOLDER,
        "Open OneDrive folder",
        true,
        None::<&str>,
    )?;

    let quit = MenuItem::with_id(app, menu_ids::QUIT, "Quit", true, None::<&str>)?;

    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &toggle_item,
            &sep1,
            &auto_hide,
            &hide_fs,
            &lock_pos,
            &theme_submenu,
            &opacity_submenu,
            &sep2,
            &startup,
            &reset_pos,
            &open_folder,
            &sep3,
            &quit,
        ],
    )
}

pub fn create_tray<F, G>(
    app: &AppHandle,
    settings: &AppSettings,
    on_menu: F,
    on_left_click: G,
) -> tauri::Result<TrayIcon<Wry>>
where
    F: Fn(&AppHandle, &str) + Send + Sync + 'static,
    G: Fn(&AppHandle) + Send + Sync + 'static,
{
    let menu = build_tray_menu(app, settings)?;
    let icon = render_tray_icon(SyncState::Unknown);

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("OneDrive Sync HUD — Checking status...")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| {
            on_menu(app_handle, event.id.as_ref());
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_left_click(tray.app_handle());
            }
        })
        .build(app)
}

pub fn refresh_tray_visuals(
    app: &AppHandle,
    state: SyncState,
    tooltip: &str,
    settings: Option<&AppSettings>,
) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(render_tray_icon(state)));
        let _ = tray.set_tooltip(Some(tooltip));
        if let Some(s) = settings {
            if let Ok(menu) = build_tray_menu(app, s) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}
