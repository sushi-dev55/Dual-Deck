use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub automatic_app: Option<PathBuf>,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDraft {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub automatic_app: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum ControllerInput {
    Button(ControllerButton),
    StickDirection(StickDirection),
    TriggerZone(TriggerZone),
    TouchpadZone(TouchpadZone),
    Combination(Vec<ControllerButton>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ControllerButton {
    Cross,
    Circle,
    Square,
    Triangle,
    DpadUp,
    DpadRight,
    DpadDown,
    DpadLeft,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Create,
    Options,
    Playstation,
    Mute,
    LeftStick,
    RightStick,
    Touchpad,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum StickDirection {
    LeftUp,
    LeftRight,
    LeftDown,
    LeftLeft,
    RightUp,
    RightRight,
    RightDown,
    RightLeft,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum TriggerZone {
    LeftSoft,
    LeftFull,
    RightSoft,
    RightFull,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum TouchpadZone {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Trigger {
    #[default]
    Press,
    Release,
    LongPress {
        duration_ms: u64,
    },
    DoublePress {
        interval_ms: u64,
    },
    HoldRepeat {
        initial_delay_ms: u64,
        interval_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ActionDefinition {
    Incomplete {
        action_id: String,
        #[serde(default)]
        configuration: serde_json::Value,
    },
    OpenApplication {
        path: PathBuf,
        #[serde(default)]
        arguments: Vec<String>,
        working_directory: Option<PathBuf>,
    },
    OpenPath {
        path: PathBuf,
    },
    OpenUrl {
        url: String,
    },
    Hotkey {
        hotkey: Hotkey,
    },
    TypeText {
        text: String,
    },
    Media {
        command: MediaCommand,
    },
    Volume {
        command: VolumeCommand,
    },
    PlaySound {
        path: PathBuf,
    },
    Webhook {
        request: WebhookRequest,
    },
    CloseApplication {
        executable_name: String,
    },
    SwitchProfile {
        profile_id: Uuid,
    },
    Delay {
        duration_ms: u64,
    },
    MultiAction {
        steps: Vec<ActionStep>,
        #[serde(default = "default_stop_on_error")]
        stop_on_error: bool,
    },
}

fn default_stop_on_error() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionStep {
    pub action: ActionDefinition,
    #[serde(default)]
    pub delay_after_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    #[serde(default)]
    pub modifiers: Vec<KeyModifier>,
    pub key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KeyModifier {
    Control,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaCommand {
    PlayPause,
    NextTrack,
    PreviousTrack,
    Stop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VolumeCommand {
    Up,
    Down,
    Mute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebhookRequest {
    pub url: String,
    #[serde(default)]
    pub method: WebhookMethod,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub body: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    10_000
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum WebhookMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputBinding {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub input: ControllerInput,
    pub trigger: Trigger,
    pub action: ActionDefinition,
    pub label: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BindingDraft {
    pub profile_id: Uuid,
    pub input: ControllerInput,
    #[serde(default)]
    pub trigger: Trigger,
    pub action: ActionDefinition,
    pub label: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub active_profile_id: Uuid,
    pub launch_at_startup: bool,
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub mappings_paused: bool,
    pub check_for_updates: bool,
    pub automatic_profile_switching: bool,
    #[serde(default = "default_true")]
    pub action_toasts: bool,
    #[serde(default)]
    pub controller_feedback: bool,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub update_channel: UpdateChannel,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Preview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub profiles: Vec<Profile>,
    pub active_profile: Profile,
    pub bindings: Vec<InputBinding>,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOutcome {
    pub completed_steps: usize,
    pub profile_switched_to: Option<Uuid>,
}
