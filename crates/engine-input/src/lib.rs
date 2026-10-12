//! Validated input events, frame state, action routing, and analog processing.

mod action_routing;
mod analog;
mod event_queue;
mod frame_state;
mod input_action_types;
mod input_event_types;

pub use action_routing::ActionRouter;
pub use analog::{AnalogError, AnalogSettings, process_analog};
pub use event_queue::EventQueue;
pub use frame_state::{FrameError, FrameInput};
pub use input_action_types::{
    ActionName, ActionSample, Binding, BindingError, MAX_BINDINGS, Trigger,
};
pub use input_event_types::{
    ControlDescriptor, ControlKind, DeviceDescriptor, DeviceId, DeviceKind, EventError, InputEvent,
    InputEventKind, InputValue, MAX_ACTIVE_DEVICES, MAX_DEVICE_LIFETIME,
};
