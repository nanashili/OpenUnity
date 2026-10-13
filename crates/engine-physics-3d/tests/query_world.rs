//! Contract tests for deterministic owner-bound AABB queries.

use engine_identifiers::{ObjectId, allocate_next};
use engine_math::Aabb;
use engine_physics_3d::{AabbQueryWorld, MAX_QUERY_BOXES};
use engine_physics_api::QueryError;

fn owner() -> ObjectId {
    allocate_next().unwrap()
}

fn bounds(min: [f32; 3], max: [f32; 3]) -> Aabb {
    Aabb::try_new(min, max).unwrap()
}

#[test]
fn registration_is_bounded_and_failure_atomic() {
    let mut world = AabbQueryWorld::default();
    let first = owner();
    world.add_aabb(first, bounds([0.0; 3], [1.0; 3])).unwrap();

    assert_eq!(
        world.add_aabb(first, bounds([2.0; 3], [3.0; 3])),
        Err(QueryError::DuplicateOwner)
    );
    assert_eq!(
        world.add_aabb_from_vectors(owner(), [2.0, 1.0, 0.0], [1.0, 2.0, 1.0]),
        Err(QueryError::InvalidQuery)
    );
    assert_eq!(
        world.add_aabb_from_vectors(owner(), [f32::NAN; 3], [1.0; 3]),
        Err(QueryError::Numeric)
    );
    assert_eq!(
        world.overlap_aabb(&bounds([0.0; 3], [1.0; 3])).unwrap(),
        vec![first]
    );

    let mut full = AabbQueryWorld::default();
    let mut owners = Vec::with_capacity(MAX_QUERY_BOXES + 1);
    for index in 0..MAX_QUERY_BOXES {
        let current = owner();
        owners.push(current);
        let coordinate = index as f32 * 2.0;
        full.add_aabb(current, bounds([coordinate; 3], [coordinate + 1.0; 3]))
            .unwrap();
    }
    let overflow = owner();
    assert_eq!(
        full.add_aabb(overflow, bounds([0.0; 3], [1.0; 3])),
        Err(QueryError::Capacity)
    );
    assert_eq!(
        full.overlap_aabb(&bounds([0.0; 3], [1.0; 3])).unwrap(),
        vec![owners[0]]
    );
}

#[test]
fn raycast_selects_exact_nearest_hit_and_lower_owner_tie() {
    let mut world = AabbQueryWorld::default();
    let near = owner();
    let far = owner();
    world
        .add_aabb(far, bounds([4.0, -1.0, -1.0], [5.0; 3]))
        .unwrap();
    world
        .add_aabb(near, bounds([2.0, -1.0, -1.0], [3.0; 3]))
        .unwrap();

    let hit = world
        .raycast_segment([0.0, 0.0, 0.0], [10.0, 0.0, 0.0])
        .unwrap()
        .unwrap();
    assert_eq!(hit.owner(), near);
    assert_eq!(hit.parameter(), 0.2);
    assert_eq!(hit.point(), [2.0, 0.0, 0.0]);

    let tie_world_owner = owner();
    let tie_world_other = owner();
    let mut ties = AabbQueryWorld::default();
    ties.add_aabb(tie_world_other, bounds([2.0, 0.0, 0.0], [3.0; 3]))
        .unwrap();
    ties.add_aabb(tie_world_owner, bounds([2.0, -1.0, -1.0], [3.0; 3]))
        .unwrap();
    let tie = ties
        .raycast_segment([0.0, 0.0, 0.0], [4.0, 0.0, 0.0])
        .unwrap()
        .unwrap();
    let lower_owner = if tie_world_owner.to_raw() < tie_world_other.to_raw() {
        tie_world_owner
    } else {
        tie_world_other
    };
    assert_eq!(tie.owner(), lower_owner);
}

#[test]
fn raycast_handles_inside_tangency_and_invalid_segments() {
    let mut world = AabbQueryWorld::default();
    let box_owner = owner();
    world
        .add_aabb(box_owner, bounds([0.0; 3], [1.0; 3]))
        .unwrap();

    let inside = world
        .raycast_segment([0.5; 3], [1.0, 0.0, 0.0])
        .unwrap()
        .unwrap();
    assert_eq!(inside.parameter(), 0.0);
    let tangent = world
        .raycast_segment([-1.0, 1.0, 1.0], [1.0, 0.0, 0.0])
        .unwrap()
        .unwrap();
    assert_eq!(tangent.parameter(), 1.0);
    assert_eq!(tangent.point(), [0.0, 1.0, 1.0]);
    assert_eq!(
        world.raycast_segment([2.0; 3], [0.0; 3]),
        Err(QueryError::InvalidQuery)
    );
}

#[test]
fn empty_world_still_rejects_invalid_ray_inputs() {
    let world = AabbQueryWorld::default();

    assert_eq!(
        world.raycast_segment([0.0; 3], [0.0; 3]),
        Err(QueryError::InvalidQuery)
    );
    assert_eq!(
        world.raycast_segment([f32::NAN; 3], [1.0, 0.0, 0.0]),
        Err(QueryError::Numeric)
    );
    assert_eq!(
        world.raycast_segment([0.0; 3], [f32::INFINITY, 0.0, 0.0]),
        Err(QueryError::Numeric)
    );
}

#[test]
fn overlap_is_closed_and_sorted_by_raw_owner() {
    let mut world = AabbQueryWorld::default();
    let lower = owner();
    let upper = owner();
    world.add_aabb(upper, bounds([1.0; 3], [2.0; 3])).unwrap();
    world.add_aabb(lower, bounds([0.0; 3], [1.0; 3])).unwrap();

    assert_eq!(
        world.overlap_aabb(&bounds([1.0; 3], [1.0; 3])).unwrap(),
        vec![lower, upper]
    );
}

#[test]
fn worlds_are_isolated_and_retirement_is_inert_until_commit() {
    let first = owner();
    let second = owner();
    let mut world = AabbQueryWorld::default();
    let mut other = AabbQueryWorld::default();
    world.add_aabb(first, bounds([0.0; 3], [1.0; 3])).unwrap();
    other.add_aabb(second, bounds([0.0; 3], [1.0; 3])).unwrap();

    let prepared = world.prepare_owner_retirement(&[first]);
    drop(prepared);
    assert_eq!(
        world.overlap_aabb(&bounds([0.0; 3], [1.0; 3])).unwrap(),
        vec![first]
    );
    assert_eq!(
        other.overlap_aabb(&bounds([0.0; 3], [1.0; 3])).unwrap(),
        vec![second]
    );

    world.prepare_owner_retirement(&[first]).unwrap().commit();
    assert!(
        world
            .overlap_aabb(&bounds([0.0; 3], [1.0; 3]))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        other.overlap_aabb(&bounds([0.0; 3], [1.0; 3])).unwrap(),
        vec![second]
    );
}
