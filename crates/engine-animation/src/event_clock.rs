use super::event_validation::{validate_clock_limits, validate_delta, validate_events};
#[path = "traversal_plan.rs"]
mod traversal_plan;

use traversal_plan::TraversalPlan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Defines how an animation clock traverses its duration.
pub enum PlaybackMode {
    /// Advances to the duration endpoint and then stops.
    Once,
    /// Wraps to zero after each duration.
    Loop,
    /// Reverses direction at each duration endpoint.
    PingPong,
}

#[derive(Clone, Debug, PartialEq)]
/// An event authored at a normalized clock time.
pub struct ScheduledEvent {
    /// Event time in seconds from the start of the clip.
    pub time: f32,
    /// Stable event identifier delivered to the caller.
    pub key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Errors produced while creating or advancing an animation clock.
pub enum ClockError {
    /// The duration is zero, negative, or non-finite.
    InvalidDuration,
    /// An event time is outside the clip or non-finite.
    InvalidEventTime,
    /// The requested delta is non-finite.
    NonFiniteDelta,
    /// The requested delta is negative.
    NegativeDelta,
    /// The requested event batch or crossing count exceeds its bound.
    EventOverflow,
    /// The resulting clock time cannot be represented.
    TimeOverflow,
    /// Repeating traversal arithmetic cannot preserve a positive residual.
    PrecisionLoss,
}

/// Advances authored events with deterministic bounded delivery.
pub struct AnimationClock {
    duration: f32,
    mode: PlaybackMode,
    time: f32,
    forward: bool,
    events: Vec<ScheduledEvent>,
    max_batch: usize,
    max_loop_crossings: usize,
}

impl AnimationClock {
    /// Creates a clock after validating duration, events, and delivery bounds.
    pub fn new(
        duration: f32,
        mode: PlaybackMode,
        mut events: Vec<ScheduledEvent>,
        max_batch: usize,
        max_loop_crossings: usize,
    ) -> Result<Self, ClockError> {
        validate_clock_limits(duration, max_batch, max_loop_crossings)?;
        validate_events(duration, &events)?;

        events.sort_by(|left, right| {
            if left.time == right.time {
                std::cmp::Ordering::Equal
            } else {
                left.time.total_cmp(&right.time)
            }
        });

        Ok(Self {
            duration,
            mode,
            time: 0.0,
            forward: true,
            events,
            max_batch,
            max_loop_crossings,
        })
    }

    /// Returns the current position in seconds.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Advances the clock and returns events crossed by the update.
    pub fn advance(&mut self, delta: f32) -> Result<Vec<ScheduledEvent>, ClockError> {
        validate_delta(delta)?;

        let plan = TraversalPlan::build(
            self.mode,
            self.duration,
            self.time,
            self.forward,
            delta,
            self.max_loop_crossings,
        )?;

        let events = plan.collect_events(&self.events, self.max_batch)?;

        self.time = plan.final_time();
        self.forward = plan.final_direction();

        Ok(events)
    }
}
