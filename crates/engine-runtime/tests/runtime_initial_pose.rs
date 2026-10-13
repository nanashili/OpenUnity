//! Runtime acceptance for the PHYSICS-POSE-001 initial orientation contract.
//! This file uses only public `Runtime` APIs and keeps authored Transform bits
//! (`qT`) separate from the backend orientation comparison (`qB`).

use engine_identifiers::ObjectId;
use engine_runtime::Runtime;

const MAX_ONE_NORMALIZATION_ANGLE_RADIANS: f64 = 2.384_185_81e-7;
const POSITION_ONLY_SYNC_TARGET: [f32; 3] = [4.0, -1.5, 2.25];
const MANUAL_STEP_SECONDS: f32 = 0.015625;

#[derive(Clone, Copy)]
struct InitialPoseCase {
    name: &'static str,
    input_rotation: [f32; 4],
    expected_transform_bits: [u32; 4],
}

const IDENTITY_CASE: InitialPoseCase = InitialPoseCase {
    name: "identity-positive-x",
    input_rotation: [0.0, 0.0, 0.0, 1.0],
    expected_transform_bits: [0x0000_0000, 0x0000_0000, 0x0000_0000, 0x3f80_0000],
};

const OBSERVED_NONIDENTITY_CASE: InitialPoseCase = InitialPoseCase {
    name: "nonidentity-y-positive-x",
    input_rotation: [0.0, 0.6, 0.0, 0.8],
    expected_transform_bits: [0x0000_0000, 0x3f19_999a, 0x0000_0000, 0x3f4c_cccd],
};

const SYNTHETIC_NONIDEMPOTENCE_CASE: InitialPoseCase = InitialPoseCase {
    name: "synthetic-normalizer-nonidempotence",
    input_rotation: [1.0, 1.0, 6.0, 6.0],
    expected_transform_bits: [0x3dee_133e, 0x3dee_133e, 0x3f32_8e6e, 0x3f32_8e6e],
};

#[test]
fn identity_authored_rotation_survives_sync_and_zero_angular_step() {
    assert_initial_pose_survives_position_only_sync_and_step(IDENTITY_CASE);
}

#[test]
fn observed_nonidentity_authored_rotation_survives_sync_and_zero_angular_step() {
    assert_initial_pose_survives_position_only_sync_and_step(OBSERVED_NONIDENTITY_CASE);
}

#[test]
fn synthetic_nonidempotence_pose_keeps_authored_bits_separate_from_body_bound() {
    assert_initial_pose_survives_position_only_sync_and_step(SYNTHETIC_NONIDEMPOTENCE_CASE);
}

fn assert_initial_pose_survives_position_only_sync_and_step(case: InitialPoseCase) {
    let mut fixture = PoseRuntimeFixture::attach(case);

    fixture.sync_authored_position();
    fixture.run_zero_angular_velocity_step();
}

struct PoseRuntimeFixture {
    runtime: Runtime,
    case: InitialPoseCase,
    transform: ObjectId,
    body: ObjectId,
    attached_body_rotation: [f32; 4],
}

impl PoseRuntimeFixture {
    fn attach(case: InitialPoseCase) -> Self {
        let mut runtime = Runtime::default();
        let owner = runtime
            .create_gameobject(case.name)
            .expect("create pose owner");
        let transform = runtime
            .gameobject_transform(owner)
            .expect("read owner Transform");

        runtime
            .transform_write(transform, 2, &case.input_rotation)
            .expect("author initial local rotation");
        assert_transform_bits(&runtime, transform, case, "before attachment");

        let body = runtime
            .rigidbody_reserve(owner)
            .expect("reserve body with authored initial pose");
        runtime
            .rigidbody_commit(body)
            .expect("commit body with authored initial pose");
        assert_transform_bits(&runtime, transform, case, "after attachment");

        let attached_body_rotation = backend_rotation(&runtime, body);
        assert_backend_orientation_bound(case, attached_body_rotation, "after attachment");

        Self {
            runtime,
            case,
            transform,
            body,
            attached_body_rotation,
        }
    }

    fn sync_authored_position(&mut self) {
        self.runtime
            .physics_mode_write(1)
            .expect("select Script mode for manual simulation");
        self.runtime
            .transform_write(self.transform, 1, &POSITION_ONLY_SYNC_TARGET)
            .expect("author position without changing rotation");
        assert_transform_bits(
            &self.runtime,
            self.transform,
            self.case,
            "after position write",
        );

        self.runtime
            .physics_sync_transforms()
            .expect("synchronize the authored position");
        assert_transform_bits(
            &self.runtime,
            self.transform,
            self.case,
            "after SyncTransforms",
        );
        assert_eq!(
            self.runtime
                .rigidbody_state(self.body)
                .expect("read synced body")
                .position(),
            POSITION_ONLY_SYNC_TARGET,
            "{}: SyncTransforms should publish the authored position",
            self.case.name
        );

        let body_rotation_after_sync = backend_rotation(&self.runtime, self.body);
        assert_eq!(
            quaternion_bits(body_rotation_after_sync),
            quaternion_bits(self.attached_body_rotation),
            "{}: position-only SyncTransforms should retain qB",
            self.case.name
        );
        assert_backend_orientation_bound(
            self.case,
            body_rotation_after_sync,
            "after SyncTransforms",
        );
    }

    fn run_zero_angular_velocity_step(&mut self) {
        self.configure_stationary_body();

        self.runtime
            .physics_simulate(MANUAL_STEP_SECONDS)
            .expect("run one zero-angular-velocity step");
        self.assert_zero_step_preserved_pose();
    }

    fn configure_stationary_body(&mut self) {
        self.runtime
            .set_physics_gravity([0.0; 3])
            .expect("disable gravity for a stationary step");
        self.runtime
            .rigidbody_set_linear_velocity(self.body, [0.0; 3])
            .expect("keep the body stationary");
        self.runtime
            .rigidbody_set_angular_velocity(self.body, [0.0; 3])
            .expect("disable angular integration");

        assert_transform_bits(
            &self.runtime,
            self.transform,
            self.case,
            "after zero-motion body property writes",
        );

        let body_rotation_after_property_writes = backend_rotation(&self.runtime, self.body);
        assert_eq!(
            quaternion_bits(body_rotation_after_property_writes),
            quaternion_bits(self.attached_body_rotation),
            "{}: body property writes should retain qB bits",
            self.case.name
        );
        assert_backend_orientation_bound(
            self.case,
            body_rotation_after_property_writes,
            "after body property writes",
        );
    }

    fn assert_zero_step_preserved_pose(&self) {
        assert_transform_bits(
            &self.runtime,
            self.transform,
            self.case,
            "after zero-angular step",
        );

        let stepped_body = self
            .runtime
            .rigidbody_state(self.body)
            .expect("read stepped body");
        assert_eq!(
            stepped_body.position(),
            POSITION_ONLY_SYNC_TARGET,
            "{}: zero velocity and gravity should keep the body stationary",
            self.case.name
        );

        let body_rotation_after_step = stepped_body.rotation();
        assert_eq!(
            quaternion_bits(body_rotation_after_step),
            quaternion_bits(self.attached_body_rotation),
            "{}: a zero-angular step should retain qB bits",
            self.case.name
        );
        assert_backend_orientation_bound(
            self.case,
            body_rotation_after_step,
            "after zero-angular step",
        );
    }
}

fn assert_transform_bits(
    runtime: &Runtime,
    transform: ObjectId,
    case: InitialPoseCase,
    phase: &str,
) {
    for (selector, channel) in [(2, "localRotation"), (5, "rotation")] {
        let rotation = runtime
            .transform_read(transform, selector)
            .expect("read Transform rotation");
        let actual_bits: Vec<u32> = rotation.iter().map(|value| value.to_bits()).collect();

        assert_eq!(
            actual_bits.as_slice(),
            case.expected_transform_bits.as_slice(),
            "{}: authored {channel} bits changed {phase}",
            case.name
        );
    }
}

fn backend_rotation(runtime: &Runtime, body: ObjectId) -> [f32; 4] {
    runtime
        .rigidbody_state(body)
        .expect("read copied backend body state")
        .rotation()
}

fn assert_backend_orientation_bound(case: InitialPoseCase, body_rotation: [f32; 4], phase: &str) {
    let transform_rotation = case.expected_transform_bits.map(f32::from_bits);
    let angle = sign_insensitive_orientation_angle(transform_rotation, body_rotation);

    assert!(
        angle <= MAX_ONE_NORMALIZATION_ANGLE_RADIANS,
        "{}: qB is {angle:.12e} radians from qT {phase}; bound is {:.12e}",
        case.name,
        MAX_ONE_NORMALIZATION_ANGLE_RADIANS
    );
}

fn sign_insensitive_orientation_angle(left: [f32; 4], right: [f32; 4]) -> f64 {
    let left = normalized_f64(left);
    let right = normalized_f64(right);
    let opposite_right = right.map(|component| -component);

    let direct_chord = chord_length(left, right);
    let antipodal_chord = chord_length(left, opposite_right);
    let sign_insensitive_chord = direct_chord.min(antipodal_chord);

    4.0 * (sign_insensitive_chord / 2.0).asin()
}

fn normalized_f64(rotation: [f32; 4]) -> [f64; 4] {
    assert!(
        rotation.iter().all(|component| component.is_finite()),
        "backend quaternion must be finite"
    );

    let widened = rotation.map(f64::from);
    let squared_norm = widened
        .iter()
        .map(|component| component * component)
        .sum::<f64>();
    let norm = squared_norm.sqrt();
    assert!(
        norm.is_finite() && norm > 0.0,
        "backend quaternion must be nonzero"
    );

    widened.map(|component| component / norm)
}

fn chord_length(left: [f64; 4], right: [f64; 4]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left_component, right_component)| {
            let difference = *left_component - right_component;
            difference * difference
        })
        .sum::<f64>()
        .sqrt()
}

fn quaternion_bits(rotation: [f32; 4]) -> [u32; 4] {
    rotation.map(f32::to_bits)
}
