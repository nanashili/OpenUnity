//! Public conformance for caller-owned Runtime navigation routes.

use engine_navigation::{
    FollowError, NavigationEdge, NavigationGraph, NavigationNode, PathFollower, shortest_path,
};
use engine_runtime::{Runtime, RuntimeNavigationError};

fn graph(points: &[[f32; 3]], edges: &[(usize, usize, f32)]) -> NavigationGraph {
    let nodes = points
        .iter()
        .copied()
        .map(|position| NavigationNode { position })
        .collect();
    let edges = edges
        .iter()
        .map(|(from, to, cost)| NavigationEdge {
            from: *from,
            to: *to,
            cost: *cost,
        })
        .collect();

    NavigationGraph::build(nodes, edges).expect("fixture graph is valid")
}

#[test]
fn route_advances_across_waypoints_and_matches_navigation_follower() {
    let graph = graph(
        &[[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 10.0)],
    );
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("navigation-agent")
        .expect("create route agent");
    let transform = runtime
        .gameobject_transform(agent)
        .expect("resolve route transform");
    let path = shortest_path(&graph, 0, 2).expect("route exists");
    let mut expected = PathFollower::new(&graph, path.clone()).expect("path is followable");
    let mut route = runtime
        .plan_navigation_route(agent, &graph, 0, 2)
        .expect("plan route from the agent's exact start position");

    assert_eq!(route.path(), &path);
    assert_eq!(route.agent(), agent);
    assert!(!route.is_complete());

    for (budget, expected_position, expected_next) in [
        (0.5, [0.5, 0.0, 0.0], Some(1)),
        (0.5, [1.0, 0.0, 0.0], Some(2)),
        (1.0, [2.0, 0.0, 0.0], None),
    ] {
        let expected_step = expected
            .prepare_advance(budget)
            .expect("follower accepts the bounded step");
        let expected_result = expected_step.result();
        expected_step.commit();

        let result = runtime
            .advance_navigation_route(&mut route, budget)
            .expect("runtime publishes the same route step");

        assert_eq!(result, expected_result);
        assert_eq!(result.position, expected_position);
        assert_eq!(result.next_node, expected_next);
        assert_eq!(
            runtime
                .transform_read(transform, 4)
                .expect("read route pose"),
            expected_position.to_vec()
        );
    }

    assert!(route.is_complete());
    let completed = runtime
        .advance_navigation_route(&mut route, 10.0)
        .expect("completed route is idempotent");
    assert_eq!(completed.position, [2.0, 0.0, 0.0]);
    assert_eq!(completed.next_node, None);
    assert_eq!(completed.consumed_distance, 0.0);
    assert!(completed.reached_goal);
}

#[test]
fn route_advancement_preserves_external_rotation_and_scale_writes() {
    let graph = graph(&[[0.0; 3], [1.0, 0.0, 0.0]], &[(0, 1, 1.0)]);
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("pose-owned-navigation-agent")
        .expect("create route agent");
    let transform = runtime
        .gameobject_transform(agent)
        .expect("resolve route transform");
    let mut route = runtime
        .plan_navigation_route(agent, &graph, 0, 1)
        .expect("plan route from the exact start position");

    runtime
        .transform_write(transform, 2, &[0.0, 0.0, 1.0, 0.0])
        .expect("write an independent rotation");
    runtime
        .transform_write(transform, 3, &[2.0, 3.0, 4.0])
        .expect("write an independent scale");
    let rotation = runtime
        .transform_read(transform, 2)
        .expect("read the authored rotation");
    assert_eq!(rotation, vec![0.0, 0.0, 1.0, 0.0]);
    let scale = runtime
        .transform_read(transform, 3)
        .expect("read the authored scale");
    assert_eq!(scale, vec![2.0, 3.0, 4.0]);

    let result = runtime
        .advance_navigation_route(&mut route, 0.5)
        .expect("advance after non-position Transform writes");
    let position_after = runtime
        .transform_read(transform, 4)
        .expect("read the route position");
    let rotation_after = runtime
        .transform_read(transform, 2)
        .expect("read the preserved rotation");
    let scale_after = runtime
        .transform_read(transform, 3)
        .expect("read the preserved scale");

    assert_eq!(result.position, [0.5, 0.0, 0.0]);
    assert_eq!(position_after, vec![0.5, 0.0, 0.0]);
    assert_eq!(rotation_after, rotation);
    assert_eq!(scale_after, scale);
}

#[test]
fn start_position_and_any_external_edit_are_checked_exactly() {
    let graph = graph(&[[0.0; 3], [1.0, 0.0, 0.0]], &[(0, 1, 1.0)]);
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("edited-navigation-agent")
        .expect("create route agent");
    let transform = runtime
        .gameobject_transform(agent)
        .expect("resolve route transform");

    runtime
        .transform_write(transform, 4, &[0.25, 0.0, 0.0])
        .expect("write a valid external position");
    assert_eq!(
        runtime.plan_navigation_route(agent, &graph, 0, 1).err(),
        Some(RuntimeNavigationError::StartPositionMismatch)
    );

    runtime
        .transform_write(transform, 4, &[0.0; 3])
        .expect("restore the selected start node");
    let mut route = runtime
        .plan_navigation_route(agent, &graph, 0, 1)
        .expect("plan from the exact start node");

    runtime
        .transform_write(transform, 4, &[0.5, 0.0, 0.0])
        .expect("make an on-path external edit");
    assert_eq!(
        runtime.advance_navigation_route(&mut route, 0.25),
        Err(RuntimeNavigationError::AgentPositionChanged)
    );
    assert_eq!(
        runtime
            .transform_read(transform, 4)
            .expect("read unchanged pose"),
        vec![0.5, 0.0, 0.0]
    );

    runtime
        .transform_write(transform, 4, &[0.0; 3])
        .expect("restore the last committed route position");
    let retry = runtime
        .advance_navigation_route(&mut route, 0.25)
        .expect("route remains usable after rejecting an external edit");
    assert_eq!(retry.position, [0.25, 0.0, 0.0]);
    assert_eq!(retry.consumed_distance, 0.25);
}

#[test]
fn precision_failure_keeps_runtime_pose_and_route_retryable() {
    let graph = graph(
        &[[0.0; 3], [1.0, 0.0, 0.0], [f32::MAX, 0.0, 0.0]],
        &[(0, 1, 1.0), (1, 2, 1.0)],
    );
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("precision-navigation-agent")
        .expect("create route agent");
    let transform = runtime
        .gameobject_transform(agent)
        .expect("resolve route transform");
    let mut route = runtime
        .plan_navigation_route(agent, &graph, 0, 2)
        .expect("plan large-exponent route");

    assert_eq!(
        runtime.advance_navigation_route(&mut route, f32::MAX),
        Err(RuntimeNavigationError::Follow(FollowError::PrecisionLoss))
    );
    assert_eq!(
        runtime
            .transform_read(transform, 4)
            .expect("read failed-step pose"),
        vec![0.0; 3]
    );
    assert_eq!(route.path().nodes, [0, 1, 2]);
    assert!(!route.is_complete());

    let retry = runtime
        .advance_navigation_route(&mut route, 1.0)
        .expect("retry a representable first segment");
    assert_eq!(retry.position, [1.0, 0.0, 0.0]);
    assert_eq!(retry.next_node, Some(2));
    assert_eq!(retry.consumed_distance, 1.0);
}

#[test]
fn rounded_endpoint_failure_does_not_publish_pose_or_completion() {
    let graph = graph(&[[0.0; 3], [1.0, 0.0, 0.0]], &[(0, 1, 1.0)]);
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("endpoint-navigation-agent")
        .expect("create route agent");
    let transform = runtime
        .gameobject_transform(agent)
        .expect("resolve route transform");
    let mut route = runtime
        .plan_navigation_route(agent, &graph, 0, 1)
        .expect("plan unit-length route");

    for exponent in 1..=53 {
        runtime
            .advance_navigation_route(&mut route, 2.0_f32.powi(-exponent))
            .expect("commit progressively smaller positive steps");
    }

    assert_eq!(
        runtime
            .transform_read(transform, 4)
            .expect("read rounded pose"),
        vec![1.0, 0.0, 0.0]
    );
    assert!(!route.is_complete());
    assert_eq!(
        runtime.advance_navigation_route(&mut route, 3.0 * 2.0_f32.powi(-55)),
        Err(RuntimeNavigationError::Follow(FollowError::PrecisionLoss))
    );
    assert_eq!(
        runtime
            .transform_read(transform, 4)
            .expect("read unchanged pose"),
        vec![1.0, 0.0, 0.0]
    );
    assert!(!route.is_complete());

    let retry = runtime
        .advance_navigation_route(&mut route, 2.0_f32.powi(-53))
        .expect("finish on a representable retry");
    assert!(retry.reached_goal);
    assert!(route.is_complete());
}
