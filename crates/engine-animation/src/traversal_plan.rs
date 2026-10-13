use super::super::event_validation::ensure_capacity;
use super::{ClockError, PlaybackMode, ScheduledEvent};

#[path = "traversal_plan/precision.rs"]
mod precision;
#[path = "traversal_plan/repeating.rs"]
mod repeating;

use repeating::{build_repeating, ensure_minimum_crossings, rounded_endpoint};

#[derive(Clone, Copy)]
struct Segment {
    start: f32,
    end: f32,
    forward: bool,
    loop_boundary_after: bool,
}

pub(super) struct TraversalPlan {
    segments: Vec<Segment>,
    final_time: f32,
    final_forward: bool,
}
impl TraversalPlan {
    pub(super) fn build(
        mode: PlaybackMode,
        duration: f32,
        time: f32,
        forward: bool,
        delta: f32,
        max_crossings: usize,
    ) -> Result<Self, ClockError> {
        if mode == PlaybackMode::Once {
            return Self::build_once(duration, time, delta);
        }
        if delta == 0.0 {
            return Ok(Self::stationary(time, forward));
        }

        ensure_minimum_crossings(duration, time, forward, delta, max_crossings)?;

        build_repeating(mode, duration, time, forward, delta, max_crossings)
    }

    pub(super) fn collect_events(
        &self,
        authored: &[ScheduledEvent],
        max_batch: usize,
    ) -> Result<Vec<ScheduledEvent>, ClockError> {
        let mut collected = Vec::new();

        for segment in &self.segments {
            let capacity = max_batch
                .checked_sub(collected.len())
                .ok_or(ClockError::EventOverflow)?;
            let segment_events = collect_segment(authored, *segment, capacity)?;
            collected.extend(segment_events);

            if segment.loop_boundary_after {
                let capacity = max_batch
                    .checked_sub(collected.len())
                    .ok_or(ClockError::EventOverflow)?;
                let boundary_events = authored.iter().filter(|event| event.time == 0.0);
                ensure_capacity(boundary_events.clone().count(), capacity)?;
                collected.extend(boundary_events.cloned());
            }
        }

        Ok(collected)
    }

    pub(super) fn final_time(&self) -> f32 {
        self.final_time
    }
    pub(super) fn final_direction(&self) -> bool {
        self.final_forward
    }
    fn stationary(time: f32, forward: bool) -> Self {
        Self {
            segments: Vec::new(),
            final_time: time,
            final_forward: forward,
        }
    }

    fn build_once(duration: f32, time: f32, delta: f32) -> Result<Self, ClockError> {
        let endpoint = rounded_endpoint(time, true, f64::from(delta))?;
        let endpoint = endpoint.min(duration);
        if endpoint == time {
            return Ok(Self::stationary(time, true));
        }

        Ok(Self {
            segments: vec![Segment {
                start: time,
                end: endpoint,
                forward: true,
                loop_boundary_after: false,
            }],
            final_time: endpoint,
            final_forward: true,
        })
    }
}

fn collect_segment(
    authored: &[ScheduledEvent],
    segment: Segment,
    capacity: usize,
) -> Result<Vec<ScheduledEvent>, ClockError> {
    if segment.forward {
        let matching = authored
            .iter()
            .filter(|event| event.time > segment.start && event.time <= segment.end);
        ensure_capacity(matching.clone().count(), capacity)?;
        return Ok(matching.cloned().collect());
    }

    let matching = authored
        .iter()
        .filter(|event| event.time < segment.start && event.time >= segment.end);
    ensure_capacity(matching.clone().count(), capacity)?;
    let mut reverse = matching.cloned().collect::<Vec<_>>();
    reverse.sort_by(|left, right| {
        if left.time == right.time {
            std::cmp::Ordering::Equal
        } else {
            right.time.total_cmp(&left.time)
        }
    });
    Ok(reverse)
}

fn boundary_for(duration: f32, forward: bool) -> f32 {
    if forward { duration } else { 0.0 }
}

fn reaches_boundary(endpoint: f32, duration: f32, forward: bool) -> bool {
    if forward {
        endpoint >= duration
    } else {
        endpoint <= 0.0
    }
}
