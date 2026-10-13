//! Acceptance of copied native orientation values, without simulation claims.

use engine_physics_api::{BodyState, PhysicsError};

#[test]
fn new_body_has_identity_rotation() {
    let state = BodyState::new([1.0, 2.0, 3.0]).unwrap();

    assert_eq!(state.rotation(), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(state.position(), [1.0, 2.0, 3.0]);
    assert_eq!(state.linear_velocity(), [0.0; 3]);
    assert!(!state.is_sleeping());
}

#[test]
fn scaled_cardinal_rotations_normalize_without_changing_sign() {
    let mut state = seeded_body();

    state.set_rotation([0.0, 2.0, 0.0, 0.0]).unwrap();
    assert_eq!(state.rotation(), [0.0, 1.0, 0.0, 0.0]);

    state.set_rotation([0.0, 0.0, 0.0, -3.0]).unwrap();
    assert_eq!(state.rotation(), [0.0, 0.0, 0.0, -1.0]);
}

#[test]
fn general_rotation_has_unit_length_and_preserves_component_ratios() {
    let mut state = seeded_body();
    state.set_rotation([1.0, 2.0, 3.0, 4.0]).unwrap();
    let rotation = state.rotation().map(f64::from);
    let length = rotation
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();

    // Binary32 component rounding permits at most this absolute norm error.
    assert!((length - 1.0).abs() <= 2e-7);
    for (index, component) in rotation.into_iter().enumerate() {
        let original = (index + 1) as f64;
        let expected = original / 30.0_f64.sqrt();
        assert!((component - expected).abs() <= 2e-7);
        assert!((component / rotation[3] - original / 4.0).abs() <= 2e-7);
    }
}

#[test]
fn detached_rotation_and_body_copies_preserve_source_and_other_fields() {
    let source = seeded_body();
    let mut detached = source;
    detached.set_rotation([0.0, 0.0, -5.0, 0.0]).unwrap();

    assert_eq!(source.rotation(), [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(detached.rotation(), [0.0, 0.0, -1.0, 0.0]);
    assert_eq!(detached.position(), source.position());
    assert_eq!(detached.linear_velocity(), source.linear_velocity());
    assert_eq!(detached.configuration(), source.configuration());
    assert_eq!(detached.is_sleeping(), source.is_sleeping());

    let mut returned = detached.rotation();
    returned[2] = 0.5;
    assert_eq!(returned[2], 0.5);
    assert_eq!(detached.rotation(), [0.0, 0.0, -1.0, 0.0]);
}

#[test]
fn invalid_rotations_leave_complete_body_unchanged() {
    let mut state = seeded_body();
    let previous = state;

    assert_eq!(state.set_rotation([0.0; 4]), Err(PhysicsError::Numeric));
    assert_eq!(state, previous);

    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for index in 0..4 {
            let mut rotation = [1.0, 2.0, 3.0, 4.0];
            rotation[index] = invalid;
            assert_eq!(state.set_rotation(rotation), Err(PhysicsError::Numeric));
            assert_eq!(state, previous);
        }
    }
}

#[test]
fn extreme_finite_axis_inputs_normalize_without_overflow_or_underflow() {
    let mut state = seeded_body();

    state.set_rotation([f32::MAX, 0.0, 0.0, 0.0]).unwrap();
    assert_eq!(state.rotation(), [1.0, 0.0, 0.0, 0.0]);

    state
        .set_rotation([0.0, -f32::from_bits(1), 0.0, 0.0])
        .unwrap();
    assert_eq!(state.rotation(), [0.0, -1.0, 0.0, 0.0]);
}

fn seeded_body() -> BodyState {
    let mut state = BodyState::new([1.0, 2.0, 3.0]).unwrap();
    state.set_rotation([2.0, 0.0, 0.0, 0.0]).unwrap();
    state.set_linear_velocity([4.0, 5.0, 6.0]).unwrap();

    let mut configuration = state.configuration();
    configuration.set_mass(7.0).unwrap();
    configuration.set_use_gravity(false);
    configuration
        .set_angular_velocity([0.5, -1.0, 1.5])
        .unwrap();
    state.set_configuration(configuration);
    state
}
