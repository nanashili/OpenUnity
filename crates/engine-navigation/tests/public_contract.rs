//! Public navigation conformance candidates for the approved lane 9 contract.

use engine_navigation::{
    FollowError, GraphError, NavigationEdge, NavigationGraph, NavigationNode, PathError,
    follow_path, shortest_path,
};

fn nodes(count: usize) -> Vec<NavigationNode> {
    (0..count)
        .map(|index| NavigationNode {
            position: [index as f32, 0.0, 0.0],
        })
        .collect()
}

fn edge(from: usize, to: usize, cost: f32) -> NavigationEdge {
    NavigationEdge { from, to, cost }
}

#[test]
fn graph_validation_is_directed_and_neighbors_are_deterministic() {
    let graph = NavigationGraph::build(
        nodes(3),
        vec![edge(0, 2, 2.0), edge(0, 1, 1.0), edge(1, 2, 1.0)],
    )
    .unwrap();
    assert_eq!(graph.node_count(), 3);
    assert_eq!(
        graph
            .neighbors(0)
            .unwrap()
            .iter()
            .map(|item| item.to)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(graph.connected(0, 2), Ok(true));
    assert_eq!(graph.connected(2, 0), Ok(false));
    assert_eq!(graph.connected(1, 1), Ok(true));
    assert_eq!(graph.position(3), Err(GraphError::InvalidNode));
    assert_eq!(graph.connected(3, 0), Err(GraphError::InvalidNode));
}

#[test]
fn graph_rejects_invalid_edges_positions_and_duplicate_pairs() {
    assert_eq!(
        NavigationGraph::build(Vec::new(), Vec::new()),
        Err(GraphError::Empty)
    );
    assert_eq!(
        NavigationGraph::build(
            vec![NavigationNode {
                position: [f32::NAN, 0.0, 0.0]
            }],
            Vec::new()
        ),
        Err(GraphError::NonFinitePosition)
    );
    assert_eq!(
        NavigationGraph::build(nodes(2), vec![edge(0, 2, 1.0)]),
        Err(GraphError::InvalidNode)
    );
    assert_eq!(
        NavigationGraph::build(nodes(2), vec![edge(0, 0, 1.0)]),
        Err(GraphError::SelfEdge)
    );
    assert_eq!(
        NavigationGraph::build(nodes(2), vec![edge(0, 1, 1.0), edge(0, 1, 2.0)]),
        Err(GraphError::DuplicateEdge)
    );
    assert_eq!(
        NavigationGraph::build(nodes(2), vec![edge(0, 1, f32::NAN)]),
        Err(GraphError::NonFiniteCost)
    );
    assert_eq!(
        NavigationGraph::build(nodes(2), vec![edge(0, 1, -1.0)]),
        Err(GraphError::NegativeCost)
    );
}

#[test]
fn shortest_path_selects_lexicographic_tie_and_reports_failures() {
    let graph = NavigationGraph::build(
        nodes(4),
        vec![
            edge(0, 2, 1.0),
            edge(0, 1, 1.0),
            edge(1, 3, 1.0),
            edge(2, 3, 1.0),
        ],
    )
    .unwrap();
    let path = shortest_path(&graph, 0, 3).unwrap();
    assert_eq!(path.nodes, vec![0, 1, 3]);
    assert_eq!(path.total_cost, 2.0);
    assert_eq!(shortest_path(&graph, 2, 2).unwrap().nodes, vec![2]);
    assert_eq!(shortest_path(&graph, 3, 0), Err(PathError::Unreachable));
    assert_eq!(shortest_path(&graph, 4, 0), Err(PathError::InvalidNode));

    let overflowing =
        NavigationGraph::build(nodes(3), vec![edge(0, 1, f32::MAX), edge(1, 2, f32::MAX)]).unwrap();
    assert_eq!(
        shortest_path(&overflowing, 0, 2),
        Err(PathError::CostOverflow)
    );
    assert_eq!(overflowing.node_count(), 3);
    assert_eq!(overflowing.neighbors(0).unwrap()[0].to, 1);

    let discarded_exploration = NavigationGraph::build(
        nodes(4),
        vec![
            edge(0, 1, f32::MAX),
            edge(1, 3, f32::MAX),
            edge(0, 2, 0.5),
            edge(2, 3, 0.5),
        ],
    )
    .unwrap();
    let lower_route = shortest_path(&discarded_exploration, 0, 3).unwrap();
    assert_eq!(lower_route.nodes, vec![0, 2, 3]);
    assert_eq!(lower_route.total_cost, 1.0);
}

#[test]
fn follow_path_is_bounded_and_rejects_off_path_or_discontinuous_inputs() {
    let graph = NavigationGraph::build(nodes(3), vec![edge(0, 1, 1.0), edge(1, 2, 1.0)]).unwrap();
    let partial = follow_path(&graph, &[0, 1, 2], [0.25, 0.0, 0.0], 0.5).unwrap();
    assert_eq!(partial.position, [0.75, 0.0, 0.0]);
    assert_eq!(partial.next_node, Some(1));
    assert_eq!(partial.consumed_distance, 0.5);
    assert!(!partial.reached_goal);

    let goal = follow_path(&graph, &[0, 1, 2], [0.25, 0.0, 0.0], 2.0).unwrap();
    assert_eq!(goal.position, [2.0, 0.0, 0.0]);
    assert_eq!(goal.next_node, None);
    assert!(goal.reached_goal);
    assert_eq!(
        follow_path(&graph, &[0, 1], [0.0, 1.0, 0.0], 0.5),
        Err(FollowError::OffPath)
    );
    assert_eq!(
        follow_path(&graph, &[1, 0], [1.0, 0.0, 0.0], 0.5),
        Err(FollowError::PathDiscontinuous)
    );
    assert_eq!(
        follow_path(&graph, &[1], [0.0, 0.0, 0.0], 0.0),
        Err(FollowError::OffPath)
    );
    assert_eq!(
        follow_path(&graph, &[0, 1], [0.0, 0.0, 0.0], -0.1),
        Err(FollowError::InvalidStep)
    );
    assert_eq!(
        follow_path(&graph, &[], [0.0; 3], 0.0),
        Err(FollowError::EmptyPath)
    );
    assert_eq!(
        follow_path(&graph, &[0, 3], [0.0, 0.0, 0.0], 0.0),
        Err(FollowError::InvalidNode)
    );
    assert_eq!(
        follow_path(&graph, &[0, 1], [f32::NAN, 0.0, 0.0], 0.0),
        Err(FollowError::NonFiniteCoordinate)
    );
    assert_eq!(
        follow_path(&graph, &[0, 1], [0.0, 0.0, 0.0], f32::INFINITY),
        Err(FollowError::NonFiniteDistance)
    );

    let extreme_graph = NavigationGraph::build(
        vec![
            NavigationNode {
                position: [0.0, 0.0, 0.0],
            },
            NavigationNode {
                position: [f32::MAX, 0.0, 0.0],
            },
        ],
        vec![edge(0, 1, f32::MAX)],
    )
    .unwrap();
    let extreme = follow_path(&extreme_graph, &[0, 1], [0.0; 3], f32::MAX).unwrap();
    assert_eq!(extreme.position, [f32::MAX, 0.0, 0.0]);
    assert_eq!(extreme.consumed_distance, f32::MAX);
    assert!(extreme.reached_goal);
}

#[test]
fn follow_path_consumes_zero_length_segments_without_extrapolation() {
    let graph = NavigationGraph::build(
        vec![
            NavigationNode {
                position: [0.0, 0.0, 0.0],
            },
            NavigationNode {
                position: [0.0, 0.0, 0.0],
            },
            NavigationNode {
                position: [2.0, 0.0, 0.0],
            },
        ],
        vec![edge(0, 1, 0.0), edge(1, 2, 2.0)],
    )
    .unwrap();
    let result = follow_path(&graph, &[0, 1, 2], [0.0; 3], 1.0).unwrap();
    assert_eq!(result.position, [1.0, 0.0, 0.0]);
    assert_eq!(result.consumed_distance, 1.0);
    assert_eq!(result.next_node, Some(2));
    assert!(!result.reached_goal);
}
