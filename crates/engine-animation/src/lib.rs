//! Validated animation clips, curves, transform tracks, and state events.
#![forbid(unsafe_code)]

mod clip_data;
mod curve;
mod state_machine;
/// Transform-channel sampling and ordered blending.
pub mod transform_track;

pub use clip_data::{
    AnimationClip, Channel, ClipDataError, ClipDuration, FiniteValue, KeyTime, Keyframe, Track,
    TrackId,
};
pub use curve::{Curve, CurveError, Extrapolation, InterpolationMode};
pub use state_machine::{
    AnimationClock, AnimationStateId, ClockError, ConditionKey, InterruptionRule, PlaybackMode,
    ScheduledEvent, StateCommand, StateError, StateGraph, StateMachine, StateRecord, Transition,
};
