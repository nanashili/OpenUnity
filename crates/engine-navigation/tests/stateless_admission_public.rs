//! Public conformance for exact stateless path-position admission.

use engine_navigation::{
    FollowError, NavigationEdge, NavigationGraph, NavigationNode, follow_path,
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

#[test]
fn endpoint_bounds_reject_minimum_subnormal_offsets_in_both_directions() {
    let minimum_subnormal = f32::from_bits(1);
    let forward = graph(&[[-f32::MAX, 0.0, 0.0], [0.0, 0.0, 0.0]], &[(0, 1)]);
    let reverse = graph(&[[f32::MAX, 0.0, 0.0], [0.0, 0.0, 0.0]], &[(0, 1)]);

    assert_eq!(
        follow_path(&forward, &[0, 1], [minimum_subnormal, 0.0, 0.0], 0.0),
        Err(FollowError::OffPath)
    );
    assert_eq!(
        follow_path(
            &forward,
            &[0, 1],
            [minimum_subnormal, 0.0, 0.0],
            minimum_subnormal,
        ),
        Err(FollowError::OffPath)
    );
    assert_eq!(
        follow_path(&reverse, &[0, 1], [-minimum_subnormal, 0.0, 0.0], 0.0),
        Err(FollowError::OffPath)
    );
}

#[test]
fn interior_minimum_subnormal_position_is_admitted_when_exactly_collinear() {
    let minimum_subnormal = f32::from_bits(1);
    let twice_minimum_subnormal = f32::from_bits(2);
    let route = graph(
        &[[0.0, 0.0, 0.0], [twice_minimum_subnormal, 0.0, 0.0]],
        &[(0, 1)],
    );
    let position = [minimum_subnormal, 0.0, 0.0];

    let result = follow_path(&route, &[0, 1], position, 0.0)
        .expect("the midpoint lies exactly on the segment");

    assert_eq!(result.position[0].to_bits(), minimum_subnormal.to_bits());
    assert_eq!(result.next_node, Some(1));
    assert_eq!(result.consumed_distance, 0.0);
    assert!(!result.reached_goal);
}

#[test]
fn in_bounds_minimum_subnormal_offsets_are_rejected_on_all_axis_pairs() {
    let minimum_subnormal = f32::from_bits(1);
    let twice_minimum_subnormal = f32::from_bits(2);

    let xy_route = graph(
        &[
            [0.0, 0.0, 0.0],
            [twice_minimum_subnormal, twice_minimum_subnormal, 0.0],
        ],
        &[(0, 1)],
    );
    assert_eq!(
        follow_path(&xy_route, &[0, 1], [minimum_subnormal, 0.0, 0.0], 0.0),
        Err(FollowError::OffPath)
    );

    let xz_route = graph(
        &[
            [0.0, 0.0, 0.0],
            [twice_minimum_subnormal, 0.0, twice_minimum_subnormal],
        ],
        &[(0, 1)],
    );
    assert_eq!(
        follow_path(&xz_route, &[0, 1], [minimum_subnormal, 0.0, 0.0], 0.0),
        Err(FollowError::OffPath)
    );

    let yz_route = graph(
        &[
            [0.0, 0.0, 0.0],
            [0.0, twice_minimum_subnormal, twice_minimum_subnormal],
        ],
        &[(0, 1)],
    );
    assert_eq!(
        follow_path(&yz_route, &[0, 1], [0.0, minimum_subnormal, 0.0], 0.0),
        Err(FollowError::OffPath)
    );
}

#[test]
fn one_minimum_subnormal_coordinate_mismatch_is_not_admitted() {
    let minimum_subnormal = f32::from_bits(1);
    let route = graph(
        &[
            [0.0, 0.0, 0.0],
            [
                3.0 * minimum_subnormal,
                6.0 * minimum_subnormal,
                9.0 * minimum_subnormal,
            ],
        ],
        &[(0, 1)],
    );
    let position = [
        minimum_subnormal,
        2.0 * minimum_subnormal,
        4.0 * minimum_subnormal,
    ];

    assert_eq!(
        follow_path(&route, &[0, 1], position, 0.0),
        Err(FollowError::OffPath)
    );
}

#[test]
fn exactly_collinear_position_has_zero_determinants_and_is_admitted() {
    let minimum_subnormal = f32::from_bits(1);
    let route = graph(
        &[
            [0.0, 0.0, 0.0],
            [
                3.0 * minimum_subnormal,
                6.0 * minimum_subnormal,
                9.0 * minimum_subnormal,
            ],
        ],
        &[(0, 1)],
    );
    let position = [
        minimum_subnormal,
        2.0 * minimum_subnormal,
        3.0 * minimum_subnormal,
    ];

    let result = follow_path(&route, &[0, 1], position, 0.0)
        .expect("one third of the endpoint vector lies exactly on the segment");

    assert_eq!(result.position, position);
    assert_eq!(result.next_node, Some(1));
    assert_eq!(result.consumed_distance, 0.0);
    assert!(!result.reached_goal);
}

#[test]
fn exact_predicate_rejects_small_noncollinear_offset_after_rounded_cancellation() {
    let route = graph(
        &[[-f32::MAX, -f32::MAX, 0.0], [f32::MAX, f32::MAX, 0.0]],
        &[(0, 1)],
    );

    assert_eq!(
        follow_path(&route, &[0, 1], [1.0, 2.0, 0.0], 0.0),
        Err(FollowError::OffPath)
    );
}

#[test]
fn exact_admission_preserves_signed_zero_bits_for_zero_budget() {
    let route = graph(&[[-0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], &[(0, 1)]);
    let position = [-0.0, -0.0, 0.0];

    let result = follow_path(&route, &[0, 1], position, 0.0).unwrap();

    for (actual, expected) in result.position.iter().zip(position) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
    assert_eq!(result.next_node, Some(1));
    assert_eq!(result.consumed_distance, 0.0);
    assert!(!result.reached_goal);
}
