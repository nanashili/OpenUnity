use std::collections::{HashMap, HashSet, VecDeque};

use super::super::input_event_types::{
    ControlDescriptor, DeviceDescriptor, DeviceKind, EventError, InputEvent, InputEventKind,
};
use super::EventQueue;

impl EventQueue {
    /// Build a fallible deep snapshot without changing this queue.
    pub fn try_clone(&self) -> Result<Self, EventError> {
        let mut events = VecDeque::new();
        events
            .try_reserve(self.events.len())
            .map_err(|_| EventError::CapacityOverflow)?;
        for event in &self.events {
            events.push_back(try_clone_event(event)?);
        }

        let mut devices = HashMap::new();
        devices
            .try_reserve(self.devices.len())
            .map_err(|_| EventError::CapacityOverflow)?;
        for (id, descriptor) in &self.devices {
            devices.insert(*id, try_clone_descriptor(descriptor)?);
        }

        let mut used_device_ids = HashSet::new();
        used_device_ids
            .try_reserve(self.used_device_ids.len())
            .map_err(|_| EventError::CapacityOverflow)?;
        used_device_ids.extend(self.used_device_ids.iter().copied());

        Ok(Self {
            capacity: self.capacity,
            events,
            devices,
            used_device_ids,
            last_sequence: self.last_sequence,
        })
    }
}

pub(super) fn try_clone_event(event: &InputEvent) -> Result<InputEvent, EventError> {
    let kind = match &event.kind {
        InputEventKind::Connected(descriptor) => {
            InputEventKind::Connected(try_clone_descriptor(descriptor)?)
        }
        InputEventKind::Disconnected(device) => InputEventKind::Disconnected(*device),
        InputEventKind::Changed {
            device,
            control,
            value,
        } => InputEventKind::Changed {
            device: *device,
            control: try_clone_string(control)?,
            value: value.clone(),
        },
    };

    Ok(InputEvent {
        sequence: event.sequence,
        timestamp_ns: event.timestamp_ns,
        kind,
    })
}

pub(super) fn try_clone_descriptor(
    descriptor: &DeviceDescriptor,
) -> Result<DeviceDescriptor, EventError> {
    let mut controls = Vec::new();
    controls
        .try_reserve(descriptor.controls.len())
        .map_err(|_| EventError::CapacityOverflow)?;
    for control in &descriptor.controls {
        controls.push(clone_control(control)?);
    }
    Ok(DeviceDescriptor {
        id: descriptor.id,
        name: try_clone_string(&descriptor.name)?,
        kind: try_clone_device_kind(&descriptor.kind)?,
        controls,
    })
}

fn try_clone_device_kind(kind: &DeviceKind) -> Result<DeviceKind, EventError> {
    match kind {
        DeviceKind::Keyboard => Ok(DeviceKind::Keyboard),
        DeviceKind::Pointer => Ok(DeviceKind::Pointer),
        DeviceKind::Gamepad => Ok(DeviceKind::Gamepad),
        DeviceKind::Touch => Ok(DeviceKind::Touch),
        DeviceKind::Custom(name) => Ok(DeviceKind::Custom(try_clone_string(name)?)),
    }
}

fn clone_control(control: &ControlDescriptor) -> Result<ControlDescriptor, EventError> {
    Ok(ControlDescriptor {
        path: try_clone_string(&control.path)?,
        kind: control.kind,
    })
}

fn try_clone_string(value: &str) -> Result<String, EventError> {
    let mut clone = String::new();
    clone
        .try_reserve(value.len())
        .map_err(|_| EventError::CapacityOverflow)?;
    clone.push_str(value);
    Ok(clone)
}
