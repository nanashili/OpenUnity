//! Validate and project the caller's initial position onto the path.

use engine_math::Vector;

use super::super::{NavigationGraph, NodeIndex};
use super::collinearity::is_exactly_collinear;
use super::geometry::{difference, dot, segment_length};
use super::traversal::FollowProgress;
use super::{FollowError, node_position};

pub(super) fn initial_progress(
    graph: &NavigationGraph,
    path: &[NodeIndex],
    position: Vector,
) -> Result<FollowProgress, FollowError> {
    let start = node_position(graph, path[0])?;
    let end = node_position(graph, path[1])?;
    let length = segment_length(start, end);

    if length == 0.0 {
        if position != start {
            return Err(FollowError::OffPath);
        }

        return Ok(FollowProgress::at_position(0, 0.0, position));
    }

    let parameter = validate_initial_position(position, start, end)?;
    if position == end {
        return Ok(FollowProgress::at_position(1, 0.0, position));
    }

    let offset = length * parameter;
    if offset >= length {
        return Err(FollowError::PrecisionLoss);
    }

    Ok(FollowProgress::at_position(0, offset, position))
}

pub(super) fn zero_distance_progress(
    graph: &NavigationGraph,
    path: &[NodeIndex],
    position: Vector,
) -> Result<FollowProgress, FollowError> {
    let start = node_position(graph, path[0])?;
    let end = node_position(graph, path[1])?;

    if segment_length(start, end) == 0.0 {
        if position != start {
            return Err(FollowError::OffPath);
        }

        return Ok(FollowProgress::at_position(0, 0.0, position));
    }

    validate_initial_position(position, start, end)?;

    let segment_ordinal = if position == end { 1 } else { 0 };
    Ok(FollowProgress::at_position(segment_ordinal, 0.0, position))
}

fn validate_initial_position(
    position: Vector,
    start: Vector,
    end: Vector,
) -> Result<f64, FollowError> {
    validate_component_bounds(position, start, end)?;

    let delta = difference(end, start);
    let offset = difference(position, start);
    let length_squared = dot(delta, delta);
    let parameter = dot(offset, delta) / length_squared;

    if !(0.0..=1.0).contains(&parameter) || !is_exactly_collinear(position, start, end) {
        return Err(FollowError::OffPath);
    }

    Ok(parameter)
}

fn validate_component_bounds(
    position: Vector,
    start: Vector,
    end: Vector,
) -> Result<(), FollowError> {
    // Projection can round an outside subnormal offset onto an endpoint when
    // the segment spans an extreme coordinate range, so bound-check first.
    for axis in 0..position.len() {
        let lower_bound = start[axis].min(end[axis]);
        let upper_bound = start[axis].max(end[axis]);

        if position[axis] < lower_bound || position[axis] > upper_bound {
            return Err(FollowError::OffPath);
        }
    }

    Ok(())
}
