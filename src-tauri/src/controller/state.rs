use std::collections::{BTreeMap, BTreeSet};

use super::{ButtonState, ControllerAxis, ControllerButton, TriggerThresholds};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NormalizedInput {
    pub timestamp_ns: u64,
    pub kind: NormalizedInputKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NormalizedInputKind {
    Button {
        button: ControllerButton,
        state: ButtonState,
    },
    Axis {
        axis: ControllerAxis,
        value: f32,
    },
}

#[derive(Clone, Copy, Debug, Default)]
struct DigitalLatch {
    physical: bool,
    active: bool,
}

impl DigitalLatch {
    fn observe(&mut self, pressed: bool, emit: bool) -> Option<ButtonState> {
        if self.physical == pressed {
            return None;
        }

        self.physical = pressed;
        if pressed {
            if emit {
                self.active = true;
                Some(ButtonState::Pressed)
            } else {
                None
            }
        } else if self.active {
            self.active = false;
            Some(ButtonState::Released)
        } else {
            None
        }
    }

    fn deactivate(&mut self) -> bool {
        let was_active = self.active;
        self.active = false;
        was_active
    }
}

#[derive(Clone, Copy, Debug)]
struct HysteresisGate {
    thresholds: TriggerThresholds,
    pressed: bool,
}

impl HysteresisGate {
    fn new(thresholds: TriggerThresholds) -> Self {
        Self {
            thresholds,
            pressed: false,
        }
    }

    fn observe(&mut self, value: f32) -> Option<bool> {
        if !self.pressed && value >= self.thresholds.press {
            self.pressed = true;
            Some(true)
        } else if self.pressed && value <= self.thresholds.release {
            self.pressed = false;
            Some(false)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TriggerLatch {
    gate: HysteresisGate,
    active: bool,
}

impl TriggerLatch {
    fn new(thresholds: TriggerThresholds) -> Self {
        Self {
            gate: HysteresisGate::new(thresholds),
            active: false,
        }
    }

    fn observe(&mut self, value: f32, emit: bool) -> Option<ButtonState> {
        match self.gate.observe(value) {
            Some(true) if emit => {
                self.active = true;
                Some(ButtonState::Pressed)
            }
            Some(false) if self.active => {
                self.active = false;
                Some(ButtonState::Released)
            }
            _ => None,
        }
    }

    fn deactivate(&mut self) -> bool {
        let was_active = self.active;
        self.active = false;
        was_active
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AxisLatch {
    current: f32,
    emitted: f32,
    last_emitted_at_ns: u64,
}

impl AxisLatch {
    fn update(&mut self, value: f32) {
        self.current = value;
    }

    fn take_pending(&mut self, timestamp_ns: u64, epsilon: f32, interval_ns: u64) -> Option<f32> {
        if (self.current - self.emitted).abs() < epsilon
            || timestamp_ns.saturating_sub(self.last_emitted_at_ns) < interval_ns
        {
            return None;
        }

        self.emitted = self.current;
        self.last_emitted_at_ns = timestamp_ns;
        Some(self.current)
    }
}

pub(crate) struct InputNormalizer {
    buttons: BTreeMap<ControllerButton, DigitalLatch>,
    triggers: BTreeMap<ControllerButton, TriggerLatch>,
    axes: BTreeMap<ControllerAxis, AxisLatch>,
    axis_epsilon: f32,
    axis_interval_ns: u64,
    paused: bool,
}

impl InputNormalizer {
    pub fn new(
        thresholds: TriggerThresholds,
        axis_epsilon: f32,
        axis_interval_ns: u64,
        paused: bool,
    ) -> Self {
        Self {
            buttons: BTreeMap::new(),
            triggers: BTreeMap::from([
                (ControllerButton::L2, TriggerLatch::new(thresholds)),
                (ControllerButton::R2, TriggerLatch::new(thresholds)),
            ]),
            axes: ControllerAxis::ALL
                .into_iter()
                .map(|axis| (axis, AxisLatch::default()))
                .collect(),
            axis_epsilon,
            axis_interval_ns,
            paused,
        }
    }

    pub fn button(
        &mut self,
        button: ControllerButton,
        pressed: bool,
        timestamp_ns: u64,
    ) -> Option<NormalizedInput> {
        let state = self
            .buttons
            .entry(button)
            .or_default()
            .observe(pressed, true)?;
        Some(NormalizedInput {
            timestamp_ns,
            kind: NormalizedInputKind::Button { button, state },
        })
    }

    pub fn axis(
        &mut self,
        axis: ControllerAxis,
        raw_value: i16,
        timestamp_ns: u64,
    ) -> Vec<NormalizedInput> {
        let value = normalize_axis(axis, raw_value);
        let mut inputs = Vec::with_capacity(2);

        if let Some(button) = trigger_button(axis) {
            if let Some(state) = self.triggers.get_mut(&button).unwrap().observe(value, true) {
                inputs.push(NormalizedInput {
                    timestamp_ns,
                    kind: NormalizedInputKind::Button { button, state },
                });
            }
        }

        let latch = self.axes.get_mut(&axis).unwrap();
        latch.update(value);
        if let Some(value) =
            latch.take_pending(timestamp_ns, self.axis_epsilon, self.axis_interval_ns)
        {
            inputs.push(NormalizedInput {
                timestamp_ns,
                kind: NormalizedInputKind::Axis { axis, value },
            });
        }

        inputs
    }

    pub fn flush_axes(&mut self, timestamp_ns: u64) -> Vec<NormalizedInput> {
        self.axes
            .iter_mut()
            .filter_map(|(axis, latch)| {
                latch
                    .take_pending(timestamp_ns, self.axis_epsilon, self.axis_interval_ns)
                    .map(|value| NormalizedInput {
                        timestamp_ns,
                        kind: NormalizedInputKind::Axis { axis: *axis, value },
                    })
            })
            .collect()
    }

    pub fn set_paused(&mut self, paused: bool, _timestamp_ns: u64) -> Vec<NormalizedInput> {
        if self.paused == paused {
            return Vec::new();
        }
        self.paused = paused;
        Vec::new()
    }

    pub fn disconnect(&mut self, timestamp_ns: u64) -> Vec<NormalizedInput> {
        let mut inputs = Vec::new();
        for (button, latch) in &mut self.buttons {
            if latch.deactivate() {
                inputs.push(button_input(*button, ButtonState::Released, timestamp_ns));
            }
        }
        for (button, latch) in &mut self.triggers {
            if latch.deactivate() {
                inputs.push(button_input(*button, ButtonState::Released, timestamp_ns));
            }
        }

        self.buttons.clear();
        for latch in self.triggers.values_mut() {
            latch.gate.pressed = false;
            latch.active = false;
        }
        for latch in self.axes.values_mut() {
            *latch = AxisLatch::default();
        }
        inputs
    }

    pub fn pressed_buttons(&self) -> BTreeSet<ControllerButton> {
        self.buttons
            .iter()
            .filter_map(|(button, latch)| latch.active.then_some(*button))
            .chain(
                self.triggers
                    .iter()
                    .filter_map(|(button, latch)| latch.active.then_some(*button)),
            )
            .collect()
    }

    pub fn axes(&self) -> Vec<(ControllerAxis, f32)> {
        self.axes
            .iter()
            .map(|(axis, latch)| (*axis, latch.current))
            .collect()
    }
}

fn button_input(
    button: ControllerButton,
    state: ButtonState,
    timestamp_ns: u64,
) -> NormalizedInput {
    NormalizedInput {
        timestamp_ns,
        kind: NormalizedInputKind::Button { button, state },
    }
}

fn trigger_button(axis: ControllerAxis) -> Option<ControllerButton> {
    match axis {
        ControllerAxis::L2 => Some(ControllerButton::L2),
        ControllerAxis::R2 => Some(ControllerButton::R2),
        _ => None,
    }
}

fn normalize_axis(axis: ControllerAxis, raw_value: i16) -> f32 {
    match axis {
        ControllerAxis::L2 | ControllerAxis::R2 => {
            f32::from(raw_value.max(0)) / f32::from(i16::MAX)
        }
        _ if raw_value < 0 => f32::from(raw_value) / 32768.0,
        _ => f32::from(raw_value) / f32::from(i16::MAX),
    }
    .clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalizer() -> InputNormalizer {
        InputNormalizer::new(TriggerThresholds::default(), 0.01, 10, false)
    }

    #[test]
    fn digital_buttons_are_edge_triggered_with_original_timestamps() {
        let mut state = normalizer();

        let pressed = state.button(ControllerButton::Triangle, true, 123).unwrap();
        assert_eq!(pressed.timestamp_ns, 123);
        assert_eq!(
            pressed.kind,
            NormalizedInputKind::Button {
                button: ControllerButton::Triangle,
                state: ButtonState::Pressed
            }
        );
        assert!(
            state
                .button(ControllerButton::Triangle, true, 124)
                .is_none()
        );

        let released = state
            .button(ControllerButton::Triangle, false, 456)
            .unwrap();
        assert_eq!(released.timestamp_ns, 456);
        assert_eq!(
            released.kind,
            NormalizedInputKind::Button {
                button: ControllerButton::Triangle,
                state: ButtonState::Released
            }
        );
    }

    #[test]
    fn trigger_hysteresis_rejects_threshold_chatter() {
        let mut state = normalizer();

        assert!(
            !state
                .axis(ControllerAxis::L2, 17_000, 10)
                .iter()
                .any(|input| matches!(input.kind, NormalizedInputKind::Button { .. }))
        );
        let pressed = state.axis(ControllerAxis::L2, 19_000, 20);
        assert!(pressed.iter().any(|input| {
            input.kind
                == NormalizedInputKind::Button {
                    button: ControllerButton::L2,
                    state: ButtonState::Pressed,
                }
        }));
        assert!(
            !state
                .axis(ControllerAxis::L2, 16_000, 30)
                .iter()
                .any(|input| matches!(input.kind, NormalizedInputKind::Button { .. }))
        );
        let released = state.axis(ControllerAxis::L2, 14_000, 40);
        assert!(released.iter().any(|input| {
            input.kind
                == NormalizedInputKind::Button {
                    button: ControllerButton::L2,
                    state: ButtonState::Released,
                }
        }));
    }

    #[test]
    fn pause_keeps_input_observation_active_without_synthetic_edges() {
        let mut state = normalizer();

        state.button(ControllerButton::Cross, true, 1).unwrap();
        assert!(state.set_paused(true, 2).is_empty());
        assert!(state.pressed_buttons().contains(&ControllerButton::Cross));
        let released = state.button(ControllerButton::Cross, false, 3).unwrap();
        assert_eq!(
            released.kind,
            NormalizedInputKind::Button {
                button: ControllerButton::Cross,
                state: ButtonState::Released
            }
        );
        state.set_paused(false, 4);
        assert_eq!(
            state.button(ControllerButton::Cross, true, 5).unwrap().kind,
            NormalizedInputKind::Button {
                button: ControllerButton::Cross,
                state: ButtonState::Pressed
            }
        );
    }

    #[test]
    fn axis_normalization_covers_full_ranges() {
        assert_eq!(normalize_axis(ControllerAxis::LeftX, i16::MIN), -1.0);
        assert_eq!(normalize_axis(ControllerAxis::LeftX, i16::MAX), 1.0);
        assert_eq!(normalize_axis(ControllerAxis::L2, -1), 0.0);
        assert_eq!(normalize_axis(ControllerAxis::R2, i16::MAX), 1.0);
    }

    #[test]
    fn pending_axis_value_is_flushed_after_rate_limit() {
        let mut state = normalizer();

        assert!(state.axis(ControllerAxis::LeftX, 10_000, 5).is_empty());
        let flushed = state.flush_axes(10);
        assert_eq!(flushed.len(), 1);
        assert!(matches!(
            flushed[0].kind,
            NormalizedInputKind::Axis {
                axis: ControllerAxis::LeftX,
                ..
            }
        ));
    }
}
