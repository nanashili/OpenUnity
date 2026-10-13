//! Stateless bounded following over a validated directed path.

use engine_math::Vector;

use super::{GraphError, NavigationGraph, NodeIndex};

#[path = "navigation_following_admission.rs"]
mod admission;
#[path = "navigation_following_collinearity.rs"]
mod collinearity;
#[path = "navigation_following_geometry.rs"]
mod geometry;
#[path = "navigation_path_follower.rs"]
mod path_follower;
#[path = "navigation_following_traversal.rs"]
mod traversal;

use admission::{initial_progress, zero_distance_progress};
use traversal::advance_from_progress;

pub use path_follower::{PathFollower, PreparedFollow};

/// Errors raised while validating or consuming a bounded path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FollowError {
    /// A path contains a node index outside the graph.
    InvalidNode,
    /// A path contains no nodes.
    EmptyPath,
    /// A path has a non-finite or negative advisory cost.
    InvalidPathCost,
    /// Consecutive path nodes do not share a directed edge.
    PathDiscontinuous,
    /// An input position or graph coordinate is not finite.
    NonFiniteCoordinate,
    /// The requested movement distance is not finite.
    NonFiniteDistance,
    /// The initial position is not on the path's first segment.
    OffPath,
    /// The maximum movement distance is negative.
    InvalidStep,
    /// A calculated position or consumed distance exceeds finite `f32` range.
    StepOverflow,
    /// Binary64 precision cannot represent the requested positive progress.
    PrecisionLoss,
}

/// The result of consuming at most one bounded step along a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FollowResult {
    /// Position after consuming the bounded movement distance.
    pub position: Vector,
    /// Next path node that remains ahead, if one remains.
    pub next_node: Option<NodeIndex>,
    /// Distance actually traveled during this call.
    pub consumed_distance: f32,
    /// Whether traversal reached the final node.
    pub reached_goal: bool,
}

/// Follow a directed path by at most `max_distance`, without extrapolation.
///
/// The supplied path must be continuous in the graph, and `position` must lie
/// exactly on its first segment. A rounded partial result is not guaranteed to
/// be suitable as the starting position of another stateless call; use
/// [`PathFollower`] for resumable movement.
///
/// # Errors
///
/// Returns [`FollowError`] when the path, position, or movement bound is
/// invalid, or when the resulting values do not fit finite `f32` coordinates.
pub fn follow_path(
    graph: &NavigationGraph,
    path: &[NodeIndex],
    position: Vector,
    max_distance: f32,
) -> Result<FollowResult, FollowError> {
    validate_input(position, max_distance)?;

    if path.is_empty() {
        return Err(FollowError::EmptyPath);
    }

    validate_path(graph, path)?;

    if path.len() == 1 {
        return completed_single_node(graph, path[0], position);
    }

    if max_distance == 0.0 {
        let progress = zero_distance_progress(graph, path, position)?;
        let (_, result) = advance_from_progress(graph, path, progress, 0.0)?;
        return Ok(result);
    }

    let progress = initial_progress(graph, path, position)?;
    let (_, result) = advance_from_progress(graph, path, progress, f64::from(max_distance))?;
    Ok(result)
}

pub(super) fn validate_input(position: Vector, max_distance: f32) -> Result<(), FollowError> {
    if position.iter().any(|component| !component.is_finite()) {
        return Err(FollowError::NonFiniteCoordinate);
    }

    validate_distance(max_distance)
}

pub(super) fn validate_distance(max_distance: f32) -> Result<(), FollowError> {
    if !max_distance.is_finite() {
        return Err(FollowError::NonFiniteDistance);
    }

    if max_distance < 0.0 {
        return Err(FollowError::InvalidStep);
    }

    Ok(())
}

pub(super) fn validate_path(
    graph: &NavigationGraph,
    path: &[NodeIndex],
) -> Result<(), FollowError> {
    validate_path_nodes(graph, path)?;
    validate_path_edges(graph, path)
}

pub(super) fn node_position(
    graph: &NavigationGraph,
    node: NodeIndex,
) -> Result<Vector, FollowError> {
    graph.position(node).map_err(map_graph_error)
}

fn completed_single_node(
    graph: &NavigationGraph,
    node: NodeIndex,
    position: Vector,
) -> Result<FollowResult, FollowError> {
    let goal = node_position(graph, node)?;
    if position != goal {
        return Err(FollowError::OffPath);
    }

    Ok(FollowResult {
        position,
        next_node: None,
        consumed_distance: 0.0,
        reached_goal: true,
    })
}

fn validate_path_nodes(graph: &NavigationGraph, path: &[NodeIndex]) -> Result<(), FollowError> {
    for &node in path {
        let position = node_position(graph, node)?;
        if position.iter().any(|component| !component.is_finite()) {
            return Err(FollowError::NonFiniteCoordinate);
        }
    }

    Ok(())
}

fn validate_path_edges(graph: &NavigationGraph, path: &[NodeIndex]) -> Result<(), FollowError> {
    for window in path.windows(2) {
        let has_edge = graph
            .neighbors(window[0])
            .map_err(map_graph_error)?
            .iter()
            .any(|edge| edge.to == window[1]);
        if !has_edge {
            return Err(FollowError::PathDiscontinuous);
        }
    }

    Ok(())
}

fn map_graph_error(_error: GraphError) -> FollowError {
    FollowError::InvalidNode
}
