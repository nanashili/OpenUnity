//! Conformance tests for bounded quaternion interpolation.

use engine_math::{MathError, Quaternion, slerp};

const HALF_TURN_COMPONENT: f32 = 0.70710677;
const MIDPOINT_Y_COMPONENT: f32 = 0.38268343;
const MIDPOINT_W_COMPONENT: f32 = 0.9238795;

fn assert_quaternion_close(actual: Quaternion, expected: Quaternion) {
    for (actual_component, expected_component) in actual.iter().zip(expected) {
        assert!((actual_component - expected_component).abs() < 2e-6);
    }
}

#[test]
fn slerp_returns_endpoints_and_an_independent_midpoint() {
    let identity: Quaternion = [0.0, 0.0, 0.0, 1.0];
    let quarter_turn: Quaternion = [0.0, HALF_TURN_COMPONENT, 0.0, HALF_TURN_COMPONENT];

    assert_quaternion_close(slerp(identity, quarter_turn, 0.0).unwrap(), identity);
    assert_quaternion_close(slerp(identity, quarter_turn, 1.0).unwrap(), quarter_turn);
    assert_quaternion_close(
        slerp(identity, quarter_turn, 0.5).unwrap(),
        [0.0, MIDPOINT_Y_COMPONENT, 0.0, MIDPOINT_W_COMPONENT],
    );
}

#[test]
fn slerp_chooses_the_shortest_hemisphere_for_opposite_signs() {
    let rotation: Quaternion = [0.0, HALF_TURN_COMPONENT, 0.0, HALF_TURN_COMPONENT];
    let opposite_sign = rotation.map(|component| -component);

    assert_quaternion_close(slerp(rotation, opposite_sign, 0.5).unwrap(), rotation);
}

#[test]
fn slerp_rejects_invalid_parameters_and_endpoints() {
    let identity: Quaternion = [0.0, 0.0, 0.0, 1.0];

    for parameter in [-f32::EPSILON, 2.0] {
        assert_eq!(
            slerp(identity, identity, parameter),
            Err(MathError::Unsupported)
        );
    }
    assert_eq!(slerp(identity, identity, f32::NAN), Err(MathError::Numeric));
    assert_eq!(
        slerp([0.0, 0.0, 0.0, 0.0], identity, 0.5),
        Err(MathError::Numeric)
    );
    assert_eq!(
        slerp([f32::INFINITY, 0.0, 0.0, 1.0], identity, 0.5),
        Err(MathError::Numeric)
    );
}
