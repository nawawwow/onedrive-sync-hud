mod models;
mod onedrive;
mod settings;
mod tray;
mod window_ctrl;

use models::{AppSettings, HudSnapshot, SyncState};
use onedrive::accounts::DiscoveredAccount;
use onedrive::status_ui::invoke_provider_command;
use onedrive::TelemetryEngine;
use serde::Deserialize;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

struct VisibilityController {
    expanded: bool,
    hovered: bool,
    manually_hidden: bool,
    manual_show_until: Option<Instant>,
    dismissed_attention_state: Option<SyncState>,
    idle_since: Option<Instant>,
    last_state: SyncState,
    window_visible: bool,
    ignore_moves_until: Option<Instant>,
    pending_drag_settle_at: Option<Instant>,
}

impl Default for VisibilityController {
    fn default() -> Self {
        Self {
            expanded: false,
            hovered: false,
            manually_hidden: false,
            // Show briefly on startup so the user knows the HUD is live
            manual_show_until: Some(Instant::now() + Duration::from_secs(7)),
            dismissed_attention_state: None,
            idle_since: None,
            last_state: SyncState::Unknown,
            window_visible: false,
            ignore_moves_until: Some(Instant::now() + Duration::from_millis(600)),
            pending_drag_settle_at: None,
        }
    }
}

struct AppState {
    settings: Mutex<AppSettings>,
    snapshot: Mutex<HudSnapshot>,
    visibility: Mutex<VisibilityController>,
    accounts: Mutex<Vec<DiscoveredAccount>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    auto_hide_when_idle: Option<bool>,
    idle_hide_delay_secs: Option<u64>,
    hide_in_fullscreen: Option<bool>,
    locked: Option<bool>,
    opacity: Option<f64>,
    theme: Option<String>,
    start_with_windows: Option<bool>,
}

fn empty_snapshot() -> HudSnapshot {
    HudSnapshot {
        onedrive_running: true,
        onedrive_exe_path: None,
        aggregate_state: SyncState::Unknown,
        aggregate_label: "Checking OneDrive...".to_string(),
        aggregate_summary: "Connecting to Windows Sync Provider".to_string(),
        aggregate_progress_percent: None,
        total_speed_bps: 0,
        upload_speed_bps: 0,
        download_speed_bps: 0,
        total_files_remaining: 0,
        total_bytes_remaining: 0,
        total_eta_seconds: None,
        speed_history_bps: vec![0; 30],
        system_theme: onedrive::accounts::detect_windows_theme(),
        is_fullscreen: false,
        accounts: Vec::new(),
        updated_at_ms: 0,
    }
}

fn format_tooltip(snap: &HudSnapshot) -> String {
    if !snap.onedrive_running {
        return "OneDrive Sync HUD — OneDrive not running".to_string();
    }
    let pct_part = snap
        .aggregate_progress_percent
        .filter(|_| snap.aggregate_state == SyncState::Syncing)
        .map(|p| format!(" ({:.0}%)", p))
        .unwrap_or_default();
    format!(
        "OneDrive Sync HUD — {}{}\n{}",
        snap.aggregate_label, pct_part, snap.aggregate_summary
    )
}

fn evaluate_visibility(app: &AppHandle, state: &AppState) {
    let settings = state
        .settings
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let snap = state
        .snapshot
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| empty_snapshot());

    let should_show = {
        let Ok(mut vis) = state.visibility.lock() else {
            return;
        };
        let now = Instant::now();
        let current_state = snap.aggregate_state;

        // Track state transitions
        if current_state != vis.last_state {
            // New active sync or attention state clears previous manual dismiss/hide
            if current_state.is_active() || current_state.is_attention_required() {
                vis.manually_hidden = false;
                if vis.dismissed_attention_state != Some(current_state) {
                    vis.dismissed_attention_state = None;
                }
            }
            if current_state == SyncState::InSync {
                vis.idle_since = Some(now);
            } else {
                vis.idle_since = None;
            }
            vis.last_state = current_state;
        } else if current_state == SyncState::InSync && vis.idle_since.is_none() {
            vis.idle_since = Some(now);
        }

        // 1. Fullscreen suppression or explicit user hide always wins unless expanded
        if (settings.hide_in_fullscreen && snap.is_fullscreen && !vis.expanded)
            || vis.manually_hidden
        {
            false
        } else if vis.expanded
            || vis.hovered
            || !settings.auto_hide_when_idle
            || current_state.is_active()
            || vis.manual_show_until.is_some_and(|t| now < t)
        {
            true
        } else if current_state.is_attention_required() {
            vis.dismissed_attention_state != Some(current_state)
        } else if let Some(since) = vis.idle_since {
            now.duration_since(since) < Duration::from_secs(settings.idle_hide_delay_secs)
        } else {
            true
        }
    };

    if let Some(window) = app.get_webview_window("main") {
        let mut changed = false;
        if let Ok(mut vis) = state.visibility.lock() {
            if vis.window_visible != should_show {
                vis.window_visible = should_show;
                changed = true;
            }
        }
        if changed {
            if should_show {
                window_ctrl::apply_toolwindow_style(&window);
                let _ = window.show();
                let _ = app.emit("hud-visibility", true);
            } else {
                let _ = app.emit("hud-visibility", false);
                let _ = window.hide();
            }
        }
    }
}

fn toggle_overlay_manual(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = Instant::now();
    {
        let Ok(mut vis) = state.visibility.lock() else {
            return;
        };
        if vis.window_visible {
            vis.manually_hidden = true;
            vis.expanded = false;
            vis.manual_show_until = None;
            vis.ignore_moves_until = Some(now + Duration::from_millis(450));
            vis.pending_drag_settle_at = None;
            if let Some(window) = app.get_webview_window("main") {
                window_ctrl::resize_with_smart_anchor(&window, false, None);
            }
            let _ = app.emit("hud-expanded-changed", false);
        } else {
            vis.manually_hidden = false;
            vis.dismissed_attention_state = None;
            vis.manual_show_until = Some(now + Duration::from_secs(12));
        }
    }
    evaluate_visibility(app, &state);
}

fn apply_settings_update(app: &AppHandle, updated: AppSettings) -> Result<AppSettings, String> {
    let state = app.state::<AppState>();
    {
        let mut guard = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned".to_string())?;
        *guard = updated.clone();
    }
    settings::save_settings(&updated)?;
    let _ = app.emit("hud-settings", &updated);

    let snap = state
        .snapshot
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| empty_snapshot());
    tray::refresh_tray_visuals(
        app,
        snap.aggregate_state,
        &format_tooltip(&snap),
        Some(&updated),
    );
    evaluate_visibility(app, &state);
    Ok(updated)
}

fn handle_tray_menu(app: &AppHandle, id: &str) {
    match id {
        tray::menu_ids::TOGGLE_OVERLAY => toggle_overlay_manual(app),
        tray::menu_ids::AUTO_HIDE_IDLE => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
            s.auto_hide_when_idle = !s.auto_hide_when_idle;
            if !s.auto_hide_when_idle {
                if let Ok(mut vis) = state.visibility.lock() {
                    vis.manually_hidden = false;
                }
            }
            let _ = apply_settings_update(app, s);
        }
        tray::menu_ids::HIDE_FULLSCREEN => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
            s.hide_in_fullscreen = !s.hide_in_fullscreen;
            let _ = apply_settings_update(app, s);
        }
        tray::menu_ids::LOCK_POSITION => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
            s.locked = !s.locked;
            let _ = apply_settings_update(app, s);
        }
        tray::menu_ids::THEME_AUTO | tray::menu_ids::THEME_LIGHT | tray::menu_ids::THEME_DARK => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
            s.theme = match id {
                tray::menu_ids::THEME_LIGHT => "light".to_string(),
                tray::menu_ids::THEME_DARK => "dark".to_string(),
                _ => "auto".to_string(),
            };
            let _ = apply_settings_update(app, s);
        }
        tray::menu_ids::OPACITY_100
        | tray::menu_ids::OPACITY_90
        | tray::menu_ids::OPACITY_80
        | tray::menu_ids::OPACITY_70 => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
            s.opacity = match id {
                tray::menu_ids::OPACITY_90 => 0.9,
                tray::menu_ids::OPACITY_80 => 0.8,
                tray::menu_ids::OPACITY_70 => 0.7,
                _ => 1.0,
            };
            let _ = apply_settings_update(app, s);
        }
        tray::menu_ids::START_WITH_WINDOWS => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
            let target = !s.start_with_windows;
            if settings::set_windows_startup(target).is_ok() {
                s.start_with_windows = target;
                let _ = apply_settings_update(app, s);
            }
        }
        tray::menu_ids::RESET_POSITION => {
            let state = app.state::<AppState>();
            if let Some(window) = app.get_webview_window("main") {
                mark_programmatic_move(&state, 500);
                if let Ok(mut vis) = state.visibility.lock() {
                    vis.expanded = false;
                    vis.hovered = false;
                    vis.manually_hidden = false;
                    vis.manual_show_until = Some(Instant::now() + Duration::from_secs(10));
                }
                let pos = window_ctrl::reset_window_position(&window);
                let mut s = state.settings.lock().map(|g| g.clone()).unwrap_or_default();
                s.position_x = Some(pos.x);
                s.position_y = Some(pos.y);
                let _ = app.emit("hud-expanded-changed", false);
                let _ = app.emit("hud-anchor-changed", true);
                let _ = apply_settings_update(app, s);
            }
        }
        tray::menu_ids::OPEN_ONEDRIVE_FOLDER => {
            let state = app.state::<AppState>();
            let folder = state
                .accounts
                .lock()
                .ok()
                .and_then(|list| list.first().map(|a| a.folder_path.clone()));
            if let Some(path) = folder {
                let _ = Command::new("explorer.exe").arg(path).spawn();
            }
        }
        tray::menu_ids::QUIT => {
            app.exit(0);
        }
        _ => {}
    }
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> HudSnapshot {
    state
        .snapshot
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| empty_snapshot())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state
        .settings
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<AppSettings, String> {
    let mut current = state
        .settings
        .lock()
        .map(|g| g.clone())
        .map_err(|_| "Settings lock poisoned".to_string())?;

    if let Some(v) = patch.auto_hide_when_idle {
        current.auto_hide_when_idle = v;
        if !v {
            if let Ok(mut vis) = state.visibility.lock() {
                vis.manually_hidden = false;
            }
        }
    }
    if let Some(v) = patch.idle_hide_delay_secs {
        current.idle_hide_delay_secs = v.clamp(2, 60);
    }
    if let Some(v) = patch.hide_in_fullscreen {
        current.hide_in_fullscreen = v;
    }
    if let Some(v) = patch.locked {
        current.locked = v;
    }
    if let Some(v) = patch.opacity {
        current.opacity = v.clamp(0.55, 1.0);
    }
    if let Some(v) = patch.theme {
        if matches!(v.as_str(), "auto" | "light" | "dark") {
            current.theme = v;
        }
    }
    if let Some(v) = patch.start_with_windows {
        settings::set_windows_startup(v)?;
        current.start_with_windows = v;
    }

    apply_settings_update(&app, current)
}

fn mark_programmatic_move(state: &AppState, millis: u64) {
    if let Ok(mut vis) = state.visibility.lock() {
        vis.ignore_moves_until = Some(Instant::now() + Duration::from_millis(millis));
        vis.pending_drag_settle_at = None;
    }
}

fn settle_finished_drag_if_ready(app: &AppHandle, state: &AppState) {
    let should_settle = {
        let Ok(vis) = state.visibility.lock() else {
            return;
        };
        let Some(settle_at) = vis.pending_drag_settle_at else {
            return;
        };
        Instant::now() >= settle_at && !window_ctrl::is_left_mouse_button_down()
    };

    if !should_settle {
        return;
    }

    if let Ok(mut vis) = state.visibility.lock() {
        vis.pending_drag_settle_at = None;
        vis.ignore_moves_until = Some(Instant::now() + Duration::from_millis(450));
    }

    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let snapped = window_ctrl::snap_and_clamp_after_drag(&window);
    let lower_half = window_ctrl::is_window_in_lower_half(&window);
    let _ = app.emit("hud-anchor-changed", lower_half);

    let mut current = state
        .settings
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    if current.position_x != Some(snapped.x) || current.position_y != Some(snapped.y) {
        current.position_x = Some(snapped.x);
        current.position_y = Some(snapped.y);
        let _ = apply_settings_update(app, current);
    }
}

#[tauri::command]
fn set_expanded(
    app: AppHandle,
    state: State<'_, AppState>,
    expanded: bool,
    target_height: Option<f64>,
) -> bool {
    mark_programmatic_move(&state, 450);
    let mut expanded_upward = true;
    if let Some(window) = app.get_webview_window("main") {
        expanded_upward = window_ctrl::resize_with_smart_anchor(&window, expanded, target_height);
    }
    if let Ok(mut vis) = state.visibility.lock() {
        vis.expanded = expanded;
        if !expanded {
            vis.manual_show_until = Some(Instant::now() + Duration::from_secs(4));
        }
    }
    let _ = app.emit("hud-anchor-changed", expanded_upward);
    evaluate_visibility(&app, &state);
    expanded_upward
}

#[tauri::command]
fn set_hovered(app: AppHandle, state: State<'_, AppState>, hovered: bool) {
    if let Ok(mut vis) = state.visibility.lock() {
        vis.hovered = hovered;
    }
    evaluate_visibility(&app, &state);
}

#[tauri::command]
fn finish_drag(app: AppHandle, state: State<'_, AppState>) -> Result<(i32, i32), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("Main overlay window not found".to_string());
    };
    mark_programmatic_move(&state, 450);
    let snapped = window_ctrl::snap_and_clamp_after_drag(&window);
    let lower_half = window_ctrl::is_window_in_lower_half(&window);
    let _ = app.emit("hud-anchor-changed", lower_half);
    let mut current = state
        .settings
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    current.position_x = Some(snapped.x);
    current.position_y = Some(snapped.y);
    let _ = apply_settings_update(&app, current);
    Ok((snapped.x, snapped.y))
}

#[tauri::command]
fn dismiss_hud(app: AppHandle, state: State<'_, AppState>) {
    let current_sync_state = state
        .snapshot
        .lock()
        .map(|g| g.aggregate_state)
        .unwrap_or(SyncState::InSync);

    mark_programmatic_move(&state, 450);
    if let Some(window) = app.get_webview_window("main") {
        window_ctrl::resize_with_smart_anchor(&window, false, None);
    }
    if let Ok(mut vis) = state.visibility.lock() {
        vis.expanded = false;
        vis.hovered = false;
        vis.manual_show_until = None;
        if current_sync_state.is_attention_required() {
            vis.dismissed_attention_state = Some(current_sync_state);
        } else {
            vis.manually_hidden = true;
        }
    }
    let _ = app.emit("hud-expanded-changed", false);
    evaluate_visibility(&app, &state);
}

#[tauri::command]
fn reset_position(app: AppHandle, state: State<'_, AppState>) -> Result<(i32, i32), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("Main overlay window not found".to_string());
    };
    mark_programmatic_move(&state, 500);
    if let Ok(mut vis) = state.visibility.lock() {
        vis.expanded = false;
        vis.hovered = false;
        vis.manually_hidden = false;
        vis.manual_show_until = Some(Instant::now() + Duration::from_secs(10));
    }
    let pos = window_ctrl::reset_window_position(&window);
    let mut current = state
        .settings
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    current.position_x = Some(pos.x);
    current.position_y = Some(pos.y);
    let _ = app.emit("hud-expanded-changed", false);
    let _ = app.emit("hud-anchor-changed", true);
    let _ = apply_settings_update(&app, current);
    Ok((pos.x, pos.y))
}

#[tauri::command]
fn open_onedrive_folder(folder_path: String) -> Result<(), String> {
    if folder_path.trim().is_empty() {
        return Err("Folder path is empty".to_string());
    }
    Command::new("explorer.exe")
        .arg(&folder_path)
        .spawn()
        .map_err(|e| format!("Failed to open Explorer: {e}"))?;
    Ok(())
}

#[tauri::command]
fn reveal_file_in_explorer(file_path: String) -> Result<(), String> {
    if file_path.trim().is_empty() {
        return Err("File path is empty".to_string());
    }
    Command::new("explorer.exe")
        .arg(format!("/select,\"{file_path}\""))
        .spawn()
        .map_err(|e| format!("Failed to reveal file in Explorer: {e}"))?;
    Ok(())
}

#[tauri::command]
fn launch_onedrive(state: State<'_, AppState>) -> Result<(), String> {
    let exe = state
        .snapshot
        .lock()
        .ok()
        .and_then(|s| s.onedrive_exe_path.clone())
        .or_else(onedrive::accounts::find_onedrive_exe);
    let Some(path) = exe else {
        return Err("OneDrive.exe could not be located on this machine".to_string());
    };
    Command::new(path)
        .spawn()
        .map_err(|e| format!("Failed to launch OneDrive: {e}"))?;
    Ok(())
}

#[tauri::command]
fn open_onedrive_flyout(state: State<'_, AppState>) -> Result<(), String> {
    let exe = state
        .snapshot
        .lock()
        .ok()
        .and_then(|s| s.onedrive_exe_path.clone())
        .or_else(onedrive::accounts::find_onedrive_exe);
    let Some(path) = exe else {
        return Err("OneDrive.exe could not be located on this machine".to_string());
    };
    // Running OneDrive.exe when already active brings up OneDrive / opens the OneDrive root
    Command::new(path)
        .spawn()
        .map_err(|e| format!("Failed to open OneDrive: {e}"))?;
    Ok(())
}

#[tauri::command]
async fn invoke_onedrive_command(
    state: State<'_, AppState>,
    account_id: String,
    command_id: String,
) -> Result<(), String> {
    let target = state
        .accounts
        .lock()
        .ok()
        .and_then(|list| list.iter().find(|a| a.id == account_id).cloned());

    let Some(account) = target else {
        return Err(format!("Account '{account_id}' not found"));
    };
    let Some(sync_root_id) = account.sync_root_id else {
        return Err("Windows SyncRoot ID is unavailable for this account".to_string());
    };
    let clsid = account.status_ui_factory_clsid;

    // Invoke COM/WinRT on a dedicated worker thread to respect COM threading
    tokio::task::spawn_blocking(move || {
        invoke_provider_command(&sync_root_id, clsid.as_deref(), &command_id)
    })
    .await
    .map_err(|e| format!("Command worker failed: {e}"))?
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loaded_settings = settings::load_settings();
    let initial_state = AppState {
        settings: Mutex::new(loaded_settings.clone()),
        snapshot: Mutex::new(empty_snapshot()),
        visibility: Mutex::new(VisibilityController::default()),
        accounts: Mutex::new(Vec::new()),
    };

    tauri::Builder::default()
        .manage(initial_state)
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_overlay_manual(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();

            // 1. Position and immediately reveal floating overlay (unless fullscreen app is active)
            if let Some(window) = app.get_webview_window("main") {
                window_ctrl::position_initial_window(&window, &loaded_settings);
                let lower_half = window_ctrl::is_window_in_lower_half(&window);
                let _ = handle.emit("hud-anchor-changed", lower_half);

                let suppress_fs = loaded_settings.hide_in_fullscreen
                    && onedrive::fullscreen::is_fullscreen_app_active();
                if !suppress_fs {
                    let _ = window.show();
                    let st = handle.state::<AppState>();
                    if let Ok(mut vis) = st.visibility.lock() {
                        vis.window_visible = true;
                    };
                }

                // Listen for user drag moves to run debounced edge-snapping & persistence after drop
                let move_handle = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Moved(_) = event {
                        let st = move_handle.state::<AppState>();
                        if let Ok(mut vis) = st.visibility.lock() {
                            let now = Instant::now();
                            if vis.ignore_moves_until.is_none_or(|until| now >= until) {
                                vis.pending_drag_settle_at = Some(now + Duration::from_millis(280));
                            }
                        };
                    }
                });
            }

            // 1b. Fast lightweight drag-settle watcher (200ms) so edge-snap & save feel instant on drop
            let drag_handle = handle.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(160));
                let st = drag_handle.state::<AppState>();
                settle_finished_drag_if_ready(&drag_handle, &st);
            });

            // 2. Register System Tray icon & menu
            let _ = tray::create_tray(
                &handle,
                &loaded_settings,
                handle_tray_menu,
                toggle_overlay_manual,
            )?;

            // 3. Register global shortcut Ctrl+Alt+O (non-fatal if another app claimed it)
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyO);
            let _ = app.global_shortcut().register(shortcut);

            // 4. Spawn dedicated telemetry polling thread (MTA COM safe)
            let bg_handle = handle.clone();
            std::thread::spawn(move || {
                let mut engine = TelemetryEngine::new();
                let mut last_tray_state = SyncState::Unknown;
                let mut last_tray_tooltip = String::new();

                loop {
                    let snapshot = engine.poll();
                    let accounts_copy = engine.accounts().to_vec();

                    let state = bg_handle.state::<AppState>();
                    if let Ok(mut guard) = state.accounts.lock() {
                        *guard = accounts_copy;
                    }
                    if let Ok(mut guard) = state.snapshot.lock() {
                        *guard = snapshot.clone();
                    }

                    let tooltip = format_tooltip(&snapshot);
                    if snapshot.aggregate_state != last_tray_state || tooltip != last_tray_tooltip {
                        tray::refresh_tray_visuals(
                            &bg_handle,
                            snapshot.aggregate_state,
                            &tooltip,
                            None,
                        );
                        last_tray_state = snapshot.aggregate_state;
                        last_tray_tooltip = tooltip;
                    }

                    let _ = bg_handle.emit("hud-snapshot", &snapshot);
                    evaluate_visibility(&bg_handle, &state);

                    std::thread::sleep(Duration::from_millis(1200));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_settings,
            update_settings,
            set_expanded,
            set_hovered,
            finish_drag,
            dismiss_hud,
            reset_position,
            open_onedrive_folder,
            reveal_file_in_explorer,
            launch_onedrive,
            open_onedrive_flyout,
            invoke_onedrive_command,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running OneDrive Sync HUD");
}
