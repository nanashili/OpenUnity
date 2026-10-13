//! Deterministic shortest paths over validated directed graphs.

use super::{NavigationGraph, NodeIndex};

#[path = "navigation_search.rs"]
mod search;

/// Failure returned when a shortest path cannot be represented or found.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathError {
    /// The start or goal index is outside the graph.
    InvalidNode,
    /// No directed route connects the start and goal.
    Unreachable,
    /// An explored edge has negative cost.
    NegativeCost,
    /// An explored edge has non-finite cost.
    NonFiniteCost,
    /// The selected route cost cannot be represented as a finite `f32`.
    CostOverflow,
    /// A bounded allocation or checked size calculation failed.
    CapacityOverflow,
}

/// Ordered graph-local nodes and the cost of their directed route.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    /// Node indices from the start through the goal, inclusive.
    pub nodes: Vec<NodeIndex>,
    /// Sum of the selected route's edge costs.
    pub total_cost: f32,
}

/// Find a minimum-cost directed path, breaking equal-cost ties by node sequence.
pub fn shortest_path(
    graph: &NavigationGraph,
    start: NodeIndex,
    goal: NodeIndex,
) -> Result<Path, PathError> {
    search::shortest_path(graph, start, goal)
}
