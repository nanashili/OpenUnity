//! Candidate public Runtime acceptance for independent multi-body motion.
//!
//! This test uses only the safe Runtime API. Its expected values implement the
//! OpenUnity design model; they are not a Unity multi-body reference trace.

use engine_identifiers::ObjectId;
use engine_runtime::Runtime;

#[derive(Clone, Copy)]
struct BodyFixture {
    owner: ObjectId,
    transform: ObjectId,
    body: ObjectId,
}

#[derive(Clone, Copy)]
struct ExpectedState {
    position: [f32; 3],
    velocity: [f32; 3],
    mass: f32,
    use_gravity: bool,
    detect_collisions: bool,
}

const GRAVITY_START: ExpectedState = ExpectedState {
    position: [1.25, 2.0, -3.5],
    velocity: [2.0, -1.0, 0.5],
    mass: 2.0,
    use_gravity: true,
    detect_collisions: false,
};

const GRAVITY_FREE_START: ExpectedState = ExpectedState {
    position: [-4.0, 0.5, 6.0],
    velocity: [-0.5, 2.0, -1.0],
    mass: 5.0,
    use_gravity: false,
    detect_collisions: false,
};

const GRAVITY_AFTER_ONE_STEP: ExpectedState = ExpectedState {
    position: [
        f32::from_bits(0x3fa4_0000),
        f32::from_bits(0x3ffd_c000),
        f32::from_bits(0xc05f_8000),
    ],
    velocity: [2.0, -1.125, 0.5],
    ..GRAVITY_START
};

const GRAVITY_FREE_AFTER_ONE_STEP: ExpectedState = ExpectedState {
    position: [-4.0078125, 0.53125, 5.984375],
    ..GRAVITY_FREE_START
};

const GRAVITY_AFTER_TWO_STEPS: ExpectedState = ExpectedState {
    position: [
        f32::from_bits(0x3fa8_0000),
        f32::from_bits(0x3ffb_4000),
        f32::from_bits(0xc05f_0000),
    ],
    velocity: [2.0, -1.25, 0.5],
    ..GRAVITY_START
};

const GRAVITY_FREE_AFTER_TWO_STEPS: ExpectedState = ExpectedState {
    position: [-4.015625, 0.5625, 5.96875],
    ..GRAVITY_FREE_START
};

fn create_body(runtime: &mut Runtime, name: &str) -> BodyFixture {
    let owner = runtime.create_gameobject(name).unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();

    BodyFixture {
        owner,
        transform,
        body,
    }
}

fn assert_identity_layout(runtime: &Runtime, fixture: BodyFixture) {
    assert_eq!(
        runtime.component_gameobject(fixture.body).unwrap(),
        fixture.owner
    );
    assert_eq!(runtime.physics_component_count(fixture.owner).unwrap(), 2);
    assert_eq!(
        runtime.physics_component_at(fixture.owner, 0).unwrap(),
        fixture.transform
    );
    assert_eq!(
        runtime.physics_component_at(fixture.owner, 1).unwrap(),
        fixture.body
    );
}

fn assert_snapshot(runtime: &Runtime, fixture: BodyFixture, expected: ExpectedState) {
    let state = runtime.rigidbody_state(fixture.body).unwrap();
    let configuration = state.configuration();

    assert_eq!(state.position(), expected.position);
    assert_eq!(state.linear_velocity(), expected.velocity);
    assert_eq!(configuration.mass(), expected.mass);
    assert_eq!(configuration.use_gravity(), expected.use_gravity);
    assert_eq!(
        configuration.detect_collisions(),
        expected.detect_collisions
    );
    assert_eq!(
        runtime.transform_read(fixture.transform, 1).unwrap(),
        expected.position.to_vec()
    );
}

fn assert_same_time(runtime: &Runtime, before: engine_runtime::TimeSnapshot) {
    let after = runtime.time_snapshot().unwrap();
    assert_eq!(after.values, before.values);
    assert_eq!(after.frame, before.frame);
    assert_eq!(after.fixed_context, before.fixed_context);
}

fn set_initial_conditions(
    runtime: &mut Runtime,
    gravity_body: BodyFixture,
    gravity_free_body: BodyFixture,
) {
    runtime.physics_mode_write(1).unwrap();
    runtime.set_physics_gravity([0.0, -8.0, 0.0]).unwrap();
    runtime
        .rigidbody_set_linear_velocity(gravity_body.body, [2.0, -1.0, 0.5])
        .unwrap();
    runtime
        .rigidbody_set_linear_velocity(gravity_free_body.body, [-0.5, 2.0, -1.0])
        .unwrap();
    runtime.rigidbody_set_mass(gravity_body.body, 2.0).unwrap();
    runtime
        .rigidbody_set_mass(gravity_free_body.body, 5.0)
        .unwrap();
    runtime
        .rigidbody_set_use_gravity(gravity_body.body, true)
        .unwrap();
    runtime
        .rigidbody_set_use_gravity(gravity_free_body.body, false)
        .unwrap();
    runtime
        .rigidbody_set_detect_collisions(gravity_body.body, false)
        .unwrap();
    runtime
        .rigidbody_set_detect_collisions(gravity_free_body.body, false)
        .unwrap();
    runtime
        .transform_write(gravity_body.transform, 1, &[1.25, 2.0, -3.5])
        .unwrap();
    runtime
        .transform_write(gravity_free_body.transform, 1, &[-4.0, 0.5, 6.0])
        .unwrap();
    runtime.physics_sync_transforms().unwrap();
}

fn assert_first_step(
    runtime: &mut Runtime,
    gravity_body: BodyFixture,
    gravity_free_body: BodyFixture,
) {
    let time_before = runtime.time_snapshot().unwrap();
    runtime.physics_simulate(0.015625).unwrap();
    assert_same_time(runtime, time_before);

    assert_snapshot(runtime, gravity_free_body, GRAVITY_FREE_AFTER_ONE_STEP);
    assert_snapshot(runtime, gravity_body, GRAVITY_AFTER_ONE_STEP);
}

fn assert_second_step(
    runtime: &mut Runtime,
    gravity_body: BodyFixture,
    gravity_free_body: BodyFixture,
) {
    let time_before = runtime.time_snapshot().unwrap();
    runtime.physics_simulate(0.015625).unwrap();
    assert_same_time(runtime, time_before);

    assert_snapshot(runtime, gravity_body, GRAVITY_AFTER_TWO_STEPS);
    assert_snapshot(runtime, gravity_free_body, GRAVITY_FREE_AFTER_TWO_STEPS);
}

fn assert_two_step_trace(
    runtime: &mut Runtime,
    gravity_body: BodyFixture,
    gravity_free_body: BodyFixture,
) {
    assert_snapshot(runtime, gravity_body, GRAVITY_START);
    assert_snapshot(runtime, gravity_free_body, GRAVITY_FREE_START);
    assert_first_step(runtime, gravity_body, gravity_free_body);
    assert_second_step(runtime, gravity_body, gravity_free_body);
}

#[test]
fn script_step_advances_each_body_once_and_publishes_its_own_transform() {
    let mut runtime = Runtime::default();
    let gravity_body = create_body(&mut runtime, "gravity-body-owner");
    let gravity_free_body = create_body(&mut runtime, "gravity-free-body-owner");

    assert_ne!(gravity_body.owner, gravity_free_body.owner);
    assert_ne!(gravity_body.transform, gravity_free_body.transform);
    assert_ne!(gravity_body.body, gravity_free_body.body);
    assert_identity_layout(&runtime, gravity_body);
    assert_identity_layout(&runtime, gravity_free_body);

    set_initial_conditions(&mut runtime, gravity_body, gravity_free_body);
    assert_two_step_trace(&mut runtime, gravity_body, gravity_free_body);
}
