//! Validated directed navigation graph primitives.
//!
//! This candidate implements the lane 9 OpenUnity design contract. It does
//! not assign persistent identities to graph-local node indices.

use std::collections::VecDeque;

use engine_math::{Vector, finite};

/// A zero-based index within one navigation graph.
pub type NodeIndex = usize;

/// A graph node with a finite world-space position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavigationNode {
    /// Finite world-space coordinate used by path queries.
    pub position: Vector,
}

/// A directed edge between two graph-local nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavigationEdge {
    /// Source node index within the graph.
    pub from: NodeIndex,
    /// Destination node index within the graph.
    pub to: NodeIndex,
    /// Finite non-negative traversal cost.
    pub cost: f32,
}

/// Failure reported while validating or querying a navigation graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphError {
    /// A graph must contain at least one node.
    Empty,
    /// A node coordinate contains a non-finite component.
    NonFinitePosition,
    /// An edge or query references an index outside the graph.
    InvalidNode,
    /// An edge cost is not finite.
    NonFiniteCost,
    /// An edge cost is negative.
    NegativeCost,
    /// The same directed edge occurs more than once.
    DuplicateEdge,
    /// An edge starts and ends at the same node.
    SelfEdge,
    /// A graph allocation or checked size calculation overflowed.
    CapacityOverflow,
}

/// An immutable, validated directed graph.
#[derive(Clone, Debug, PartialEq)]
pub struct NavigationGraph {
    nodes: Vec<NavigationNode>,
    adjacency: Vec<Vec<NavigationEdge>>,
}

impl NavigationGraph {
    /// Validate and construct a graph from graph-local nodes and edges.
    pub fn build(
        nodes: Vec<NavigationNode>,
        edges: Vec<NavigationEdge>,
    ) -> Result<Self, GraphError> {
        validate_graph_inputs(&nodes, &edges)?;

        let mut adjacency = empty_adjacency(nodes.len(), &edges)?;

        for edge in edges {
            adjacency[edge.from].push(edge);
        }

        for outgoing in &mut adjacency {
            outgoing.sort_unstable_by_key(|edge| edge.to);
        }

        Ok(Self { nodes, adjacency })
    }

    /// Return the number of nodes in this graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return a node position after validating its graph-local index.
    pub fn position(&self, node: NodeIndex) -> Result<Vector, GraphError> {
        self.nodes
            .get(node)
            .map(|entry| entry.position)
            .ok_or(GraphError::InvalidNode)
    }

    /// Return outgoing edges in ascending destination-index order.
    pub fn neighbors(&self, node: NodeIndex) -> Result<&[NavigationEdge], GraphError> {
        self.adjacency
            .get(node)
            .map(Vec::as_slice)
            .ok_or(GraphError::InvalidNode)
    }

    /// Determine whether a directed route exists between two valid nodes.
    pub fn connected(&self, from: NodeIndex, to: NodeIndex) -> Result<bool, GraphError> {
        self.validate_node(from)?;
        self.validate_node(to)?;

        if from == to {
            return Ok(true);
        }

        let mut visited = Vec::new();
        visited
            .try_reserve_exact(self.nodes.len())
            .map_err(|_| GraphError::CapacityOverflow)?;
        visited.resize(self.nodes.len(), false);

        let mut pending = VecDeque::new();
        pending
            .try_reserve_exact(self.nodes.len())
            .map_err(|_| GraphError::CapacityOverflow)?;

        pending.push_back(from);
        visited[from] = true;

        while let Some(current) = pending.pop_front() {
            for edge in &self.adjacency[current] {
                if edge.to == to {
                    return Ok(true);
                }

                if !visited[edge.to] {
                    visited[edge.to] = true;
                    pending.push_back(edge.to);
                }
            }
        }

        Ok(false)
    }

    fn validate_node(&self, node: NodeIndex) -> Result<(), GraphError> {
        self.nodes
            .get(node)
            .map(|_| ())
            .ok_or(GraphError::InvalidNode)
    }
}

fn validate_graph_inputs(
    nodes: &[NavigationNode],
    edges: &[NavigationEdge],
) -> Result<(), GraphError> {
    if nodes.is_empty() {
        return Err(GraphError::Empty);
    }

    nodes
        .iter()
        .try_for_each(|node| finite(&node.position).map_err(|_| GraphError::NonFinitePosition))?;

    nodes
        .len()
        .checked_add(edges.len())
        .ok_or(GraphError::CapacityOverflow)?;

    let mut pairs = Vec::new();
    pairs
        .try_reserve_exact(edges.len())
        .map_err(|_| GraphError::CapacityOverflow)?;

    for edge in edges {
        if edge.from >= nodes.len() || edge.to >= nodes.len() {
            return Err(GraphError::InvalidNode);
        }
        if edge.from == edge.to {
            return Err(GraphError::SelfEdge);
        }
        if !edge.cost.is_finite() {
            return Err(GraphError::NonFiniteCost);
        }
        if edge.cost < 0.0 {
            return Err(GraphError::NegativeCost);
        }

        pairs.push((edge.from, edge.to));
    }

    pairs.sort_unstable();
    if pairs.windows(2).any(|window| window[0] == window[1]) {
        return Err(GraphError::DuplicateEdge);
    }

    Ok(())
}

fn empty_adjacency(
    node_count: usize,
    edges: &[NavigationEdge],
) -> Result<Vec<Vec<NavigationEdge>>, GraphError> {
    let mut adjacency = Vec::new();
    adjacency
        .try_reserve_exact(node_count)
        .map_err(|_| GraphError::CapacityOverflow)?;
    adjacency.resize_with(node_count, Vec::new);

    let mut edge_counts: Vec<usize> = Vec::new();
    edge_counts
        .try_reserve_exact(node_count)
        .map_err(|_| GraphError::CapacityOverflow)?;
    edge_counts.resize(node_count, 0usize);
    for edge in edges {
        edge_counts[edge.from] = edge_counts[edge.from]
            .checked_add(1)
            .ok_or(GraphError::CapacityOverflow)?;
    }

    for (outgoing, count) in adjacency.iter_mut().zip(edge_counts) {
        outgoing
            .try_reserve_exact(count)
            .map_err(|_| GraphError::CapacityOverflow)?;
    }

    Ok(adjacency)
}
