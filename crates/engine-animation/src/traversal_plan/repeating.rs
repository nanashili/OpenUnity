use super::precision::TimeUnits;
use super::{ClockError, PlaybackMode, Segment, TraversalPlan, boundary_for, reaches_boundary};

pub(super) fn build_repeating(
    mode: PlaybackMode,
    duration: f32,
    time: f32,
    forward: bool,
    delta: f32,
    max_crossings: usize,
) -> Result<TraversalPlan, ClockError> {
    let exact_duration = TimeUnits::from_seconds(duration);
    let mut remaining = TimeUnits::from_seconds(delta);
    let mut start = time;
    let mut direction = forward;
    let mut segments = Vec::new();
    let mut crossings = 0_usize;

    loop {
        let distance = boundary_distance(exact_duration, start, direction)?;
        let crosses_exactly = remaining >= distance;
        let interior_endpoint = if crosses_exactly {
            None
        } else {
            Some(rounded_interior_endpoint(start, direction, remaining)?)
        };
        let crosses_by_rounding = interior_endpoint
            .is_some_and(|endpoint| reaches_boundary(endpoint, duration, direction));
        let crosses = crosses_exactly || crosses_by_rounding;
        let end = if crosses {
            boundary_for(duration, direction)
        } else {
            interior_endpoint.ok_or(ClockError::PrecisionLoss)?
        };

        append_segment(&mut segments, start, end, direction, crosses, mode);

        if !crosses {
            return Ok(TraversalPlan {
                segments,
                final_time: end,
                final_forward: direction,
            });
        }

        crossings = crossings.checked_add(1).ok_or(ClockError::EventOverflow)?;
        if crossings > max_crossings {
            return Err(ClockError::EventOverflow);
        }

        remaining = residual_after_boundary(remaining, distance, crosses_exactly)?;
        if remaining == TimeUnits::ZERO {
            return Ok(plan_at_boundary(segments, duration, direction, mode));
        }

        (start, direction) = next_segment_start(duration, direction, mode);
    }
}

pub(super) fn ensure_minimum_crossings(
    duration: f32,
    time: f32,
    forward: bool,
    delta: f32,
    max_crossings: usize,
) -> Result<(), ClockError> {
    let exact_duration = TimeUnits::from_seconds(duration);
    let remaining = TimeUnits::from_seconds(delta);
    let first_distance = boundary_distance(exact_duration, time, forward)?;
    let rounded_crossing = remaining < first_distance
        && reaches_boundary(
            rounded_interior_endpoint(time, forward, remaining)?,
            duration,
            forward,
        );

    if remaining < first_distance && !rounded_crossing {
        return Ok(());
    }

    let excess = if rounded_crossing {
        TimeUnits::ZERO
    } else {
        remaining
            .checked_sub(first_distance)
            .ok_or(ClockError::PrecisionLoss)?
    };
    let Some(crossing_limit) = exact_duration.checked_mul_usize(max_crossings) else {
        // An overflowing threshold exceeds 320 bits; a finite f32 delta uses
        // at most 277 bits in subnormal units, so it cannot reach that bound.
        return Ok(());
    };

    if excess >= crossing_limit {
        return Err(ClockError::EventOverflow);
    }

    Ok(())
}

pub(super) fn rounded_endpoint(
    start: f32,
    forward: bool,
    remaining: f64,
) -> Result<f32, ClockError> {
    let start = f64::from(start);
    let endpoint = if forward {
        start + remaining
    } else {
        start - remaining
    };
    if endpoint.is_finite() {
        Ok(endpoint as f32)
    } else {
        Err(ClockError::TimeOverflow)
    }
}

fn append_segment(
    segments: &mut Vec<Segment>,
    start: f32,
    end: f32,
    direction: bool,
    crosses: bool,
    mode: PlaybackMode,
) {
    if end == start {
        return;
    }

    segments.push(Segment {
        start,
        end,
        forward: direction,
        loop_boundary_after: crosses && mode == PlaybackMode::Loop,
    });
}

fn residual_after_boundary(
    remaining: TimeUnits,
    distance: TimeUnits,
    crosses_exactly: bool,
) -> Result<TimeUnits, ClockError> {
    if !crosses_exactly {
        return Ok(TimeUnits::ZERO);
    }

    remaining
        .checked_sub(distance)
        .ok_or(ClockError::PrecisionLoss)
}

fn plan_at_boundary(
    segments: Vec<Segment>,
    duration: f32,
    direction: bool,
    mode: PlaybackMode,
) -> TraversalPlan {
    let final_time = if mode == PlaybackMode::Loop {
        0.0
    } else {
        boundary_for(duration, direction)
    };
    let final_direction = if mode == PlaybackMode::Loop {
        direction
    } else {
        !direction
    };

    TraversalPlan {
        segments,
        final_time,
        final_forward: final_direction,
    }
}

fn next_segment_start(duration: f32, direction: bool, mode: PlaybackMode) -> (f32, bool) {
    if mode == PlaybackMode::Loop {
        (0.0, direction)
    } else {
        (boundary_for(duration, direction), !direction)
    }
}

fn boundary_distance(
    duration: TimeUnits,
    time: f32,
    forward: bool,
) -> Result<TimeUnits, ClockError> {
    let exact_time = TimeUnits::from_seconds(time);
    if forward {
        duration
            .checked_sub(exact_time)
            .ok_or(ClockError::PrecisionLoss)
    } else {
        Ok(exact_time)
    }
}

fn rounded_interior_endpoint(
    start: f32,
    forward: bool,
    remaining: TimeUnits,
) -> Result<f32, ClockError> {
    let exact_start = TimeUnits::from_seconds(start);
    let exact_endpoint = if forward {
        exact_start.checked_add(remaining)
    } else {
        exact_start.checked_sub(remaining)
    }
    .ok_or(ClockError::PrecisionLoss)?;

    exact_endpoint.rounded_seconds()
}
