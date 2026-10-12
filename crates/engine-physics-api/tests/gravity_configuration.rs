//! Public-owner acceptance of the independent finite gravity setting contract.

use engine_physics_api::{GravityConfiguration, GravityError};

#[test]
fn configuration_stores_the_design_default_and_complete_finite_replacements() {
    let mut configuration = GravityConfiguration::default();
    assert_eq!(configuration.gravity(), [0.0, -9.81, 0.0]);

    let replacements = [
        [-2.0, 3.5, -0.125],
        [0.0; 3],
        [f32::MAX, f32::MIN, f32::from_bits(1)],
    ];
    for replacement in replacements {
        configuration.set_gravity(replacement).unwrap();
        assert_eq!(configuration.gravity(), replacement);
    }
}

#[test]
fn nonfinite_components_reject_the_entire_replacement() {
    let previous = [-2.0, 3.5, -0.125];
    let mut configuration = GravityConfiguration::default();
    configuration.set_gravity(previous).unwrap();

    for component in 0..3 {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut candidate = [13.0, 17.0, 19.0];
            candidate[component] = invalid;

            assert_eq!(
                configuration.set_gravity(candidate),
                Err(GravityError::NonFinite)
            );
            assert_eq!(configuration.gravity(), previous);
        }
    }
}
