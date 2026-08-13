use crate::actions::{ActionService, ReservedAction};
use crate::controller::{
    ButtonState, ControllerButton as RawButton, ControllerEventKind, ControllerEventReceiver,
    ControllerSnapshot,
};
use crate::domain::{ControllerButton, ControllerInput, InputBinding, Trigger};
use crate::error::CommandError;
use crate::lifecycle::{emit_state_changed, refresh_tray_menu};
use crate::storage::Database;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

struct TriggerTracker {
    profile_id: Option<Uuid>,
    pressed: HashSet<ControllerButton>,
    active_bindings: HashSet<Uuid>,
    generations: HashMap<Uuid, u64>,
    last_press_ns: HashMap<Uuid, u64>,
}

impl TriggerTracker {
    fn new() -> Self {
        Self {
            profile_id: None,
            pressed: HashSet::new(),
            active_bindings: HashSet::new(),
            generations: HashMap::new(),
            last_press_ns: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.pressed.clear();
        self.active_bindings.clear();
        self.last_press_ns.clear();
        for generation in self.generations.values_mut() {
            *generation = generation.wrapping_add(1);
        }
    }

    fn generation(&mut self, id: Uuid) -> u64 {
        let generation = self.generations.entry(id).or_default();
        *generation = generation.wrapping_add(1);
        *generation
    }

    fn generation_is_current(&self, id: Uuid, generation: u64) -> bool {
        self.generations.get(&id).copied() == Some(generation) && self.active_bindings.contains(&id)
    }
}

enum ScheduledAction {
    Immediate {
        binding_id: Uuid,
    },
    LongPress {
        binding_id: Uuid,
        generation: u64,
        duration_ms: u64,
    },
    HoldRepeat {
        binding_id: Uuid,
        generation: u64,
        initial_delay_ms: u64,
        interval_ms: u64,
    },
}

impl ScheduledAction {
    fn binding_id(&self) -> Uuid {
        match self {
            Self::Immediate { binding_id }
            | Self::LongPress { binding_id, .. }
            | Self::HoldRepeat { binding_id, .. } => *binding_id,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExecutionStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionEvent {
    binding_id: Uuid,
    code: Option<String>,
    message: Option<String>,
}

pub fn start(
    mut events: ControllerEventReceiver,
    database: Arc<Database>,
    actions: ActionService,
    app: AppHandle,
) {
    let tracker = Arc::new(Mutex::new(TriggerTracker::new()));
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            match &event.kind {
                ControllerEventKind::ButtonChanged { button, state } => {
                    dispatch_button(
                        *button,
                        *state,
                        event.timestamp_ns,
                        &database,
                        &actions,
                        &tracker,
                        &app,
                    );
                }
                ControllerEventKind::Disconnected { .. }
                | ControllerEventKind::PausedChanged { .. } => tracker.lock().reset(),
                ControllerEventKind::EventsDropped { snapshot, .. } => dispatch_snapshot(
                    snapshot,
                    event.timestamp_ns,
                    &database,
                    &actions,
                    &tracker,
                    &app,
                ),
                _ => {}
            }
            let _ = app.emit("controller-event", &event);
        }
    });
}

fn dispatch_button(
    raw_button: RawButton,
    state: ButtonState,
    timestamp_ns: u64,
    database: &Arc<Database>,
    actions: &ActionService,
    tracker: &Arc<Mutex<TriggerTracker>>,
    app: &AppHandle,
) {
    let settings = match database.settings() {
        Ok(settings) if !settings.mappings_paused => settings,
        _ => return,
    };
    let bindings = match database.list_bindings(settings.active_profile_id) {
        Ok(bindings) => bindings,
        Err(_) => return,
    };
    let button = map_button(raw_button);
    let scheduled = {
        let mut tracker = tracker.lock();
        if tracker.profile_id != Some(settings.active_profile_id) {
            tracker.reset();
            tracker.profile_id = Some(settings.active_profile_id);
        }
        match state {
            ButtonState::Pressed => {
                tracker.pressed.insert(button);
            }
            ButtonState::Released => {
                tracker.pressed.remove(&button);
            }
        }
        collect_actions(&bindings, timestamp_ns, &mut tracker)
    };
    for action in scheduled {
        schedule(action, actions.clone(), tracker.clone(), app.clone());
    }
}

fn dispatch_snapshot(
    snapshot: &ControllerSnapshot,
    timestamp_ns: u64,
    database: &Arc<Database>,
    actions: &ActionService,
    tracker: &Arc<Mutex<TriggerTracker>>,
    app: &AppHandle,
) {
    if snapshot.paused || snapshot.device.is_none() {
        tracker.lock().reset();
        return;
    }
    let settings = match database.settings() {
        Ok(settings) if !settings.mappings_paused => settings,
        _ => {
            tracker.lock().reset();
            return;
        }
    };
    let bindings = match database.list_bindings(settings.active_profile_id) {
        Ok(bindings) => bindings,
        Err(_) => {
            tracker.lock().reset();
            return;
        }
    };
    let pressed = snapshot
        .pressed_buttons
        .iter()
        .copied()
        .map(map_button)
        .collect();
    let scheduled = {
        let mut tracker = tracker.lock();
        if tracker.profile_id != Some(settings.active_profile_id) {
            tracker.reset();
            tracker.profile_id = Some(settings.active_profile_id);
        }
        tracker.pressed = pressed;
        collect_actions(&bindings, timestamp_ns, &mut tracker)
    };
    for action in scheduled {
        schedule(action, actions.clone(), tracker.clone(), app.clone());
    }
}

fn collect_actions(
    bindings: &[InputBinding],
    timestamp_ns: u64,
    tracker: &mut TriggerTracker,
) -> Vec<ScheduledAction> {
    let mut scheduled = Vec::new();
    for binding in bindings.iter().filter(|binding| binding.enabled) {
        let Some(active) = input_active(&binding.input, &tracker.pressed) else {
            continue;
        };
        let was_active = tracker.active_bindings.contains(&binding.id);
        if active {
            tracker.active_bindings.insert(binding.id);
        } else {
            tracker.active_bindings.remove(&binding.id);
        }
        match (&binding.trigger, active, was_active) {
            (Trigger::Press, true, false) => scheduled.push(immediate(binding)),
            (Trigger::Release, false, true) => scheduled.push(immediate(binding)),
            (Trigger::LongPress { duration_ms }, true, false) => {
                let generation = tracker.generation(binding.id);
                scheduled.push(ScheduledAction::LongPress {
                    binding_id: binding.id,
                    generation,
                    duration_ms: *duration_ms,
                });
            }
            (Trigger::LongPress { .. }, false, true)
            | (Trigger::HoldRepeat { .. }, false, true) => {
                tracker.generation(binding.id);
            }
            (
                Trigger::HoldRepeat {
                    initial_delay_ms,
                    interval_ms,
                },
                true,
                false,
            ) => {
                let generation = tracker.generation(binding.id);
                scheduled.push(ScheduledAction::HoldRepeat {
                    binding_id: binding.id,
                    generation,
                    initial_delay_ms: *initial_delay_ms,
                    interval_ms: *interval_ms,
                });
            }
            (Trigger::DoublePress { interval_ms }, true, false) => {
                let previous = tracker.last_press_ns.remove(&binding.id);
                let within_window = previous.is_some_and(|previous| {
                    timestamp_ns.saturating_sub(previous) <= interval_ms.saturating_mul(1_000_000)
                });
                if within_window {
                    scheduled.push(immediate(binding));
                } else {
                    tracker.last_press_ns.insert(binding.id, timestamp_ns);
                }
            }
            _ => {}
        }
    }
    scheduled
}

fn input_active(input: &ControllerInput, pressed: &HashSet<ControllerButton>) -> Option<bool> {
    match input {
        ControllerInput::Button(button) => Some(pressed.contains(button)),
        ControllerInput::Combination(buttons) => {
            Some(!buttons.is_empty() && buttons.iter().all(|button| pressed.contains(button)))
        }
        _ => None,
    }
}

fn immediate(binding: &InputBinding) -> ScheduledAction {
    ScheduledAction::Immediate {
        binding_id: binding.id,
    }
}

fn schedule(
    scheduled: ScheduledAction,
    actions: ActionService,
    tracker: Arc<Mutex<TriggerTracker>>,
    app: AppHandle,
) {
    let binding_id = scheduled.binding_id();
    let reservation = match actions.try_reserve() {
        Ok(reservation) => reservation,
        Err(error) => {
            emit_action_failure(binding_id, error, &app);
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        match scheduled {
            ScheduledAction::Immediate { binding_id } => {
                execute_and_report(binding_id, &reservation, &app, || true).await;
            }
            ScheduledAction::LongPress {
                binding_id,
                generation,
                duration_ms,
            } => {
                tokio::time::sleep(Duration::from_millis(duration_ms)).await;
                execute_and_report(binding_id, &reservation, &app, || {
                    tracker.lock().generation_is_current(binding_id, generation)
                })
                .await;
            }
            ScheduledAction::HoldRepeat {
                binding_id,
                generation,
                initial_delay_ms,
                interval_ms,
            } => {
                tokio::time::sleep(Duration::from_millis(initial_delay_ms)).await;
                loop {
                    let status = execute_and_report(binding_id, &reservation, &app, || {
                        tracker.lock().generation_is_current(binding_id, generation)
                    })
                    .await;
                    if status != ExecutionStatus::Completed {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                }
            }
        }
    });
}

async fn execute_and_report<F>(
    binding_id: Uuid,
    reservation: &ReservedAction,
    app: &AppHandle,
    still_current: F,
) -> ExecutionStatus
where
    F: Fn() -> bool,
{
    match reservation
        .execute_binding_if(binding_id, still_current)
        .await
    {
        Ok(Some(outcome)) => {
            let _ = app.emit(
                "action-completed",
                ActionEvent {
                    binding_id,
                    code: None,
                    message: None,
                },
            );
            if outcome.profile_switched_to.is_some() {
                let _ = refresh_tray_menu(app);
                let _ = emit_state_changed(app, "activeProfileChanged");
            }
            ExecutionStatus::Completed
        }
        Ok(None) => ExecutionStatus::Cancelled,
        Err(CommandError { code, .. }) if execution_was_cancelled(&code) => {
            ExecutionStatus::Cancelled
        }
        Err(error) => {
            emit_action_failure(binding_id, error, app);
            ExecutionStatus::Failed
        }
    }
}

fn emit_action_failure(binding_id: Uuid, error: CommandError, app: &AppHandle) {
    let _ = app.emit(
        "action-failed",
        ActionEvent {
            binding_id,
            code: Some(error.code),
            message: Some(error.message),
        },
    );
}

fn execution_was_cancelled(code: &str) -> bool {
    matches!(
        code,
        "bindingNotFound" | "mappingDisabled" | "mappingInactive" | "mappingsPaused"
    )
}

pub fn map_button(button: RawButton) -> ControllerButton {
    match button {
        RawButton::Triangle => ControllerButton::Triangle,
        RawButton::Circle => ControllerButton::Circle,
        RawButton::Cross => ControllerButton::Cross,
        RawButton::Square => ControllerButton::Square,
        RawButton::Create => ControllerButton::Create,
        RawButton::PlayStation => ControllerButton::Playstation,
        RawButton::Options => ControllerButton::Options,
        RawButton::L3 => ControllerButton::LeftStick,
        RawButton::R3 => ControllerButton::RightStick,
        RawButton::L1 => ControllerButton::LeftBumper,
        RawButton::R1 => ControllerButton::RightBumper,
        RawButton::L2 => ControllerButton::LeftTrigger,
        RawButton::R2 => ControllerButton::RightTrigger,
        RawButton::DPadUp => ControllerButton::DpadUp,
        RawButton::DPadRight => ControllerButton::DpadRight,
        RawButton::DPadDown => ControllerButton::DpadDown,
        RawButton::DPadLeft => ControllerButton::DpadLeft,
        RawButton::Microphone => ControllerButton::Mute,
        RawButton::Touchpad => ControllerButton::Touchpad,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ActionDefinition, ControllerInput, InputBinding, Trigger};
    use chrono::Utc;

    fn binding(trigger: Trigger) -> InputBinding {
        InputBinding {
            id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
            input: ControllerInput::Button(ControllerButton::Triangle),
            trigger,
            action: ActionDefinition::OpenUrl {
                url: "https://example.com".into(),
            },
            label: "Test".into(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn press_and_release_fire_on_their_respective_edges() {
        let mut tracker = TriggerTracker::new();
        let pressed = binding(Trigger::Press);
        let released = binding(Trigger::Release);
        tracker.pressed.insert(ControllerButton::Triangle);
        assert_eq!(
            collect_actions(&[pressed.clone(), released.clone()], 1, &mut tracker).len(),
            1
        );
        tracker.pressed.remove(&ControllerButton::Triangle);
        assert_eq!(
            collect_actions(&[pressed, released], 2, &mut tracker).len(),
            1
        );
    }

    #[test]
    fn double_press_requires_two_edges_within_the_window() {
        let mut tracker = TriggerTracker::new();
        let binding = binding(Trigger::DoublePress { interval_ms: 300 });
        tracker.pressed.insert(ControllerButton::Triangle);
        assert!(
            collect_actions(std::slice::from_ref(&binding), 1_000_000, &mut tracker).is_empty()
        );
        tracker.pressed.remove(&ControllerButton::Triangle);
        collect_actions(std::slice::from_ref(&binding), 2_000_000, &mut tracker);
        tracker.pressed.insert(ControllerButton::Triangle);
        assert_eq!(
            collect_actions(&[binding], 200_000_000, &mut tracker).len(),
            1
        );
    }

    #[test]
    fn maps_every_dualsense_button() {
        assert_eq!(map_button(RawButton::L1), ControllerButton::LeftBumper);
        assert_eq!(
            map_button(RawButton::PlayStation),
            ControllerButton::Playstation
        );
        assert_eq!(map_button(RawButton::Microphone), ControllerButton::Mute);
    }
}
