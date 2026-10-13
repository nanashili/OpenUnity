//! Deterministic animation state transitions and bounded event delivery.
#![forbid(unsafe_code)]

#[path = "event_clock.rs"]
mod event_clock;
#[path = "event_validation.rs"]
mod event_validation;
#[path = "state_graph.rs"]
mod state_graph;
#[path = "state_machine_runtime.rs"]
mod state_machine_runtime;

pub use event_clock::{AnimationClock, ClockError, PlaybackMode, ScheduledEvent};
pub use state_graph::{
    AnimationStateId, ConditionKey, InterruptionRule, StateError, StateGraph, Transition,
};
pub use state_machine_runtime::{StateCommand, StateMachine, StateRecord};
