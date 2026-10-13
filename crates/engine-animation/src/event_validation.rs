use super::event_clock::{ClockError, ScheduledEvent};

/// Validates clock duration and bounded delivery settings.
pub(super) fn validate_clock_limits(
    duration: f32,
    max_batch: usize,
    max_loop_crossings: usize,
) -> Result<(), ClockError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(ClockError::InvalidDuration);
    }
    if max_batch == 0 || max_loop_crossings == 0 {
        return Err(ClockError::EventOverflow);
    }
    Ok(())
}

/// Validates that every event lies within the clip duration.
pub(super) fn validate_events(duration: f32, events: &[ScheduledEvent]) -> Result<(), ClockError> {
    if events
        .iter()
        .any(|event| !event.time.is_finite() || event.time < 0.0 || event.time > duration)
    {
        return Err(ClockError::InvalidEventTime);
    }
    Ok(())
}

/// Validates an update delta before any clock mutation.
pub(super) fn validate_delta(delta: f32) -> Result<(), ClockError> {
    if !delta.is_finite() {
        return Err(ClockError::NonFiniteDelta);
    }
    if delta < 0.0 {
        return Err(ClockError::NegativeDelta);
    }
    Ok(())
}

/// Rejects a batch whose size exceeds its configured bound.
pub(super) fn ensure_capacity(actual: usize, capacity: usize) -> Result<(), ClockError> {
    if actual > capacity {
        Err(ClockError::EventOverflow)
    } else {
        Ok(())
    }
}
