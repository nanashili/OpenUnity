//! Shared segment traversal for stateless and owned path following.

use engine_math::Vector;

use super::super::{NavigationGraph, NodeIndex};
use super::{FollowError, FollowResult, geometry, node_position};

#[derive(Clone, Copy, Debug)]
pub(super) struct FollowProgress {
    pub(super) segment_ordinal: usize,
    pub(super) offset: f64,
    pub(super) position: Vector,
}

impl FollowProgress {
    pub(super) fn at_position(segment_ordinal: usize, offset: f64, position: Vector) -> Self {
        Self {
            segment_ordinal,
            offset,
            position,
        }
    }
}

pub(super) fn advance_from_progress(
    graph: &NavigationGraph,
    path: &[NodeIndex],
    initial: FollowProgress,
    max_distance: f64,
) -> Result<(FollowProgress, FollowResult), FollowError> {
    let mut progress = initial;
    let mut remaining = max_distance;
    let mut consumed = 0.0_f64;

    while progress.segment_ordinal < path.len() - 1 {
        let (start, end) = segment_endpoints(graph, path, progress.segment_ordinal)?;
        let length = geometry::segment_length(start, end);

        if length == 0.0 {
            progress.segment_ordinal += 1;
            progress.offset = 0.0;
            continue;
        }

        let distance_to_end = length - progress.offset;
        if remaining >= distance_to_end {
            consume_complete_segment(
                &mut progress,
                end,
                distance_to_end,
                &mut remaining,
                &mut consumed,
            )?;
            continue;
        }

        if remaining > 0.0 {
            consume_partial_segment(&mut progress, start, end, length, remaining, &mut consumed)?;
        }

        return Ok((progress, follow_result(path, progress, consumed)?));
    }

    Ok((progress, follow_result(path, progress, consumed)?))
}

fn segment_endpoints(
    graph: &NavigationGraph,
    path: &[NodeIndex],
    segment_ordinal: usize,
) -> Result<(Vector, Vector), FollowError> {
    let start = node_position(graph, path[segment_ordinal])?;
    let end = node_position(graph, path[segment_ordinal + 1])?;
    Ok((start, end))
}

fn consume_complete_segment(
    progress: &mut FollowProgress,
    end: Vector,
    distance_to_end: f64,
    remaining: &mut f64,
    consumed: &mut f64,
) -> Result<(), FollowError> {
    let next_remaining = *remaining - distance_to_end;
    if distance_to_end > 0.0 && next_remaining > 0.0 && next_remaining == *remaining {
        return Err(FollowError::PrecisionLoss);
    }

    let next_consumed = *consumed + distance_to_end;
    if !next_consumed.is_finite() || next_consumed > f64::from(f32::MAX) {
        return Err(FollowError::StepOverflow);
    }

    *remaining = next_remaining;
    *consumed = next_consumed;
    progress.segment_ordinal += 1;
    progress.offset = 0.0;
    progress.position = end;
    Ok(())
}

fn consume_partial_segment(
    progress: &mut FollowProgress,
    start: Vector,
    end: Vector,
    length: f64,
    step: f64,
    consumed: &mut f64,
) -> Result<(), FollowError> {
    let next_offset = progress.offset + step;
    if next_offset <= progress.offset || next_offset >= length {
        return Err(FollowError::PrecisionLoss);
    }

    let position = geometry::interpolate(start, end, next_offset / length)?;
    let next_consumed = *consumed + step;
    if !next_consumed.is_finite() || next_consumed > f64::from(f32::MAX) {
        return Err(FollowError::StepOverflow);
    }

    progress.offset = next_offset;
    progress.position = position;
    *consumed = next_consumed;
    Ok(())
}

fn follow_result(
    path: &[NodeIndex],
    progress: FollowProgress,
    consumed: f64,
) -> Result<FollowResult, FollowError> {
    let complete = progress.segment_ordinal == path.len() - 1;
    let next_node = if complete {
        None
    } else {
        Some(path[progress.segment_ordinal + 1])
    };

    Ok(FollowResult {
        position: progress.position,
        next_node,
        consumed_distance: geometry::checked_binary32(consumed)?,
        reached_goal: complete,
    })
}
