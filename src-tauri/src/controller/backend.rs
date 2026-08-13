use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use sdl3::{
    EventPump, GamepadSubsystem, Sdl,
    event::Event,
    gamepad::{Axis, Button, Gamepad, GamepadType},
    joystick::{ConnectionState, JoystickId, PowerLevel},
};
use tokio::sync::mpsc;

use super::{
    AxisPosition, BatteryInfo, BatteryState, ConnectionKind, ControllerAxis, ControllerButton,
    ControllerConfig, ControllerDevice, ControllerEvent, ControllerEventKind, ControllerSnapshot,
    SONY_VENDOR_ID,
    state::{InputNormalizer, NormalizedInput, NormalizedInputKind},
};

pub(crate) struct SharedState {
    pub paused: AtomicBool,
    pub shutdown: AtomicBool,
    pub snapshot: RwLock<ControllerSnapshot>,
}

impl SharedState {
    pub fn new(paused: bool) -> Self {
        Self {
            paused: AtomicBool::new(paused),
            shutdown: AtomicBool::new(false),
            snapshot: RwLock::new(ControllerSnapshot::disconnected(paused)),
        }
    }
}

struct EventSink {
    sender: mpsc::Sender<ControllerEvent>,
    shared: Arc<SharedState>,
    sequence: u64,
    dropped: u64,
    reconciliation_pending: bool,
}

impl EventSink {
    fn new(sender: mpsc::Sender<ControllerEvent>, shared: Arc<SharedState>) -> Self {
        Self {
            sender,
            shared,
            sequence: 0,
            dropped: 0,
            reconciliation_pending: false,
        }
    }

    fn emit(&mut self, kind: ControllerEventKind, timestamp_ns: u64) {
        if matches!(kind, ControllerEventKind::AxisChanged { .. }) {
            self.flush_reconciliation(timestamp_ns);
            self.emit_lossy(kind, timestamp_ns);
        } else {
            self.emit_reliable(kind, timestamp_ns);
            self.flush_reconciliation(timestamp_ns);
        }
    }

    fn emit_lossy(&mut self, kind: ControllerEventKind, timestamp_ns: u64) {
        let event = ControllerEvent {
            sequence: self.next_sequence(),
            timestamp_ns,
            kind,
        };
        match self.sender.try_send(event) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.shared.shutdown.store(true, Ordering::Release);
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.dropped = self.dropped.saturating_add(1);
                self.reconciliation_pending = true;
                self.shared.snapshot.write().dropped_event_count = self.dropped;
            }
        }
    }

    fn emit_reliable(&mut self, kind: ControllerEventKind, timestamp_ns: u64) {
        let event = ControllerEvent {
            sequence: self.next_sequence(),
            timestamp_ns,
            kind,
        };
        if self.sender.blocking_send(event).is_err() {
            self.shared.shutdown.store(true, Ordering::Release);
        }
    }

    fn flush_reconciliation(&mut self, timestamp_ns: u64) {
        if !self.reconciliation_pending {
            return;
        }
        let snapshot = self.shared.snapshot.read().clone();
        let notice = ControllerEvent {
            sequence: self.next_sequence(),
            timestamp_ns,
            kind: ControllerEventKind::EventsDropped {
                total: self.dropped,
                snapshot,
            },
        };
        match self.sender.try_send(notice) {
            Ok(()) => self.reconciliation_pending = false,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.shared.shutdown.store(true, Ordering::Release);
            }
            Err(mpsc::error::TrySendError::Full(_)) => {}
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.sequence
    }
}

#[derive(Clone)]
struct Candidate {
    id: JoystickId,
    path: String,
}

struct ActiveController {
    instance_id: u32,
    gamepad: Gamepad,
    device: ControllerDevice,
}

pub(crate) struct SdlWorker {
    active: Option<ActiveController>,
    event_pump: EventPump,
    gamepads: GamepadSubsystem,
    _sdl: Sdl,
    config: ControllerConfig,
    normalizer: InputNormalizer,
    events: EventSink,
    shared: Arc<SharedState>,
    ignored_device_count: usize,
    next_device_info_poll: Instant,
}

impl SdlWorker {
    pub fn initialize(
        config: ControllerConfig,
        sender: mpsc::Sender<ControllerEvent>,
        shared: Arc<SharedState>,
    ) -> Result<Self, String> {
        set_required_hints()?;
        let sdl = sdl3::init().map_err(|error| error.to_string())?;
        let gamepads = sdl.gamepad().map_err(|error| error.to_string())?;
        let event_pump = sdl.event_pump().map_err(|error| error.to_string())?;
        let axis_interval_ns = config.axis_interval.as_nanos().min(u128::from(u64::MAX)) as u64;
        let normalizer = InputNormalizer::new(
            config.trigger_thresholds,
            config.axis_epsilon,
            axis_interval_ns,
            config.start_paused,
        );
        let events = EventSink::new(sender, Arc::clone(&shared));

        Ok(Self {
            active: None,
            event_pump,
            gamepads,
            _sdl: sdl,
            config,
            normalizer,
            events,
            shared,
            ignored_device_count: 0,
            next_device_info_poll: Instant::now(),
        })
    }

    pub fn run(mut self) {
        let now_ns = sdl_now_ns();
        if let Err(message) = self.refresh_devices(now_ns) {
            self.events
                .emit(ControllerEventKind::BackendError { message }, now_ns);
        }

        while !self.shared.shutdown.load(Ordering::Acquire) {
            self.apply_pause_request();

            if let Some(event) = self.event_pump.wait_event_timeout(Duration::from_millis(8)) {
                self.process_event(event);
            }
            while let Some(event) = self.event_pump.poll_event() {
                self.process_event(event);
            }

            let now_ns = sdl_now_ns();
            let pending = self.normalizer.flush_axes(now_ns);
            self.publish_inputs(pending);
            self.events.flush_reconciliation(now_ns);

            if Instant::now() >= self.next_device_info_poll {
                self.poll_device_info(now_ns);
                self.next_device_info_poll = Instant::now() + self.config.device_info_interval;
            }
        }

        let timestamp_ns = sdl_now_ns();
        self.disconnect_active(timestamp_ns);
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::ControllerButtonDown {
                timestamp,
                which,
                button,
            } if self.is_active(which) => {
                if let Some(button) = map_button(button) {
                    let input = self.normalizer.button(button, true, timestamp);
                    self.publish_inputs(input.into_iter().collect());
                }
            }
            Event::ControllerButtonUp {
                timestamp,
                which,
                button,
            } if self.is_active(which) => {
                if let Some(button) = map_button(button) {
                    let input = self.normalizer.button(button, false, timestamp);
                    self.publish_inputs(input.into_iter().collect());
                }
            }
            Event::ControllerAxisMotion {
                timestamp,
                which,
                axis,
                value,
            } if self.is_active(which) => {
                let inputs = self.normalizer.axis(map_axis(axis), value, timestamp);
                self.publish_inputs(inputs);
            }
            Event::ControllerDeviceAdded { timestamp, .. }
            | Event::ControllerDeviceRemoved { timestamp, .. } => {
                if let Err(message) = self.refresh_devices(timestamp) {
                    self.events
                        .emit(ControllerEventKind::BackendError { message }, timestamp);
                }
            }
            _ => {}
        }
    }

    fn refresh_devices(&mut self, timestamp_ns: u64) -> Result<(), String> {
        let candidates = self.candidates()?;
        let active_present = self.active.as_ref().is_some_and(|active| {
            active.gamepad.connected()
                && candidates
                    .iter()
                    .any(|candidate| candidate.id.0 == active.instance_id)
        });

        if !active_present {
            self.disconnect_active(timestamp_ns);
        }

        let mut open_errors = Vec::new();
        if self.active.is_none() {
            for candidate in &candidates {
                match self.gamepads.open(candidate.id) {
                    Ok(gamepad) => {
                        let device = read_device(&gamepad, candidate.path.clone());
                        let instance_id = device.instance_id;
                        self.active = Some(ActiveController {
                            instance_id,
                            gamepad,
                            device: device.clone(),
                        });
                        {
                            let mut snapshot = self.shared.snapshot.write();
                            snapshot.device = Some(device.clone());
                            snapshot.updated_at_ns = timestamp_ns;
                        }
                        self.events
                            .emit(ControllerEventKind::Connected { device }, timestamp_ns);
                        break;
                    }
                    Err(error) => open_errors.push(error.to_string()),
                }
            }
        }

        let ignored = candidates
            .len()
            .saturating_sub(usize::from(self.active.is_some()));
        self.set_ignored_device_count(ignored, timestamp_ns);

        if self.active.is_none() && !open_errors.is_empty() {
            Err(format!(
                "failed to open compatible DualSense controller: {}",
                open_errors.join("; ")
            ))
        } else {
            Ok(())
        }
    }

    fn candidates(&self) -> Result<Vec<Candidate>, String> {
        let mut candidates: Vec<_> = self
            .gamepads
            .gamepads()
            .map_err(|error| format!("failed to enumerate gamepads: {error}"))?
            .into_iter()
            .filter(|id| self.gamepads.vendor_for_id(*id) == Some(SONY_VENDOR_ID))
            .filter(|id| {
                self.gamepads
                    .product_for_id(*id)
                    .is_some_and(|product| self.config.allowed_product_ids.contains(&product))
            })
            .filter(|id| self.gamepads.real_type_for_id(*id) == GamepadType::PS5)
            .map(|id| Candidate {
                id,
                path: self.gamepads.path_for_id(id).unwrap_or_default(),
            })
            .collect();
        candidates.sort_by_key(|candidate| (candidate.path.to_ascii_lowercase(), candidate.id.0));
        Ok(candidates)
    }

    fn disconnect_active(&mut self, timestamp_ns: u64) {
        let Some(active) = self.active.take() else {
            return;
        };
        let inputs = self.normalizer.disconnect(timestamp_ns);
        self.publish_inputs(inputs);
        {
            let mut snapshot = self.shared.snapshot.write();
            snapshot.device = None;
            snapshot.updated_at_ns = timestamp_ns;
        }
        self.events.emit(
            ControllerEventKind::Disconnected {
                instance_id: active.instance_id,
            },
            timestamp_ns,
        );
    }

    fn poll_device_info(&mut self, timestamp_ns: u64) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        if !active.gamepad.connected() {
            if let Err(message) = self.refresh_devices(timestamp_ns) {
                self.events
                    .emit(ControllerEventKind::BackendError { message }, timestamp_ns);
            }
            return;
        }

        let updated = read_device(
            &active.gamepad,
            active.device.path.clone().unwrap_or_default(),
        );
        if updated != active.device {
            active.device = updated.clone();
            {
                let mut snapshot = self.shared.snapshot.write();
                snapshot.device = Some(updated.clone());
                snapshot.updated_at_ns = timestamp_ns;
            }
            self.events.emit(
                ControllerEventKind::DeviceUpdated { device: updated },
                timestamp_ns,
            );
        }
    }

    fn apply_pause_request(&mut self) {
        let paused = self.shared.paused.load(Ordering::Acquire);
        let timestamp_ns = sdl_now_ns();
        let inputs = self.normalizer.set_paused(paused, timestamp_ns);
        if inputs.is_empty() && self.shared.snapshot.read().paused == paused {
            return;
        }

        self.publish_inputs(inputs);
        {
            let mut snapshot = self.shared.snapshot.write();
            snapshot.paused = paused;
            snapshot.updated_at_ns = timestamp_ns;
        }
        self.events
            .emit(ControllerEventKind::PausedChanged { paused }, timestamp_ns);
    }

    fn publish_inputs(&mut self, inputs: Vec<NormalizedInput>) {
        if inputs.is_empty() {
            return;
        }

        let timestamp_ns = inputs
            .last()
            .map(|input| input.timestamp_ns)
            .unwrap_or_default();
        self.sync_input_snapshot(timestamp_ns);
        for input in inputs {
            let kind = match input.kind {
                NormalizedInputKind::Button { button, state } => {
                    ControllerEventKind::ButtonChanged { button, state }
                }
                NormalizedInputKind::Axis { axis, value } => {
                    ControllerEventKind::AxisChanged { axis, value }
                }
            };
            self.events.emit(kind, input.timestamp_ns);
        }
    }

    fn sync_input_snapshot(&self, timestamp_ns: u64) {
        let mut snapshot = self.shared.snapshot.write();
        snapshot.pressed_buttons = self.normalizer.pressed_buttons();
        snapshot.axes = self
            .normalizer
            .axes()
            .into_iter()
            .map(|(axis, value)| AxisPosition { axis, value })
            .collect();
        snapshot.updated_at_ns = timestamp_ns;
    }

    fn set_ignored_device_count(&mut self, count: usize, timestamp_ns: u64) {
        if self.ignored_device_count == count {
            return;
        }
        self.ignored_device_count = count;
        {
            let mut snapshot = self.shared.snapshot.write();
            snapshot.ignored_device_count = count;
            snapshot.updated_at_ns = timestamp_ns;
        }
        self.events.emit(
            ControllerEventKind::AdditionalDevicesIgnored { count },
            timestamp_ns,
        );
    }

    fn is_active(&self, instance_id: u32) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.instance_id == instance_id)
    }
}

fn set_required_hints() -> Result<(), String> {
    let hints = [
        ("SDL_JOYSTICK_THREAD", "1"),
        ("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1"),
        ("SDL_JOYSTICK_HIDAPI", "1"),
        ("SDL_JOYSTICK_HIDAPI_PS5", "1"),
    ];
    for (name, value) in hints {
        if !sdl3::hint::set_with_priority(name, value, &sdl3::hint::Hint::Override) {
            return Err(format!("failed to configure SDL hint {name}"));
        }
    }
    Ok(())
}

fn map_button(button: Button) -> Option<ControllerButton> {
    match button {
        Button::North => Some(ControllerButton::Triangle),
        Button::East => Some(ControllerButton::Circle),
        Button::South => Some(ControllerButton::Cross),
        Button::West => Some(ControllerButton::Square),
        Button::Back => Some(ControllerButton::Create),
        Button::Guide => Some(ControllerButton::PlayStation),
        Button::Start => Some(ControllerButton::Options),
        Button::LeftStick => Some(ControllerButton::L3),
        Button::RightStick => Some(ControllerButton::R3),
        Button::LeftShoulder => Some(ControllerButton::L1),
        Button::RightShoulder => Some(ControllerButton::R1),
        Button::DPadUp => Some(ControllerButton::DPadUp),
        Button::DPadDown => Some(ControllerButton::DPadDown),
        Button::DPadLeft => Some(ControllerButton::DPadLeft),
        Button::DPadRight => Some(ControllerButton::DPadRight),
        Button::Misc1 => Some(ControllerButton::Microphone),
        Button::Touchpad => Some(ControllerButton::Touchpad),
        _ => None,
    }
}

fn map_axis(axis: Axis) -> ControllerAxis {
    match axis {
        Axis::LeftX => ControllerAxis::LeftX,
        Axis::LeftY => ControllerAxis::LeftY,
        Axis::RightX => ControllerAxis::RightX,
        Axis::RightY => ControllerAxis::RightY,
        Axis::TriggerLeft => ControllerAxis::L2,
        Axis::TriggerRight => ControllerAxis::R2,
    }
}

fn read_device(gamepad: &Gamepad, path: String) -> ControllerDevice {
    let connection = match gamepad
        .connection_state()
        .unwrap_or(ConnectionState::Unknown)
    {
        ConnectionState::Wired => ConnectionKind::Wired,
        ConnectionState::Wireless => ConnectionKind::Wireless,
        _ => ConnectionKind::Unknown,
    };
    let power = gamepad.power_info();
    let battery_state = match power.state {
        PowerLevel::Charging => BatteryState::Charging,
        PowerLevel::Charged => BatteryState::Charged,
        PowerLevel::OnBattery => BatteryState::Discharging,
        PowerLevel::NoBattery => BatteryState::NotPresent,
        _ => BatteryState::Unknown,
    };
    let percentage = (0..=100)
        .contains(&power.percentage)
        .then_some(power.percentage as u8);

    ControllerDevice {
        instance_id: gamepad.id().map(|id| id.0).unwrap_or_default(),
        name: gamepad
            .name()
            .unwrap_or_else(|| "DualSense Wireless Controller".into()),
        path: (!path.is_empty()).then_some(path),
        serial_number: gamepad.serial_number(),
        vendor_id: gamepad.vendor_id().unwrap_or(SONY_VENDOR_ID),
        product_id: gamepad.product_id().unwrap_or_default(),
        product_version: gamepad.product_version(),
        firmware_version: gamepad.firmware_version(),
        connection,
        battery: BatteryInfo {
            state: battery_state,
            percentage,
        },
    }
}

fn sdl_now_ns() -> u64 {
    sdl3::timer::ticks().saturating_mul(1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis_event(value: f32) -> ControllerEventKind {
        ControllerEventKind::AxisChanged {
            axis: ControllerAxis::LeftX,
            value,
        }
    }

    fn assert_reliable_when_channel_is_full(kind: ControllerEventKind) {
        let shared = Arc::new(SharedState::new(false));
        let (sender, mut receiver) = mpsc::channel(1);
        let mut events = EventSink::new(sender, shared);
        events.emit(axis_event(0.25), 1);

        let worker = std::thread::spawn(move || {
            events.emit(kind, 2);
        });

        let first = receiver.blocking_recv().expect("queued axis event");
        assert!(matches!(
            first.kind,
            ControllerEventKind::AxisChanged { .. }
        ));
        worker.join().expect("reliable event sender");
        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn positional_face_buttons_use_playstation_names() {
        assert_eq!(map_button(Button::North), Some(ControllerButton::Triangle));
        assert_eq!(map_button(Button::East), Some(ControllerButton::Circle));
        assert_eq!(map_button(Button::South), Some(ControllerButton::Cross));
        assert_eq!(map_button(Button::West), Some(ControllerButton::Square));
    }

    #[test]
    fn unsupported_extra_buttons_are_ignored() {
        assert_eq!(map_button(Button::RightPaddle1), None);
    }

    #[test]
    fn digital_and_lifecycle_events_are_lossless_under_axis_pressure() {
        assert_reliable_when_channel_is_full(ControllerEventKind::ButtonChanged {
            button: ControllerButton::Triangle,
            state: super::super::ButtonState::Released,
        });
        assert_reliable_when_channel_is_full(ControllerEventKind::Disconnected { instance_id: 7 });
        assert_reliable_when_channel_is_full(ControllerEventKind::PausedChanged { paused: true });
    }

    #[test]
    fn a_final_loss_reconciles_without_another_controller_event() {
        let shared = Arc::new(SharedState::new(false));
        let (sender, mut receiver) = mpsc::channel(1);
        let mut events = EventSink::new(sender, Arc::clone(&shared));

        events.emit(axis_event(0.25), 1);
        events.emit(axis_event(0.75), 2);
        shared
            .snapshot
            .write()
            .pressed_buttons
            .insert(ControllerButton::Triangle);

        let _ = receiver.try_recv().expect("queued axis event");
        events.flush_reconciliation(3);

        let reconciled = receiver.try_recv().expect("reconciliation event");
        match reconciled.kind {
            ControllerEventKind::EventsDropped { total, snapshot } => {
                assert_eq!(total, 1);
                assert_eq!(snapshot.dropped_event_count, 1);
                assert!(
                    snapshot
                        .pressed_buttons
                        .contains(&ControllerButton::Triangle)
                );
            }
            other => panic!("expected reconciliation event, got {other:?}"),
        }
    }
}
