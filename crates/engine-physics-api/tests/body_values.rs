//! Primary acceptance of the owned finite body value/profile contract.

use engine_identifiers::{ObjectId, allocate_next};
use engine_physics_api::{BodyConfiguration, BodyId, BodyInterpolation, BodyState, PhysicsError};

#[test]
fn body_identity_and_finite_state_copies_preserve_owner_values() {
    let object = allocate_next().unwrap();
    let body = BodyId::from(object);
    assert_eq!(ObjectId::from(body), object);

    let mut state = BodyState::new([1.0, 2.0, 3.0]).unwrap();
    assert_eq!(state.linear_velocity(), [0.0; 3]);
    assert!(!state.is_sleeping());

    let mut configuration = state.configuration();
    configuration.set_mass(2.0).unwrap();
    configuration.set_use_gravity(false);
    configuration
        .set_angular_velocity([-0.0, 0.0, 0.0])
        .unwrap();
    configuration.set_is_kinematic(false).unwrap();
    configuration.set_detect_collisions(false).unwrap();
    configuration.set_linear_damping(0.0).unwrap();
    configuration.set_angular_damping(0.0).unwrap();
    configuration.set_sleep_threshold(0.0).unwrap();
    configuration.set_interpolation(BodyInterpolation::None);
    state.set_configuration(configuration);
    state.set_linear_velocity([4.0, 5.0, 6.0]).unwrap();
    state.wake_up();

    let snapshot = state;
    state.set_position([7.0, 8.0, 9.0]).unwrap();
    assert_eq!(snapshot.position(), [1.0, 2.0, 3.0]);
    assert_eq!(snapshot.linear_velocity(), [4.0, 5.0, 6.0]);
    assert_eq!(snapshot.configuration().mass(), 2.0);
    assert!(!snapshot.configuration().use_gravity());
    assert_eq!(snapshot.configuration().angular_velocity(), [0.0; 3]);
    assert!(!snapshot.configuration().is_kinematic());
    assert!(!snapshot.configuration().detect_collisions());
    assert_eq!(snapshot.configuration().linear_damping(), 0.0);
    assert_eq!(snapshot.configuration().angular_damping(), 0.0);
    assert_eq!(snapshot.configuration().sleep_threshold(), 0.0);
    assert_eq!(
        snapshot.configuration().interpolation(),
        BodyInterpolation::None
    );
    assert!(!snapshot.is_sleeping());
    assert_eq!(state.position(), [7.0, 8.0, 9.0]);
}

#[test]
fn invalid_state_vectors_preserve_the_complete_previous_value() {
    assert_eq!(
        BodyState::new([0.0, 0.0, f32::INFINITY]),
        Err(PhysicsError::Numeric)
    );
    let mut state = BodyState::new([1.0, 2.0, 3.0]).unwrap();
    state.set_linear_velocity([4.0, 5.0, 6.0]).unwrap();
    let previous = state;

    assert_eq!(
        state.set_position([7.0, f32::NAN, 9.0]),
        Err(PhysicsError::Numeric)
    );
    assert_eq!(state, previous);
    assert_eq!(
        state.set_linear_velocity([7.0, 8.0, f32::NEG_INFINITY]),
        Err(PhysicsError::Numeric)
    );
    assert_eq!(state, previous);

    let replacement = [f32::MAX, f32::MIN, f32::from_bits(1)];
    state.set_position(replacement).unwrap();
    assert_eq!(state.position(), replacement);
    assert_eq!(state.linear_velocity(), previous.linear_velocity());
}

#[test]
fn numeric_and_unsupported_profiles_fail_without_partial_replacement() {
    let mut configuration = BodyConfiguration::default();
    configuration.set_mass(5.0).unwrap();
    configuration.set_use_gravity(false);
    let previous = configuration;

    for mass in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(configuration.set_mass(mass), Err(PhysicsError::Numeric));
        assert_eq!(configuration, previous);
    }
    assert_eq!(
        configuration.set_angular_velocity([1.0, f32::NAN, 0.0]),
        Err(PhysicsError::Numeric)
    );
    assert_eq!(configuration, previous);
    assert_eq!(
        configuration.set_angular_velocity([0.0, 0.0, 1.0]),
        Err(PhysicsError::Unsupported)
    );
    assert_eq!(configuration, previous);
    assert_eq!(
        configuration.set_is_kinematic(true),
        Err(PhysicsError::Unsupported)
    );
    assert_eq!(configuration, previous);
    assert_eq!(
        configuration.set_detect_collisions(true),
        Err(PhysicsError::Unsupported)
    );
    assert_eq!(configuration, previous);

    type ReplaceScalar = fn(&mut BodyConfiguration, f32) -> Result<(), PhysicsError>;
    let replacements: [ReplaceScalar; 3] = [
        BodyConfiguration::set_linear_damping,
        BodyConfiguration::set_angular_damping,
        BodyConfiguration::set_sleep_threshold,
    ];
    for replace in replacements {
        assert_eq!(
            replace(&mut configuration, f32::INFINITY),
            Err(PhysicsError::Numeric)
        );
        assert_eq!(configuration, previous);
        assert_eq!(
            replace(&mut configuration, 0.125),
            Err(PhysicsError::Unsupported)
        );
        assert_eq!(configuration, previous);
    }
}
