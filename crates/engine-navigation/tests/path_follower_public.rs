//! Public conformance for owned path progress and stateless boundaries.

use engine_navigation::{
    FollowError, NavigationEdge, NavigationGraph, NavigationNode, Path, PathFollower, follow_path,
};

fn graph(points: &[[f32; 3]], edges: &[(usize, usize)]) -> NavigationGraph {
    let nodes = points
        .iter()
        .copied()
        .map(|position| NavigationNode { position })
        .collect();
    let edges = edges
        .iter()
        .map(|(from, to)| NavigationEdge {
            from: *from,
            to: *to,
            cost: 1.0,
        })
        .collect();
    NavigationGraph::build(nodes, edges).unwrap()
}

fn path(nodes: &[usize]) -> Path {
    Path {
        nodes: nodes.to_vec(),
        total_cost: nodes.len().saturating_sub(1) as f32,
    }
}

#[test]
fn follower_constructor_has_stable_validation_precedence() {
    let graph = graph(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], &[(0, 1)]);

    assert_eq!(
        PathFollower::new(
            &graph,
            Path {
                nodes: Vec::new(),
                total_cost: f32::NAN,
            },
        )
        .err(),
        Some(FollowError::EmptyPath)
    );
    assert_eq!(
        PathFollower::new(
            &graph,
            Path {
                nodes: vec![usize::MAX],
                total_cost: f32::NAN,
            },
        )
        .err(),
        Some(FollowError::InvalidPathCost)
    );
    assert_eq!(
        PathFollower::new(
            &graph,
            Path {
                nodes: vec![usize::MAX],
                total_cost: 0.0
            }
        )
        .err(),
        Some(FollowError::InvalidNode)
    );
    assert_eq!(
        PathFollower::new(&graph, path(&[1, 0])).err(),
        Some(FollowError::PathDiscontinuous)
    );
    assert_eq!(
        PathFollower::new(
            &graph,
            Path {
                nodes: vec![0],
                total_cost: -1.0,
            },
        )
        .err(),
        Some(FollowError::InvalidPathCost)
    );
}

#[test]
fn one_node_follower_is_completed_and_idempotent() {
    let graph = graph(&[[3.0, -2.0, 1.0]], &[]);
    let mut follower = PathFollower::new(&graph, path(&[0])).unwrap();

    assert!(follower.is_complete());
    assert_eq!(follower.expected_position(), [3.0, -2.0, 1.0]);

    let first = follower.prepare_advance(4.0).unwrap();
    assert_eq!(first.result().position, [3.0, -2.0, 1.0]);
    assert_eq!(first.result().next_node, None);
    assert_eq!(first.result().consumed_distance, 0.0);
    assert!(first.result().reached_goal);
    first.commit();

    let second = follower.prepare_advance(0.0).unwrap();
    assert_eq!(second.result(), follower_result([3.0, -2.0, 1.0]));
}

fn follower_result(position: [f32; 3]) -> engine_navigation::FollowResult {
    engine_navigation::FollowResult {
        position,
        next_node: None,
        consumed_distance: 0.0,
        reached_goal: true,
    }
}

#[test]
fn dropping_preparation_aborts_progress_and_exact_boundaries_skip_zero_segments() {
    let zero_graph = graph(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
        ],
        &[(0, 1), (1, 2), (2, 3)],
    );
    let mut follower = PathFollower::new(&zero_graph, path(&[0, 1, 2, 3])).unwrap();

    let abandoned = follower.prepare_advance(0.5).unwrap();
    assert_eq!(abandoned.result().position, [0.5, 0.0, 0.0]);
    drop(abandoned);
    assert_eq!(follower.expected_position(), [0.0, 0.0, 0.0]);
    assert!(!follower.is_complete());

    let retry = follower.prepare_advance(1.0).unwrap();
    assert_eq!(retry.result().position, [1.0, 0.0, 0.0]);
    assert_eq!(retry.result().next_node, Some(3));
    assert_eq!(retry.result().consumed_distance, 1.0);
    assert!(!retry.result().reached_goal);
    retry.commit();

    let finish = follower.prepare_advance(1.0).unwrap();
    assert_eq!(finish.result().position, [2.0, 0.0, 0.0]);
    assert_eq!(finish.result().next_node, None);
    assert!(finish.result().reached_goal);
    finish.commit();

    let after_finish = follower.prepare_advance(10.0).unwrap();
    assert_eq!(after_finish.result(), follower_result([2.0, 0.0, 0.0]));
}

#[test]
fn positive_binary64_progress_may_keep_the_rounded_position_unchanged() {
    let graph = graph(&[[1.0, 0.0, 0.0], [2.0, 0.0, 0.0]], &[(0, 1)]);
    let mut follower = PathFollower::new(&graph, path(&[0, 1])).unwrap();
    let tiny_step = f32::from_bits(1);

    let first = follower.prepare_advance(tiny_step).unwrap();
    assert_eq!(first.result().position, [1.0, 0.0, 0.0]);
    assert_eq!(first.result().consumed_distance, tiny_step);
    assert!(!first.result().reached_goal);
    first.commit();

    let next = follower.prepare_advance(0.5).unwrap();
    assert_eq!(next.result().position, [1.5, 0.0, 0.0]);
    assert_eq!(next.result().consumed_distance, 0.5);
}

#[test]
fn rounded_endpoint_and_lost_subtraction_fail_atomically_and_can_retry() {
    let unit_graph = graph(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], &[(0, 1)]);
    let mut endpoint = PathFollower::new(&unit_graph, path(&[0, 1])).unwrap();

    for exponent in 1..=53 {
        let budget = 2.0_f32.powi(-exponent);
        endpoint.prepare_advance(budget).unwrap().commit();
    }

    assert_eq!(endpoint.expected_position(), [1.0, 0.0, 0.0]);
    assert!(!endpoint.is_complete());
    assert_eq!(
        endpoint.prepare_advance(3.0 * 2.0_f32.powi(-55)).err(),
        Some(FollowError::PrecisionLoss)
    );
    assert_eq!(endpoint.expected_position(), [1.0, 0.0, 0.0]);
    assert!(!endpoint.is_complete());

    endpoint
        .prepare_advance(2.0_f32.powi(-53))
        .unwrap()
        .commit();
    assert!(endpoint.is_complete());

    let large_graph = graph(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [f32::MAX, 0.0, 0.0]],
        &[(0, 1), (1, 2)],
    );
    let mut large = PathFollower::new(&large_graph, path(&[0, 1, 2])).unwrap();
    assert_eq!(
        large.prepare_advance(f32::MAX).err(),
        Some(FollowError::PrecisionLoss)
    );
    assert_eq!(large.expected_position(), [0.0, 0.0, 0.0]);
    assert!(!large.is_complete());

    let retry = large.prepare_advance(1.0).unwrap();
    assert_eq!(retry.result().position, [1.0, 0.0, 0.0]);
    assert_eq!(retry.result().next_node, Some(2));
}

#[test]
fn stateless_zero_budget_preserves_bits_and_consumes_only_zero_boundaries() {
    let zero_graph = graph(
        &[
            [-0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        &[(0, 1), (1, 2), (2, 3)],
    );
    let input = [-0.0, 0.0, 0.0];
    let result = follow_path(&zero_graph, &[0, 1, 2, 3], input, 0.0).unwrap();

    assert_eq!(result.position[0].to_bits(), input[0].to_bits());
    assert_eq!(result.next_node, Some(3));
    assert_eq!(result.consumed_distance, 0.0);
    assert!(!result.reached_goal);

    let all_zero_graph = graph(
        &[[2.0, 3.0, 4.0], [2.0, 3.0, 4.0], [2.0, 3.0, 4.0]],
        &[(0, 1), (1, 2)],
    );
    let all_zero = follow_path(&all_zero_graph, &[0, 1, 2], [2.0, 3.0, 4.0], 0.0).unwrap();
    assert_eq!(all_zero.next_node, None);
    assert!(all_zero.reached_goal);
    assert_eq!(all_zero.consumed_distance, 0.0);

    let extreme_graph = graph(&[[-f32::MAX, 0.0, 0.0], [0.0, 0.0, 0.0]], &[(0, 1)]);
    let near_endpoint = [-f32::from_bits(1), 0.0, 0.0];
    let stationary = follow_path(&extreme_graph, &[0, 1], near_endpoint, 0.0).unwrap();

    assert_eq!(stationary.position[0].to_bits(), near_endpoint[0].to_bits());
    assert_eq!(stationary.next_node, Some(1));
    assert_eq!(stationary.consumed_distance, 0.0);
    assert!(!stationary.reached_goal);
}
