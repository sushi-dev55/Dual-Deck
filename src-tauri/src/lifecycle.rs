use crate::commands::AppState;
use crate::error::{AppResult, CommandError};
use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, Window, WindowEvent};

const TRAY_ID: &str = "dual-deck-tray";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StateChangedEvent<'a> {
    reason: &'a str,
}

pub fn setup(app: &mut App) -> AppResult<()> {
    let menu = build_tray_menu(app.handle())?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Dual Deck")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .build(app)
        .map_err(|error| CommandError::new("trayError", error.to_string()))?;
    let state = app.state::<AppState>();
    let settings = state.database.settings()?;
    let launched_at_startup =
        std::env::args().any(|argument| matches!(argument.as_str(), "--autostart" | "--minimized"));
    if !(launched_at_startup && settings.start_minimized) {
        show_main_window(app.handle())?;
    }
    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            let close_to_tray = window
                .try_state::<AppState>()
                .and_then(|state| state.database.settings().ok())
                .is_some_and(|settings| settings.close_to_tray);
            if close_to_tray {
                api.prevent_close();
                let _ = window.hide();
            }
        }
        WindowEvent::Resized(_) => {
            let minimize_to_tray = window
                .try_state::<AppState>()
                .and_then(|state| state.database.settings().ok())
                .is_some_and(|settings| settings.minimize_to_tray);
            if minimize_to_tray && window.is_minimized().unwrap_or(false) {
                let _ = window.hide();
            }
        }
        _ => {}
    }
}

pub fn refresh_tray_menu(app: &AppHandle) -> AppResult<()> {
    let menu = build_tray_menu(app)?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))
            .map_err(|error| CommandError::new("trayError", error.to_string()))?;
    }
    Ok(())
}

pub fn show_main_window(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
            .map_err(|error| CommandError::new("windowError", error.to_string()))?;
    }
    Ok(())
}

pub fn emit_state_changed(app: &AppHandle, reason: &str) -> AppResult<()> {
    app.emit("state-changed", StateChangedEvent { reason })
        .map_err(|error| CommandError::new("eventError", error.to_string()))
}

fn build_tray_menu(app: &AppHandle) -> AppResult<tauri::menu::Menu<tauri::Wry>> {
    let state = app.state::<AppState>();
    let snapshot = state.database.snapshot()?;
    let status = MenuItem::with_id(
        app,
        "active-profile-status",
        format!("Active: {}", snapshot.active_profile.name),
        false,
        None::<&str>,
    )
    .map_err(|error| CommandError::new("trayError", error.to_string()))?;
    let mut profiles = SubmenuBuilder::with_id(app, "profiles", "Profiles");
    for profile in &snapshot.profiles {
        let label = if profile.id == snapshot.settings.active_profile_id {
            format!("✓ {}", profile.name)
        } else {
            profile.name.clone()
        };
        profiles = profiles.text(format!("profile:{}", profile.id), label);
    }
    let profiles = profiles
        .build()
        .map_err(|error| CommandError::new("trayError", error.to_string()))?;
    let pause_label = if snapshot.settings.mappings_paused {
        "Resume mappings"
    } else {
        "Pause mappings"
    };
    MenuBuilder::new(app)
        .item(&status)
        .item(&profiles)
        .separator()
        .text("toggle-pause", pause_label)
        .text("show", "Open Dual Deck")
        .separator()
        .text("quit", "Quit")
        .build()
        .map_err(|error| CommandError::new("trayError", error.to_string()))
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "show" => {
            let _ = show_main_window(app);
        }
        "toggle-pause" => {
            let state = app.state::<AppState>();
            if let Ok(settings) = state.database.toggle_mappings_paused() {
                if let Some(controller) = &state.controller {
                    controller.set_paused(settings.mappings_paused);
                }
                let _ = refresh_tray_menu(app);
                let _ = emit_state_changed(app, "mappingPauseChanged");
            }
        }
        "quit" => app.exit(0),
        value if value.starts_with("profile:") => {
            if let Ok(profile_id) = value[8..].parse() {
                let state = app.state::<AppState>();
                if state.database.set_active_profile(profile_id).is_ok() {
                    let _ = refresh_tray_menu(app);
                    let _ = emit_state_changed(app, "activeProfileChanged");
                }
            }
        }
        _ => {}
    }
}
