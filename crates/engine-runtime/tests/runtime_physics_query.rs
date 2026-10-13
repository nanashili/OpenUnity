//! Direct Runtime coverage for the owner-bound AABB query contract.

use engine_identifiers::ObjectId;
use engine_physics_api::BodyState;
use engine_runtime::{Runtime, RuntimeError};

#[derive(Debug, PartialEq)]
struct PublicRuntimeSnapshot {
    body: BodyState,
    transform_position: Vec<f32>,
    transform_rotation: Vec<f32>,
    transform_scale: Vec<f32>,
    time_values: [f64; 6],
    time_frame: i32,
    fixed_context: bool,
    lifecycle_frame_count: i32,
}

fn owner(runtime: &mut Runtime, name: &str) -> ObjectId {
    runtime
        .create_gameobject(name)
        .expect("GameObject allocation")
}

fn ordered_owners(first: ObjectId, second: ObjectId) -> (ObjectId, ObjectId) {
    if first.to_raw() <= second.to_raw() {
        (first, second)
    } else {
        (second, first)
    }
}

fn observe_public_runtime(
    runtime: &Runtime,
    body: ObjectId,
    transform: ObjectId,
) -> PublicRuntimeSnapshot {
    let time = runtime.time_snapshot().unwrap();

    PublicRuntimeSnapshot {
        body: runtime.rigidbody_state(body).unwrap(),
        transform_position: runtime.transform_read(transform, 1).unwrap(),
        transform_rotation: runtime.transform_read(transform, 2).unwrap(),
        transform_scale: runtime.transform_read(transform, 3).unwrap(),
        time_values: time.values,
        time_frame: time.frame,
        fixed_context: time.fixed_context,
        lifecycle_frame_count: runtime.lifecycle_frame_count(),
    }
}

fn drain_timed_phase(runtime: &mut Runtime) -> Vec<i32> {
    let mut events = Vec::new();

    while let Some(delivery) = runtime.time_next(1).unwrap() {
        events.push(delivery.event);
        runtime.lifecycle_ack(delivery.token, 0).unwrap();
    }

    events
}

#[test]
fn queries_return_deterministic_hits_and_sorted_overlaps() {
    let mut runtime = Runtime::default();
    let first = owner(&mut runtime, "first");
    let second = owner(&mut runtime, "second");

    runtime
        .physics_aabb_add(first, [2.0, -1.0, -1.0], [3.0, 1.0, 1.0])
        .unwrap();
    runtime
        .physics_aabb_add(second, [2.0, -1.0, -1.0], [3.0, 1.0, 1.0])
        .unwrap();
    let (lower, upper) = ordered_owners(first, second);

    let hit = runtime
        .physics_raycast_segment([0.0; 3], [4.0, 0.0, 0.0])
        .unwrap()
        .unwrap();
    assert_eq!(hit.owner(), lower);
    assert_eq!(hit.parameter(), 0.5);
    assert_eq!(hit.point(), [2.0, 0.0, 0.0]);

    let overlaps = runtime
        .physics_overlap_aabb([3.0, -1.0, -1.0], [4.0, 1.0, 1.0])
        .unwrap();
    assert_eq!(overlaps, vec![lower, upper]);
}

#[test]
fn query_registration_rejects_invalid_owner_geometry_and_duplicates() {
    let mut runtime = Runtime::default();
    let game_object = owner(&mut runtime, "owner");
    let plain = runtime.create_object().unwrap();

    assert_eq!(
        runtime.physics_aabb_add(plain, [0.0; 3], [1.0; 3]),
        Err(RuntimeError::InvalidKind)
    );
    assert_eq!(
        runtime.physics_aabb_add(game_object, [1.0; 3], [0.0; 3]),
        Err(RuntimeError::InvalidPhysicsQuery)
    );
    runtime
        .physics_aabb_add(game_object, [0.0; 3], [1.0; 3])
        .unwrap();
    assert_eq!(
        runtime.physics_aabb_add(game_object, [0.0; 3], [1.0; 3]),
        Err(RuntimeError::Lifecycle)
    );
    assert_eq!(
        runtime.physics_raycast_segment([0.0; 3], [0.0; 3]),
        Err(RuntimeError::InvalidPhysicsQuery)
    );
}

#[test]
fn inactive_owners_remain_query_visible_and_worlds_are_isolated() {
    let mut first_runtime = Runtime::default();
    let first = owner(&mut first_runtime, "first");
    first_runtime
        .physics_aabb_add(first, [0.0; 3], [1.0; 3])
        .unwrap();
    first_runtime.gameobject_set_active(first, false).unwrap();

    assert_eq!(
        first_runtime
            .physics_raycast_segment([-1.0, 0.5, 0.5], [2.0, 0.0, 0.0])
            .unwrap()
            .unwrap()
            .owner(),
        first
    );
    assert_eq!(
        Runtime::default().physics_overlap_aabb([0.0; 3], [1.0; 3]),
        Ok(vec![])
    );
}

#[test]
fn queries_preserve_public_motion_clock_and_pending_dispatch_state() {
    let mut runtime = Runtime::default();
    let game_object = owner(&mut runtime, "query-purity-owner");
    let transform = runtime.gameobject_transform(game_object).unwrap();
    let body = runtime.rigidbody_reserve(game_object).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    runtime
        .rigidbody_set_linear_velocity(body, [1.0, 2.0, 3.0])
        .unwrap();
    let script = runtime.component_reserve(game_object, 0).unwrap();
    runtime.component_commit(script).unwrap();

    runtime.physics_mode_write(1).unwrap();
    runtime.time_configure(1, 0.02).unwrap();
    runtime.time_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();

    let before_queries = observe_public_runtime(&runtime, body, transform);

    runtime
        .physics_aabb_add(game_object, [1.0; 3], [3.0; 3])
        .unwrap();
    assert_eq!(
        observe_public_runtime(&runtime, body, transform),
        before_queries
    );

    let hit = runtime
        .physics_raycast_segment([0.0, 2.0, 2.0], [4.0, 0.0, 0.0])
        .unwrap()
        .unwrap();
    assert_eq!(hit.owner(), game_object);
    assert_eq!(
        observe_public_runtime(&runtime, body, transform),
        before_queries
    );

    assert_eq!(
        runtime.physics_overlap_aabb([2.0; 3], [4.0; 3]),
        Ok(vec![game_object])
    );
    assert_eq!(
        observe_public_runtime(&runtime, body, transform),
        before_queries
    );

    assert_eq!(drain_timed_phase(&mut runtime), vec![1, 2, 3]);
}
