use super::super::graph::{GraphError, NavigationGraph, NodeIndex};
use super::{Path, PathError};
use engine_math::finite;

pub(super) fn shortest_path(
    graph: &NavigationGraph,
    start: NodeIndex,
    goal: NodeIndex,
) -> Result<Path, PathError> {
    validate_graph(graph)?;
    validate_endpoints(graph, start, goal)?;

    if start == goal {
        return single_node_path(start);
    }

    let mut labels = empty_labels(graph.node_count())?;
    labels[start] = Some(Label::start(start)?);

    let mut settled = empty_settled(graph.node_count())?;
    while let Some(current) = choose_next(&labels, &settled) {
        settled[current] = true;

        if current == goal {
            return labels[current]
                .take()
                .map(Label::into_path)
                .transpose()?
                .ok_or(PathError::Unreachable);
        }

        relax_neighbors(graph, current, &mut labels, &settled)?;
    }

    Err(PathError::Unreachable)
}

#[derive(Debug)]
struct Label {
    cost: f64,
    nodes: Vec<NodeIndex>,
}

impl Label {
    fn start(start: NodeIndex) -> Result<Self, PathError> {
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(1)
            .map_err(|_| PathError::CapacityOverflow)?;
        nodes.push(start);
        Ok(Self { cost: 0.0, nodes })
    }

    fn into_path(self) -> Result<Path, PathError> {
        let total_cost = finite_binary32(self.cost)?;
        Ok(Path {
            nodes: self.nodes,
            total_cost,
        })
    }
}

fn validate_graph(graph: &NavigationGraph) -> Result<(), PathError> {
    if graph.node_count() == 0 {
        return Err(PathError::InvalidNode);
    }

    for node in 0..graph.node_count() {
        let position = graph.position(node).map_err(map_graph_error)?;
        finite(&position).map_err(|_| PathError::InvalidNode)?;

        let mut previous_destination = None;
        for edge in graph.neighbors(node).map_err(map_graph_error)? {
            validate_cost(edge.cost)?;
            if edge.to == node {
                return Err(PathError::InvalidNode);
            }
            if edge.to >= graph.node_count() {
                return Err(PathError::InvalidNode);
            }
            if previous_destination == Some(edge.to) {
                return Err(PathError::InvalidNode);
            }
            previous_destination = Some(edge.to);
        }
    }

    Ok(())
}

fn validate_endpoints(
    graph: &NavigationGraph,
    start: NodeIndex,
    goal: NodeIndex,
) -> Result<(), PathError> {
    graph.position(start).map_err(map_graph_error)?;
    graph.position(goal).map_err(map_graph_error)?;
    Ok(())
}

fn empty_labels(node_count: usize) -> Result<Vec<Option<Label>>, PathError> {
    let mut labels = Vec::new();
    labels
        .try_reserve_exact(node_count)
        .map_err(|_| PathError::CapacityOverflow)?;
    labels.resize_with(node_count, || None);
    Ok(labels)
}

fn empty_settled(node_count: usize) -> Result<Vec<bool>, PathError> {
    let mut settled = Vec::new();
    settled
        .try_reserve_exact(node_count)
        .map_err(|_| PathError::CapacityOverflow)?;
    settled.resize(node_count, false);
    Ok(settled)
}

fn single_node_path(start: NodeIndex) -> Result<Path, PathError> {
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(1)
        .map_err(|_| PathError::CapacityOverflow)?;
    nodes.push(start);

    Ok(Path {
        nodes,
        total_cost: 0.0,
    })
}

fn choose_next(labels: &[Option<Label>], settled: &[bool]) -> Option<NodeIndex> {
    labels
        .iter()
        .enumerate()
        .filter(|(node, _)| !settled[*node])
        .filter_map(|(node, label)| label.as_ref().map(|label| (node, label)))
        .min_by(|(left_node, left), (right_node, right)| {
            compare_labels(left, right).then_with(|| left_node.cmp(right_node))
        })
        .map(|(node, _)| node)
}

fn compare_labels(left: &Label, right: &Label) -> std::cmp::Ordering {
    left.cost
        .total_cmp(&right.cost)
        .then_with(|| left.nodes.cmp(&right.nodes))
}

fn relax_neighbors(
    graph: &NavigationGraph,
    current: NodeIndex,
    labels: &mut [Option<Label>],
    settled: &[bool],
) -> Result<(), PathError> {
    for edge in graph.neighbors(current).map_err(map_graph_error)? {
        validate_cost(edge.cost)?;
        let current_label = labels[current].as_ref().ok_or(PathError::Unreachable)?;
        if settled[edge.to] || current_label.nodes.contains(&edge.to) {
            continue;
        }

        let candidate = extend_label(current_label, edge.to, edge.cost)?;
        if should_replace(labels[edge.to].as_ref(), &candidate) {
            labels[edge.to] = Some(candidate);
        }
    }

    Ok(())
}

fn extend_label(label: &Label, node: NodeIndex, edge_cost: f32) -> Result<Label, PathError> {
    let cost = label.cost + f64::from(edge_cost);
    if !cost.is_finite() {
        return Err(PathError::CostOverflow);
    }

    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(
            label
                .nodes
                .len()
                .checked_add(1)
                .ok_or(PathError::CapacityOverflow)?,
        )
        .map_err(|_| PathError::CapacityOverflow)?;
    nodes.extend_from_slice(&label.nodes);
    nodes.push(node);

    Ok(Label { cost, nodes })
}

fn should_replace(existing: Option<&Label>, candidate: &Label) -> bool {
    existing
        .map(|label| compare_labels(candidate, label).is_lt())
        .unwrap_or(true)
}

fn validate_cost(cost: f32) -> Result<(), PathError> {
    if !cost.is_finite() {
        return Err(PathError::NonFiniteCost);
    }
    if cost < 0.0 {
        return Err(PathError::NegativeCost);
    }
    Ok(())
}

fn finite_binary32(value: f64) -> Result<f32, PathError> {
    if !value.is_finite() {
        return Err(PathError::CostOverflow);
    }

    let narrowed = value as f32;
    if narrowed.is_finite() {
        Ok(narrowed)
    } else {
        Err(PathError::CostOverflow)
    }
}

fn map_graph_error(error: GraphError) -> PathError {
    match error {
        GraphError::NonFiniteCost => PathError::NonFiniteCost,
        GraphError::NegativeCost => PathError::NegativeCost,
        GraphError::CapacityOverflow => PathError::CapacityOverflow,
        GraphError::InvalidNode
        | GraphError::Empty
        | GraphError::NonFinitePosition
        | GraphError::DuplicateEdge
        | GraphError::SelfEdge => PathError::InvalidNode,
    }
}
