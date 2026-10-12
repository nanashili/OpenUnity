use std::collections::{HashMap, HashSet, VecDeque};

use super::input_event_types::{
    ControlKind, DeviceDescriptor, DeviceId, DeviceKind, EventError, InputEvent, InputEventKind,
    InputValue, MAX_ACTIVE_DEVICES, MAX_DEVICE_LIFETIME,
};

#[path = "event_queue_clone.rs"]
mod event_queue_clone;

const MAX_NAME_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 256;
const MAX_CUSTOM_KIND_BYTES: usize = 64;
const MAX_CONTROLS: usize = 1024;

/// A bounded queue that validates input events before accepting them.
#[derive(Debug)]
pub struct EventQueue {
    capacity: usize,
    events: VecDeque<InputEvent>,
    devices: HashMap<DeviceId, DeviceDescriptor>,
    used_device_ids: HashSet<DeviceId>,
    last_sequence: Option<u64>,
}

impl EventQueue {
    /// Create an empty queue with the requested nonzero event capacity.
    pub fn with_capacity(capacity: usize) -> Result<Self, EventError> {
        if capacity == 0 {
            return Err(EventError::CapacityOverflow);
        }
        let mut events = VecDeque::new();
        events
            .try_reserve(capacity)
            .map_err(|_| EventError::CapacityOverflow)?;
        Ok(Self {
            capacity,
            events,
            devices: HashMap::new(),
            used_device_ids: HashSet::new(),
            last_sequence: None,
        })
    }

    /// Validate and append an event, leaving queue state unchanged on failure.
    pub fn push(&mut self, event: InputEvent) -> Result<(), EventError> {
        self.validate_sequence(event.sequence)?;
        if self.events.len() == self.capacity {
            return Err(EventError::QueueFull);
        }
        self.validate_kind(&event.kind)?;

        let connected = match &event.kind {
            InputEventKind::Connected(descriptor) => {
                Some(event_queue_clone::try_clone_descriptor(descriptor)?)
            }
            _ => None,
        };

        self.events
            .try_reserve(1)
            .map_err(|_| EventError::CapacityOverflow)?;

        if matches!(&event.kind, InputEventKind::Connected(_)) {
            self.reserve_connection()?;
        }
        let disconnect = match &event.kind {
            InputEventKind::Disconnected(device) => Some(*device),
            _ => None,
        };
        let sequence = event.sequence;

        self.events.push_back(event);
        self.last_sequence = Some(sequence);

        if let Some(device) = disconnect {
            self.devices.remove(&device);
        }

        if let Some(descriptor) = connected {
            self.used_device_ids.insert(descriptor.id);
            self.devices.insert(descriptor.id, descriptor);
        }
        Ok(())
    }

    /// Return the number of events waiting to be drained.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Return whether the queue contains no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Append all queued events to `destination` in acceptance order.
    ///
    /// If the destination cannot reserve space for every event, neither the
    /// queue nor the destination is changed.
    pub fn drain_into(&mut self, destination: &mut Vec<InputEvent>) -> Result<usize, EventError> {
        let event_count = self.events.len();
        destination
            .try_reserve(event_count)
            .map_err(|_| EventError::CapacityOverflow)?;

        while let Some(event) = self.events.pop_front() {
            destination.push(event);
        }

        Ok(event_count)
    }

    fn validate_sequence(&self, sequence: u64) -> Result<(), EventError> {
        match self.last_sequence {
            Some(last) if sequence == last => Err(EventError::DuplicateSequence),
            Some(last) if sequence < last => Err(EventError::StaleSequence),
            _ => Ok(()),
        }
    }

    fn validate_kind(&self, kind: &InputEventKind) -> Result<(), EventError> {
        match kind {
            InputEventKind::Connected(descriptor) => self.validate_descriptor(descriptor),
            InputEventKind::Disconnected(device) => self
                .devices
                .contains_key(device)
                .then_some(())
                .ok_or(EventError::UnknownDevice),
            InputEventKind::Changed {
                device,
                control,
                value,
            } => {
                let descriptor = self.devices.get(device).ok_or(EventError::UnknownDevice)?;
                let control_kind = descriptor
                    .controls
                    .iter()
                    .find(|item| item.path == *control)
                    .map(|item| item.kind)
                    .ok_or(EventError::UnknownControl)?;
                validate_value(value, control_kind)
            }
        }
    }

    fn validate_descriptor(&self, descriptor: &DeviceDescriptor) -> Result<(), EventError> {
        if self.devices.contains_key(&descriptor.id) {
            return Err(EventError::InvalidDescriptor);
        }
        if self.used_device_ids.contains(&descriptor.id) {
            return Err(EventError::RetiredDevice);
        }
        if descriptor.name.is_empty()
            || descriptor.name.len() > MAX_NAME_BYTES
            || descriptor.controls.len() > MAX_CONTROLS
        {
            return Err(EventError::InvalidDescriptor);
        }

        if let DeviceKind::Custom(name) = &descriptor.kind {
            if name.len() > MAX_CUSTOM_KIND_BYTES {
                return Err(EventError::InvalidDescriptor);
            }
        }

        let mut paths = HashSet::new();
        paths
            .try_reserve(descriptor.controls.len())
            .map_err(|_| EventError::CapacityOverflow)?;

        for control in &descriptor.controls {
            if control.path.is_empty()
                || control.path.len() > MAX_PATH_BYTES
                || !paths.insert(&control.path)
            {
                return Err(EventError::InvalidDescriptor);
            }
        }
        Ok(())
    }

    fn reserve_connection(&mut self) -> Result<(), EventError> {
        if self.devices.len() >= MAX_ACTIVE_DEVICES
            || self.used_device_ids.len() >= MAX_DEVICE_LIFETIME
        {
            return Err(EventError::CapacityOverflow);
        }

        self.devices
            .try_reserve(1)
            .map_err(|_| EventError::CapacityOverflow)?;
        self.used_device_ids
            .try_reserve(1)
            .map_err(|_| EventError::CapacityOverflow)?;
        Ok(())
    }
}

fn validate_value(value: &InputValue, control_kind: ControlKind) -> Result<(), EventError> {
    let finite = match value {
        InputValue::Button(_) => true,
        InputValue::Scalar(value) => value.is_finite(),
        InputValue::Vector2(values) => values.iter().all(|value| value.is_finite()),
        InputValue::Vector3(values) => values.iter().all(|value| value.is_finite()),
    };
    if !finite {
        return Err(EventError::NonFiniteValue);
    }

    let matches = matches!(
        (control_kind, value),
        (ControlKind::Button, InputValue::Button(_))
            | (ControlKind::Axis, InputValue::Scalar(_))
            | (ControlKind::Vector2, InputValue::Vector2(_))
            | (ControlKind::Vector3, InputValue::Vector3(_))
    );
    matches.then_some(()).ok_or(EventError::TypeMismatch)
}
