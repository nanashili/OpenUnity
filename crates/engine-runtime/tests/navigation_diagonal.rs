//! Public regression for repeated diagonal runtime route advancement.

use engine_navigation::{NavigationEdge, NavigationGraph, NavigationNode};
use engine_runtime::Runtime;

#[test]
fn repeated_diagonal_steps_preserve_owned_route_progress() {
    let graph = NavigationGraph::build(
        vec![
            NavigationNode { position: [0.0; 3] },
            NavigationNode {
                position: [10.0, 1.0, 0.0],
            },
        ],
        vec![NavigationEdge {
            from: 0,
            to: 1,
            cost: 1.0,
        }],
    )
    .unwrap();
    let mut runtime = Runtime::default();
    let agent = runtime.create_gameobject("diagonal-agent").unwrap();
    let transform = runtime.gameobject_transform(agent).unwrap();
    let mut route = runtime.plan_navigation_route(agent, &graph, 0, 1).unwrap();
    runtime.advance_navigation_route(&mut route, 0.1).unwrap();
    let step = runtime
        .advance_navigation_route(&mut route, 0.1)
        .expect("a route must resume its own published diagonal position");
    let total_distance = 2.0 * f64::from(0.1_f32);
    let ratio = total_distance / 101.0_f64.sqrt();
    assert!((f64::from(step.position[0]) - 10.0 * ratio).abs() < 1e-7);
    assert!((f64::from(step.position[1]) - ratio).abs() < 1e-8);
    assert_eq!(
        runtime.transform_read(transform, 4).unwrap(),
        step.position.to_vec()
    );
}
