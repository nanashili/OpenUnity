//! Independent simple trajectories through the real public backend owner.

use engine_identifiers::allocate_next;
use engine_physics_3d::MotionWorld;

#[test]
fn gravity_kicks_then_drifts_without_mass_dependence_or_preparation_mutation() {
    for mass in [2.0, 5.0] {
        let mut world = MotionWorld::default();
        let body = allocate_next().unwrap().into();
        world.reserve(body, [0.0; 3]).unwrap();
        world.commit(body).unwrap();
        let mut state = world.body(body).unwrap();
        state.set_linear_velocity([2.0, 0.0, -1.0]).unwrap();
        let mut configuration = state.configuration();
        configuration.set_mass(mass).unwrap();
        state.set_configuration(configuration);
        world.set_body(body, state).unwrap();

        let first = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
        let (candidate_id, candidate) = first.body().unwrap();
        assert_eq!(candidate_id, body);
        assert_eq!(candidate.position(), [0.5, -0.25, -0.25]);
        assert_eq!(candidate.linear_velocity(), [2.0, -1.0, -1.0]);
        assert_eq!(world.body(body).unwrap(), state);
        world.commit_step(first).unwrap();
        assert_eq!(world.body(body).unwrap(), candidate);

        let second = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
        world.commit_step(second).unwrap();
        let final_state = world.body(body).unwrap();
        assert_eq!(final_state.position(), [1.0, -0.75, -0.5]);
        assert_eq!(final_state.linear_velocity(), [2.0, -2.0, -1.0]);
        assert_eq!(final_state.configuration().mass(), mass);
        assert!(final_state.configuration().use_gravity());
        assert!(!final_state.is_sleeping());
    }
}

#[test]
fn disabling_gravity_retains_velocity_and_profile_across_real_steps() {
    let mut world = MotionWorld::default();
    let body = allocate_next().unwrap().into();
    world.reserve(body, [0.0; 3]).unwrap();
    world.commit(body).unwrap();
    let mut state = world.body(body).unwrap();
    state.set_linear_velocity([2.0, 0.0, -1.0]).unwrap();
    let mut configuration = state.configuration();
    configuration.set_mass(7.0).unwrap();
    configuration.set_use_gravity(false);
    state.set_configuration(configuration);
    state.wake_up();
    world.set_body(body, state).unwrap();

    let step = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    world.commit_step(step).unwrap();
    let final_state = world.body(body).unwrap();
    assert_eq!(final_state.position(), [0.5, 0.0, -0.25]);
    assert_eq!(final_state.linear_velocity(), [2.0, 0.0, -1.0]);
    assert_eq!(final_state.configuration(), configuration);
    assert!(!final_state.is_sleeping());
}
