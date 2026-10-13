use super::*;

use engine_identifiers::allocate_next;
use engine_math::Quaternion;
use engine_physics_api::{BodyId, BodyInterpolation, BodyState, PhysicsError};

const POSITION: Vector = [1.0, 2.0, 3.0];
const IDENTITY_ROTATION: Quaternion = [0.0, 0.0, 0.0, 1.0];

fn fresh_body() -> BodyId {
    allocate_next()
        .expect("test body identity should be available")
        .into()
}

fn reserved_state(world: &MotionWorld, id: BodyId) -> BodyState {
    world
        .reserved_body(id)
        .expect("expected a reserved body slot")
        .state
}

fn assert_empty_at_revision(world: &MotionWorld, id: BodyId, revision: u64) {
    assert_eq!(world.revision, revision);
    assert!(world.registry.is_empty());
    assert_eq!(world.body(id), Err(PhysicsError::InvalidBody));
}

#[test]
fn legacy_reserve_preserves_identity_pose_and_default_profile() {
    let mut world = MotionWorld::default();
    let id = fresh_body();

    world
        .reserve(id, POSITION)
        .expect("legacy identity reservation should succeed");

    let state = reserved_state(&world, id);
    let configuration = state.configuration();

    assert_eq!(state.position(), POSITION);
    assert_eq!(state.rotation(), IDENTITY_ROTATION);
    assert_eq!(state.linear_velocity(), [0.0; 3]);
    assert!(!state.is_sleeping());
    assert_eq!(configuration.mass(), 1.0);
    assert!(configuration.use_gravity());
    assert_eq!(configuration.angular_velocity(), [0.0; 3]);
    assert!(!configuration.is_kinematic());
    assert!(!configuration.detect_collisions());
    assert_eq!(configuration.linear_damping(), 0.0);
    assert_eq!(configuration.angular_damping(), 0.0);
    assert_eq!(configuration.sleep_threshold(), 0.0);
    assert_eq!(configuration.interpolation(), BodyInterpolation::None);
    assert_eq!(world.revision, 1);
}

#[test]
fn reservation_checks_capacity_before_invalid_pose_values() {
    let mut world = MotionWorld::default();
    let occupied_id = fresh_body();
    let rejected_id = fresh_body();

    world.reserve(occupied_id, POSITION).unwrap();
    for _ in 1..crate::MAX_MOTION_BODIES {
        world
            .reserve(fresh_body(), POSITION)
            .expect("each body below the limit should reserve capacity");
    }

    let original = reserved_state(&world, occupied_id);

    assert_eq!(
        world.reserve(rejected_id, [f32::NAN, 0.0, 0.0]),
        Err(PhysicsError::Capacity)
    );
    assert_eq!(
        world.reserve_pose(rejected_id, [f32::NAN, 0.0, 0.0], [0.0; 4],),
        Err(PhysicsError::Capacity)
    );

    assert_eq!(
        world.revision,
        u64::try_from(crate::MAX_MOTION_BODIES).unwrap()
    );
    assert_eq!(reserved_state(&world, occupied_id), original);
}

#[test]
fn reserve_pose_applies_one_normalization_within_sign_insensitive_bound() {
    let mut world = MotionWorld::default();
    let id = fresh_body();
    let transform_rotation = [
        f32::from_bits(0x3dee_133e),
        f32::from_bits(0x3dee_133e),
        f32::from_bits(0x3f32_8e6e),
        f32::from_bits(0x3f32_8e6e),
    ];
    let one_normalization_bits = [0x3dee_133f, 0x3dee_133f, 0x3f32_8e6e, 0x3f32_8e6e];

    world
        .reserve_pose(id, POSITION, transform_rotation)
        .expect("finite nonzero pose should be admitted");

    let state = reserved_state(&world, id);
    let body_rotation = state.rotation();

    assert_eq!(state.position(), POSITION);
    assert_eq!(
        body_rotation.map(f32::to_bits),
        one_normalization_bits,
        "this native-only fixture pins one BodyState normalization, not Unity body bits"
    );

    let body_length = quaternion_length(body_rotation);
    assert!((body_length - 1.0).abs() <= f64::from(f32::EPSILON));

    let angular_error = sign_insensitive_angular_error(transform_rotation, body_rotation);
    let maximum_error = 4.0 * (1.0 / 16_777_216.0_f64).asin();

    assert!(
        angular_error <= maximum_error,
        "angular error {angular_error} exceeds one-normalization bound {maximum_error}"
    );
}

#[test]
fn invalid_position_and_rotation_leave_the_world_empty_and_retryable() {
    let mut world = MotionWorld::default();
    let id = fresh_body();
    let invalid_positions = [[f32::NAN, 0.0, 0.0], [0.0, f32::INFINITY, 0.0]];
    let invalid_rotations = [
        [0.0; 4],
        [f32::NAN, 0.0, 0.0, 1.0],
        [f32::INFINITY, 0.0, 0.0, 1.0],
    ];

    for position in invalid_positions {
        assert_eq!(
            world.reserve_pose(id, position, IDENTITY_ROTATION),
            Err(PhysicsError::Numeric)
        );
        assert_empty_at_revision(&world, id, 0);
    }

    for rotation in invalid_rotations {
        assert_eq!(
            world.reserve_pose(id, POSITION, rotation),
            Err(PhysicsError::Numeric)
        );
        assert_empty_at_revision(&world, id, 0);
    }

    world
        .reserve_pose(id, POSITION, IDENTITY_ROTATION)
        .expect("valid retry should succeed after rejected poses");

    assert_eq!(reserved_state(&world, id).position(), POSITION);
    assert_eq!(world.revision, 1);
}

#[test]
fn reserve_pose_validates_pose_before_revision_and_publishes_atomically() {
    let mut world = MotionWorld::default();
    let id = fresh_body();
    world.revision = u64::MAX;

    assert_eq!(
        world.reserve_pose(id, [f32::NAN, 0.0, 0.0], IDENTITY_ROTATION),
        Err(PhysicsError::Numeric)
    );
    assert_empty_at_revision(&world, id, u64::MAX);

    assert_eq!(
        world.reserve_pose(id, POSITION, [0.0; 4]),
        Err(PhysicsError::Numeric)
    );
    assert_empty_at_revision(&world, id, u64::MAX);

    assert_eq!(
        world.reserve_pose(id, POSITION, IDENTITY_ROTATION),
        Err(PhysicsError::RevisionExhausted)
    );
    assert_empty_at_revision(&world, id, u64::MAX);
}

fn quaternion_length(rotation: Quaternion) -> f64 {
    rotation
        .iter()
        .map(|component| f64::from(*component).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn normalized_f64(rotation: Quaternion) -> [f64; 4] {
    let length = quaternion_length(rotation);

    rotation.map(|component| f64::from(component) / length)
}

fn chord_distance(left: [f64; 4], right: [f64; 4]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn sign_insensitive_angular_error(left: Quaternion, right: Quaternion) -> f64 {
    let left = normalized_f64(left);
    let right = normalized_f64(right);
    let opposite = right.map(|component| -component);
    let distance = chord_distance(left, right).min(chord_distance(left, opposite));

    4.0 * (distance / 2.0).asin()
}
