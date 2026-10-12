use engine_identifiers::ObjectId;
use engine_math::Vector;

/// Identifier for an input device within an event queue and frame state.
pub type DeviceId = ObjectId;

/// Maximum number of devices that may be active at once.
pub const MAX_ACTIVE_DEVICES: usize = 4096;

/// Maximum number of distinct device IDs accepted before the state must reset.
pub const MAX_DEVICE_LIFETIME: usize = 4096;

/// Metadata used to register one input device and its controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceDescriptor {
    /// Stable identifier supplied by the device producer.
    pub id: DeviceId,
    /// Human-readable device name.
    pub name: String,
    /// Broad device category.
    pub kind: DeviceKind,
    /// Controls addressable on this device.
    pub controls: Vec<ControlDescriptor>,
}

/// Broad category assigned to an input device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    /// A keyboard-like device.
    Keyboard,
    /// A pointer-like device.
    Pointer,
    /// A game controller.
    Gamepad,
    /// A touch surface.
    Touch,
    /// A producer-defined category and its name.
    Custom(String),
}

/// Path and value type for a single device control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlDescriptor {
    /// Stable path used by change events and action bindings.
    pub path: String,
    /// Value shape accepted for this control.
    pub kind: ControlKind,
}

/// Supported value shapes for input controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    /// A two-state pressed or released value.
    Button,
    /// One scalar value.
    Axis,
    /// A two-dimensional vector value.
    Vector2,
    /// A three-dimensional vector value.
    Vector3,
}

/// Typed value carried by an input change event.
#[derive(Clone, Debug, PartialEq)]
pub enum InputValue {
    /// A pressed (`true`) or released (`false`) state.
    Button(bool),
    /// A scalar axis value.
    Scalar(f32),
    /// A two-dimensional vector value.
    Vector2([f32; 2]),
    /// A three-dimensional vector value.
    Vector3(Vector),
}

/// An input lifecycle or control-change operation.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEventKind {
    /// Register a device and its controls.
    Connected(DeviceDescriptor),
    /// Retire an active device.
    Disconnected(DeviceId),
    /// Change the value of a registered control.
    Changed {
        /// Device that owns the control.
        device: DeviceId,
        /// Registered control path.
        control: String,
        /// New finite value matching the control's declared kind.
        value: InputValue,
    },
}

/// One sequenced input operation with its source timestamp.
#[derive(Clone, Debug, PartialEq)]
pub struct InputEvent {
    /// Strictly increasing sequence within one event queue.
    pub sequence: u64,
    /// Source timestamp expressed in nanoseconds.
    pub timestamp_ns: u64,
    /// Lifecycle or value operation to apply.
    pub kind: InputEventKind,
}

/// Failure returned while validating or enqueuing an input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventError {
    /// Device metadata or a control descriptor is invalid.
    InvalidDescriptor,
    /// An event refers to a device that is not active.
    UnknownDevice,
    /// An event refers to a control that is not registered.
    UnknownControl,
    /// The event repeats the last accepted sequence number.
    DuplicateSequence,
    /// The event sequence precedes the last accepted sequence number.
    StaleSequence,
    /// A scalar or vector component is not finite.
    NonFiniteValue,
    /// The value shape differs from the control's declared kind.
    TypeMismatch,
    /// The identifier was already used during this queue's lifetime.
    RetiredDevice,
    /// The bounded event queue has no remaining slots.
    QueueFull,
    /// A bounded allocation or device limit could not be satisfied.
    CapacityOverflow,
}
