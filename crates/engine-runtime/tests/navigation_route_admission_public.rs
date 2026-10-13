//! Public conformance for Runtime navigation-route admission and precedence.

use engine_navigation::{NavigationEdge, NavigationGraph, NavigationNode, PathError};
use engine_runtime::{Runtime, RuntimeError, RuntimeNavigationError};

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

fn simple_graph() -> NavigationGraph {
    graph(&[[0.0; 3], [1.0, 0.0, 0.0]], &[(0, 1, 1.0)])
}

#[test]
fn planning_preserves_object_and_path_errors() {
    let route_graph = simple_graph();
    let mut runtime = Runtime::default();
    let plain = runtime.create_object().expect("create plain object");

    assert_eq!(
        runtime
            .plan_navigation_route(plain, &route_graph, 0, 1)
            .err(),
        Some(RuntimeNavigationError::Runtime(RuntimeError::InvalidKind))
    );

    let stale = runtime
        .create_gameobject("stale-navigation-agent")
        .expect("create stale test agent");
    assert!(runtime.destroy_object(stale));
    assert_eq!(
        runtime
            .plan_navigation_route(stale, &route_graph, 0, 1)
            .err(),
        Some(RuntimeNavigationError::Runtime(RuntimeError::InvalidObject))
    );

    let agent = runtime
        .create_gameobject("path-error-agent")
        .expect("create path test agent");
    assert_eq!(
        runtime
            .plan_navigation_route(agent, &route_graph, 0, 2)
            .err(),
        Some(RuntimeNavigationError::Path(PathError::InvalidNode))
    );

    let disconnected = graph(&[[0.0; 3], [1.0, 0.0, 0.0]], &[]);
    assert_eq!(
        runtime
            .plan_navigation_route(agent, &disconnected, 0, 1)
            .err(),
        Some(RuntimeNavigationError::Path(PathError::Unreachable))
    );
}

#[test]
fn parent_transform_error_precedes_invalid_advance_distance() {
    let route_graph = simple_graph();
    let mut runtime = Runtime::default();

    let parent = runtime
        .create_gameobject("navigation-parent")
        .expect("create parent");
    let agent = runtime
        .create_gameobject("parented-navigation-agent")
        .expect("create route agent");
    let parent_transform = runtime
        .gameobject_transform(parent)
        .expect("resolve parent transform");
    let agent_transform = runtime
        .gameobject_transform(agent)
        .expect("resolve agent transform");
    let mut route = runtime
        .plan_navigation_route(agent, &route_graph, 0, 1)
        .expect("plan route before reparenting");

    runtime
        .transform_set_parent(agent_transform, Some(parent_transform), false)
        .expect("parent the route agent");
    assert_eq!(
        runtime.advance_navigation_route(&mut route, f32::NAN),
        Err(RuntimeNavigationError::ParentTransformUnsupported)
    );
    assert_eq!(
        runtime
            .transform_read(agent_transform, 4)
            .expect("read parented pose"),
        vec![0.0; 3]
    );
    runtime
        .transform_set_parent(agent_transform, None, false)
        .expect("restore the route agent root");
    let parent_retry = runtime
        .advance_navigation_route(&mut route, 0.25)
        .expect("route remains usable after restoring its root hierarchy");
    assert_eq!(parent_retry.position, [0.25, 0.0, 0.0]);
}

#[test]
fn physics_attachment_error_precedes_invalid_advance_distance() {
    let route_graph = simple_graph();
    let mut runtime = Runtime::default();
    let physics_agent = runtime
        .create_gameobject("physics-navigation-agent")
        .expect("create physics route agent");
    let mut physics_route = runtime
        .plan_navigation_route(physics_agent, &route_graph, 0, 1)
        .expect("plan route before attaching a body");
    let body = runtime
        .rigidbody_reserve(physics_agent)
        .expect("reserve attached body");
    runtime
        .rigidbody_commit(body)
        .expect("publish attached body");

    assert_eq!(
        runtime
            .plan_navigation_route(physics_agent, &route_graph, 0, 1)
            .err(),
        Some(RuntimeNavigationError::PhysicsAttachmentUnsupported)
    );
    assert_eq!(
        runtime.advance_navigation_route(&mut physics_route, f32::NAN),
        Err(RuntimeNavigationError::PhysicsAttachmentUnsupported)
    );
    let physics_transform = runtime
        .gameobject_transform(physics_agent)
        .expect("resolve physics route transform");
    assert_eq!(
        runtime
            .transform_read(physics_transform, 4)
            .expect("read physics route pose"),
        vec![0.0; 3]
    );
    runtime
        .destroy_typed_object(body)
        .expect("remove the attached physics body");
    let physics_retry = runtime
        .advance_navigation_route(&mut physics_route, 0.25)
        .expect("route remains usable after removing the attachment");
    assert_eq!(physics_retry.position, [0.25, 0.0, 0.0]);
}

#[test]
fn lifetime_errors_precede_invalid_advance_distance() {
    let graph = simple_graph();
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("retired-navigation-agent")
        .expect("create route agent");
    let mut route = runtime
        .plan_navigation_route(agent, &graph, 0, 1)
        .expect("plan route");

    assert!(runtime.destroy_object(agent));
    assert_eq!(
        runtime.advance_navigation_route(&mut route, f32::NAN),
        Err(RuntimeNavigationError::Runtime(RuntimeError::InvalidObject))
    );
}

#[test]
fn runtime_route_limits_cover_nodes_and_directed_edges() {
    let mut runtime = Runtime::default();
    let agent = runtime
        .create_gameobject("bounded-navigation-agent")
        .expect("create route agent");

    let many_nodes: Vec<_> = (0..513)
        .map(|index| NavigationNode {
            position: [index as f32, 0.0, 0.0],
        })
        .collect();
    let maximum_node_graph = NavigationGraph::build(
        many_nodes.iter().take(512).copied().collect(),
        vec![NavigationEdge {
            from: 0,
            to: 511,
            cost: 1.0,
        }],
    )
    .expect("fixture at the runtime node bound is valid");
    assert!(
        runtime
            .plan_navigation_route(agent, &maximum_node_graph, 0, 511)
            .is_ok()
    );

    let node_limited = NavigationGraph::build(many_nodes, Vec::new())
        .expect("navigation graph accepts more nodes than Runtime routes");
    assert_eq!(
        runtime
            .plan_navigation_route(agent, &node_limited, 0, 1)
            .err(),
        Some(RuntimeNavigationError::ResourceLimit)
    );

    let points: Vec<_> = (0..65)
        .map(|index| {
            if index == 0 {
                [0.0; 3]
            } else {
                [index as f32, 1.0, 0.0]
            }
        })
        .collect();
    let mut too_many_edges = Vec::with_capacity(4_097);
    for from in 0..points.len() {
        for to in 0..points.len() {
            if from != to {
                too_many_edges.push((from, to, 1.0));
            }
            if too_many_edges.len() == 4_097 {
                break;
            }
        }
        if too_many_edges.len() == 4_097 {
            break;
        }
    }
    let exact_edge_bound = graph(&points, &too_many_edges[..4_096]);
    assert!(
        runtime
            .plan_navigation_route(agent, &exact_edge_bound, 0, 1)
            .is_ok()
    );

    let edge_limited = graph(&points, &too_many_edges);

    // Node zero is the agent's exact start and has a direct route to node one.
    assert_eq!(
        runtime
            .plan_navigation_route(agent, &edge_limited, 0, 1)
            .err(),
        Some(RuntimeNavigationError::ResourceLimit)
    );
}
