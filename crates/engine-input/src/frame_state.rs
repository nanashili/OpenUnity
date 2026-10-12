use super::input_event_types::{
    ControlKind, DeviceDescriptor, DeviceId, DeviceKind, InputEvent, InputEventKind, InputValue,
    MAX_ACTIVE_DEVICES, MAX_DEVICE_LIFETIME,
};

#[path = "frame_state_clone.rs"]
mod frame_state_clone;
#[path = "frame_state_helpers.rs"]
mod frame_state_helpers;
#[path = "frame_state_queries.rs"]
mod frame_state_queries;
#[path = "frame_state_view.rs"]
mod frame_state_view;

use frame_state_helpers::{
    build_release_tombstones, clear_non_buttons, clone_string, neutral_value, release_buttons,
    validate_descriptor, validate_value,
};

/// Failure returned when an input event cannot update frame state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// A scalar or vector component is not finite.
    NonFiniteValue,
    /// The value shape differs from the control's declared kind.
    TypeMismatch,
    /// A device or control path has no applicable state.
    UnknownControl,
    /// The identifier has already been used since the last reset.
    RetiredDevice,
    /// A configured device bound or state allocation was exceeded.
    CapacityOverflow,
}

#[derive(Clone, Debug)]
pub(super) struct ControlState {
    pub(super) path: String,
    pub(super) kind: ControlKind,
    pub(super) value: InputValue,
    pub(super) pressed: bool,
    pub(super) released: bool,
}

#[derive(Clone, Debug)]
pub(super) struct DeviceState {
    pub(super) id: DeviceId,
    pub(super) controls: Vec<ControlState>,
}

#[derive(Clone, Debug)]
pub(super) struct ReleaseTombstone {
    pub(super) device: DeviceId,
    pub(super) path: String,
    pub(super) released: bool,
}

#[derive(Clone, Debug, Default)]
/// Frame-local snapshot of device controls and button edges.
pub struct FrameInput {
    pub(super) devices: Vec<DeviceState>,
    pub(super) known_devices: Vec<DeviceId>,
    pub(super) releases: Vec<ReleaseTombstone>,
}

impl FrameInput {
    /// Construct an empty frame snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new frame and clear press/release edges from the prior frame.
    pub fn begin_frame(&mut self) -> Result<(), FrameError> {
        for device in &mut self.devices {
            for control in &mut device.controls {
                control.pressed = false;
                control.released = false;
            }
        }
        self.releases.clear();
        Ok(())
    }

    /// Apply one validated lifecycle or control-change event to this snapshot.
    pub fn apply(&mut self, event: &InputEvent) -> Result<(), FrameError> {
        match &event.kind {
            InputEventKind::Connected(descriptor) => self.connect(descriptor),
            InputEventKind::Disconnected(device) => self.disconnect(*device),
            InputEventKind::Changed {
                device,
                control,
                value,
            } => self.change(*device, control, value),
        }
    }

    /// Remove a known device and preserve release edges for the current frame.
    pub fn release_device(&mut self, device: DeviceId) -> Result<(), FrameError> {
        if let Ok(index) = self.device_index(device) {
            let tombstones = build_release_tombstones(&self.devices[index])?;
            self.releases
                .try_reserve(tombstones.len())
                .map_err(|_| FrameError::CapacityOverflow)?;

            self.devices.remove(index);
            self.releases.extend(tombstones);
        } else if !self.known_devices.contains(&device) {
            return Err(FrameError::UnknownControl);
        }
        Ok(())
    }

    /// Release held buttons and neutralize non-button controls after focus loss.
    pub fn focus_lost(&mut self) {
        self.devices.sort_by_key(|device| device.id.to_raw());
        for device in &mut self.devices {
            release_buttons(device);
            clear_non_buttons(device);
        }
    }

    /// Clear all controls, release edges, and remembered device identifiers.
    pub fn reset(&mut self) {
        self.devices.clear();
        self.known_devices.clear();
        self.releases.clear();
    }

    fn connect(&mut self, descriptor: &DeviceDescriptor) -> Result<(), FrameError> {
        validate_descriptor(descriptor)?;

        if self.devices.iter().any(|device| device.id == descriptor.id) {
            return Err(FrameError::UnknownControl);
        }
        if self.devices.len() >= MAX_ACTIVE_DEVICES {
            return Err(FrameError::CapacityOverflow);
        }
        if self.known_devices.contains(&descriptor.id) {
            return Err(FrameError::RetiredDevice);
        }
        if self.known_devices.len() >= MAX_DEVICE_LIFETIME {
            return Err(FrameError::CapacityOverflow);
        }

        let mut controls = Vec::new();
        controls
            .try_reserve(descriptor.controls.len())
            .map_err(|_| FrameError::CapacityOverflow)?;
        for descriptor in &descriptor.controls {
            controls.push(ControlState {
                path: clone_string(&descriptor.path)?,
                kind: descriptor.kind,
                value: neutral_value(descriptor.kind),
                pressed: false,
                released: false,
            });
        }
        controls.sort_by(|left, right| left.path.cmp(&right.path));

        self.devices
            .try_reserve(1)
            .map_err(|_| FrameError::CapacityOverflow)?;
        self.known_devices
            .try_reserve(1)
            .map_err(|_| FrameError::CapacityOverflow)?;
        self.known_devices.push(descriptor.id);
        self.devices.push(DeviceState {
            id: descriptor.id,
            controls,
        });
        Ok(())
    }

    fn disconnect(&mut self, device: DeviceId) -> Result<(), FrameError> {
        self.release_device(device)
    }

    fn change(
        &mut self,
        device: DeviceId,
        path: &str,
        value: &InputValue,
    ) -> Result<(), FrameError> {
        let control = self.lookup_control_mut(device, path)?;
        validate_value(value, control.kind)?;

        let previous = control.value.clone();
        control.value = value.clone();
        if let (InputValue::Button(before), InputValue::Button(after)) = (previous, value) {
            control.pressed |= !before && *after;
            control.released |= before && !*after;
        }
        Ok(())
    }

    pub(super) fn lookup(&self, device: DeviceId, path: &str) -> Result<&InputValue, FrameError> {
        Ok(&self.lookup_control(device, path)?.value)
    }

    pub(super) fn lookup_control(
        &self,
        device: DeviceId,
        path: &str,
    ) -> Result<&ControlState, FrameError> {
        let device = self
            .devices
            .iter()
            .find(|candidate| candidate.id == device)
            .ok_or(FrameError::UnknownControl)?;
        device
            .controls
            .iter()
            .find(|control| control.path == path)
            .ok_or(FrameError::UnknownControl)
    }

    fn lookup_control_mut(
        &mut self,
        device: DeviceId,
        path: &str,
    ) -> Result<&mut ControlState, FrameError> {
        let device = self
            .devices
            .iter_mut()
            .find(|candidate| candidate.id == device)
            .ok_or(FrameError::UnknownControl)?;
        device
            .controls
            .iter_mut()
            .find(|control| control.path == path)
            .ok_or(FrameError::UnknownControl)
    }

    fn device_index(&self, device: DeviceId) -> Result<usize, FrameError> {
        self.devices
            .iter()
            .position(|candidate| candidate.id == device)
            .ok_or(FrameError::UnknownControl)
    }
}
