mod actions;
mod commands;
pub mod controller;
mod dispatch;
mod domain;
mod error;
mod lifecycle;
mod platform;
mod storage;

use actions::ActionService;
use commands::AppState;
use controller::{ControllerConfig, ControllerRuntime};
use platform::SystemPlatform;
use std::sync::Arc;
use storage::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = lifecycle::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            let database_name = if cfg!(debug_assertions) {
                "dual-deck-dev.sqlite3"
            } else {
                "dual-deck.sqlite3"
            };
            let database = Arc::new(Database::open(data_directory.join(database_name))?);
            let actions = ActionService::new(database.clone(), Arc::new(SystemPlatform::new()))?;
            let settings = database.settings()?;
            let controller_config = ControllerConfig {
                start_paused: settings.mappings_paused,
                ..ControllerConfig::default()
            };
            let (controller, controller_events, controller_worker, controller_error) =
                match ControllerRuntime::start(controller_config) {
                    Ok(runtime) => {
                        let (handle, events, worker) = runtime.into_parts();
                        (Some(handle), Some(events), Some(worker), None)
                    }
                    Err(error) => (None, None, None, Some(error.to_string())),
                };
            app.manage(AppState {
                database: database.clone(),
                actions: actions.clone(),
                controller,
                controller_error,
                _controller_worker: controller_worker,
            });
            if let Some(events) = controller_events {
                dispatch::start(events, database, actions, app.handle().clone());
            }
            if !cfg!(debug_assertions) {
                commands::apply_autostart(app.handle(), settings.launch_at_startup)?;
            }
            lifecycle::setup(app)?;
            Ok(())
        })
        .on_window_event(lifecycle::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_snapshot,
            commands::get_controller_status,
            commands::create_profile,
            commands::update_profile,
            commands::duplicate_profile,
            commands::delete_profile,
            commands::set_active_profile,
            commands::list_bindings,
            commands::upsert_binding,
            commands::delete_binding,
            commands::update_settings,
            commands::set_mappings_paused,
            commands::execute_binding
        ])
        .run(tauri::generate_context!())
        .expect("Dual Deck failed to start")
}
