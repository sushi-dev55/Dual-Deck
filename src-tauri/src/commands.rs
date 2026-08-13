use crate::actions::ActionService;
use crate::controller::{ControllerHandle, ControllerSnapshot, ControllerWorker};
use crate::domain::{
    AppSettings, AppSnapshot, BindingDraft, ExecutionOutcome, InputBinding, Profile, ProfileDraft,
};
use crate::error::{AppResult, CommandError};
use crate::lifecycle::{emit_state_changed, refresh_tray_menu};
use crate::storage::Database;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use uuid::Uuid;

pub struct AppState {
    pub database: Arc<Database>,
    pub actions: ActionService,
    pub controller: Option<ControllerHandle>,
    pub controller_error: Option<String>,
    pub _controller_worker: Option<ControllerWorker>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerStatus {
    pub snapshot: Option<ControllerSnapshot>,
    pub initialization_error: Option<String>,
}

#[tauri::command]
pub fn get_app_snapshot(state: State<'_, AppState>) -> AppResult<AppSnapshot> {
    state.database.snapshot()
}

#[tauri::command]
pub fn get_controller_status(state: State<'_, AppState>) -> ControllerStatus {
    ControllerStatus {
        snapshot: state.controller.as_ref().map(ControllerHandle::snapshot),
        initialization_error: state.controller_error.clone(),
    }
}

#[tauri::command]
pub fn create_profile(
    draft: ProfileDraft,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Profile> {
    let profile = state.database.create_profile(draft)?;
    notify_state_change(&app, "profileCreated");
    Ok(profile)
}

#[tauri::command]
pub fn update_profile(
    id: Uuid,
    draft: ProfileDraft,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Profile> {
    let profile = state.database.update_profile(id, draft)?;
    notify_state_change(&app, "profileUpdated");
    Ok(profile)
}

#[tauri::command]
pub fn duplicate_profile(
    id: Uuid,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Profile> {
    let profile = state.database.duplicate_profile(id)?;
    notify_state_change(&app, "profileCreated");
    Ok(profile)
}

#[tauri::command]
pub fn delete_profile(
    id: Uuid,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    let settings = state.database.delete_profile(id)?;
    notify_state_change(&app, "profileDeleted");
    Ok(settings)
}

#[tauri::command]
pub fn set_active_profile(
    id: Uuid,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    let settings = state.database.set_active_profile(id)?;
    notify_state_change(&app, "activeProfileChanged");
    Ok(settings)
}

#[tauri::command]
pub fn list_bindings(profile_id: Uuid, state: State<'_, AppState>) -> AppResult<Vec<InputBinding>> {
    state.database.list_bindings(profile_id)
}

#[tauri::command]
pub fn upsert_binding(
    id: Uuid,
    draft: BindingDraft,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<InputBinding> {
    let binding = state.database.upsert_binding(id, draft)?;
    let _ = emit_state_changed(&app, "mappingUpdated");
    Ok(binding)
}

#[tauri::command]
pub fn delete_binding(id: Uuid, app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.database.delete_binding(id)?;
    let _ = emit_state_changed(&app, "mappingDeleted");
    Ok(())
}

#[tauri::command]
pub fn update_settings(
    settings: AppSettings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    apply_autostart(&app, settings.launch_at_startup)?;
    let settings = state.database.update_settings(settings)?;
    if let Some(controller) = &state.controller {
        controller.set_paused(settings.mappings_paused);
    }
    notify_state_change(&app, "settingsUpdated");
    Ok(settings)
}

#[tauri::command]
pub fn set_mappings_paused(
    paused: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    let settings = state.database.set_mappings_paused(paused)?;
    if let Some(controller) = &state.controller {
        controller.set_paused(paused);
    }
    notify_state_change(&app, "mappingPauseChanged");
    Ok(settings)
}

#[tauri::command]
pub async fn execute_binding(
    id: Uuid,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ExecutionOutcome> {
    let outcome = state.actions.execute_binding(id).await?;
    if outcome.profile_switched_to.is_some() {
        notify_state_change(&app, "activeProfileChanged");
    }
    Ok(outcome)
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) -> AppResult<()> {
    let manager = app.autolaunch();
    let current = manager
        .is_enabled()
        .map_err(|error| CommandError::new("autostartError", error.to_string()))?;
    if enabled != current {
        if enabled {
            manager
                .enable()
                .map_err(|error| CommandError::new("autostartError", error.to_string()))?;
        } else {
            manager
                .disable()
                .map_err(|error| CommandError::new("autostartError", error.to_string()))?;
        }
    }
    Ok(())
}

fn notify_state_change(app: &AppHandle, reason: &str) {
    let _ = refresh_tray_menu(app);
    let _ = emit_state_changed(app, reason);
}
