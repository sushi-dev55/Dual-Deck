use std::{collections::BTreeSet, time::Duration};

use serde::{Deserialize, Serialize};

pub const SONY_VENDOR_ID: u16 = 0x054c;
pub const DUALSENSE_PRODUCT_ID: u16 = 0x0ce6;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerButton {
    Triangle,
    Circle,
    Cross,
    Square,
    Create,
    PlayStation,
    Options,
    L3,
    R3,
    L1,
    R1,
    L2,
    R2,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Microphone,
    Touchpad,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    L2,
    R2,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    Wired,
    Wireless,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryState {
    Charging,
    Charged,
    Discharging,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub state: BatteryState,
    pub percentage: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerDevice {
    pub instance_id: u32,
    pub name: String,
    pub path: Option<String>,
    pub serial_number: Option<String>,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_version: Option<u16>,
    pub firmware_version: Option<u16>,
    pub connection: ConnectionKind,
    pub battery: BatteryInfo,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AxisPosition {
    pub axis: ControllerAxis,
    pub value: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerSnapshot {
    pub device: Option<ControllerDevice>,
    pub pressed_buttons: BTreeSet<ControllerButton>,
    pub axes: Vec<AxisPosition>,
    pub paused: bool,
    pub ignored_device_count: usize,
    pub dropped_event_count: u64,
    pub updated_at_ns: u64,
}

impl ControllerSnapshot {
    pub(crate) fn disconnected(paused: bool) -> Self {
        Self {
            device: None,
            pressed_buttons: BTreeSet::new(),
            axes: ControllerAxis::ALL
                .into_iter()
                .map(|axis| AxisPosition { axis, value: 0.0 })
                .collect(),
            paused,
            ignored_device_count: 0,
            dropped_event_count: 0,
            updated_at_ns: 0,
        }
    }
}

impl ControllerAxis {
    pub const ALL: [Self; 6] = [
        Self::LeftX,
        Self::LeftY,
        Self::RightX,
        Self::RightY,
        Self::L2,
        Self::R2,
    ];
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerEvent {
    pub sequence: u64,
    pub timestamp_ns: u64,
    #[serde(flatten)]
    pub kind: ControllerEventKind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControllerEventKind {
    Connected {
        device: ControllerDevice,
    },
    Disconnected {
        instance_id: u32,
    },
    DeviceUpdated {
        device: ControllerDevice,
    },
    ButtonChanged {
        button: ControllerButton,
        state: ButtonState,
    },
    AxisChanged {
        axis: ControllerAxis,
        value: f32,
    },
    PausedChanged {
        paused: bool,
    },
    AdditionalDevicesIgnored {
        count: usize,
    },
    EventsDropped {
        total: u64,
        snapshot: ControllerSnapshot,
    },
    BackendError {
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriggerThresholds {
    pub press: f32,
    pub release: f32,
}

impl Default for TriggerThresholds {
    fn default() -> Self {
        Self {
            press: 0.55,
            release: 0.45,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControllerConfig {
    pub event_capacity: usize,
    pub allowed_product_ids: BTreeSet<u16>,
    pub trigger_thresholds: TriggerThresholds,
    pub axis_epsilon: f32,
    pub axis_interval: Duration,
    pub device_info_interval: Duration,
    pub start_paused: bool,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            event_capacity: 256,
            allowed_product_ids: BTreeSet::from([DUALSENSE_PRODUCT_ID]),
            trigger_thresholds: TriggerThresholds::default(),
            axis_epsilon: 0.015,
            axis_interval: Duration::from_millis(16),
            device_info_interval: Duration::from_secs(2),
            start_paused: false,
        }
    }
}

impl ControllerConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.event_capacity == 0 {
            return Err("event capacity must be greater than zero".into());
        }
        if self.allowed_product_ids.is_empty() {
            return Err("at least one controller product ID is required".into());
        }
        if !(0.0..=1.0).contains(&self.trigger_thresholds.release)
            || !(0.0..=1.0).contains(&self.trigger_thresholds.press)
            || self.trigger_thresholds.release >= self.trigger_thresholds.press
        {
            return Err("trigger thresholds must satisfy 0 <= release < press <= 1".into());
        }
        if !self.axis_epsilon.is_finite() || !(0.0..=1.0).contains(&self.axis_epsilon) {
            return Err("axis epsilon must be between zero and one".into());
        }
        if self.axis_interval.is_zero() {
            return Err("axis interval must be greater than zero".into());
        }
        if self.device_info_interval.is_zero() {
            return Err("device info interval must be greater than zero".into());
        }
        Ok(())
    }
}
