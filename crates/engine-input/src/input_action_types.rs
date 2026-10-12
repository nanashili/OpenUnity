use super::input_event_types::{DeviceId, InputValue};

const MAX_ACTION_NAME_BYTES: usize = 128;

/// A validated action identifier used by action bindings.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionName(String);

impl ActionName {
    /// Create an action name containing between 1 and 128 UTF-8 bytes.
    pub fn new(name: String) -> Result<Self, BindingError> {
        if name.is_empty() || name.len() > MAX_ACTION_NAME_BYTES {
            return Err(BindingError::InvalidName);
        }

        Ok(Self(name))
    }

    /// Return the action name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Input state edge or value selected by an action binding.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Trigger {
    /// Sample a currently held button.
    Held,
    /// Sample a button pressed in the current frame.
    Pressed,
    /// Sample a button released in the current frame.
    Released,
    /// Sample the current typed control value.
    Value,
}

/// Associates an action with a control, optional device, and ordering priority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    /// Action name emitted for matching input.
    pub action: ActionName,
    /// Restrict the binding to one device, or match every active device.
    pub device: Option<DeviceId>,
    /// Registered control path to sample.
    pub control: String,
    /// Kind of state or value to sample.
    pub trigger: Trigger,
    /// Higher values sort before lower values in a sample result.
    pub priority: i32,
}

/// Failure returned when an action binding cannot be installed or removed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingError {
    /// The action name is empty or exceeds its byte limit.
    InvalidName,
    /// The control path is empty.
    EmptyControl,
    /// An identical action, device, control, and trigger binding exists.
    DuplicateBinding,
    /// The bounded binding count or a required allocation was exceeded.
    CapacityOverflow,
}

/// One action value emitted from a frame snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionSample {
    /// Action associated with the sample.
    pub action: ActionName,
    /// State edge or value selected by the binding.
    pub trigger: Trigger,
    /// Typed value captured from the frame snapshot.
    pub value: InputValue,
    /// Device that produced the sample, if the binding matched a device.
    pub device: Option<DeviceId>,
}

/// Maximum number of bindings retained by one action router.
pub const MAX_BINDINGS: usize = 4096;
