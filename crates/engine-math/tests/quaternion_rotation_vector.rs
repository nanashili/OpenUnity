//! Independent conformance fixtures for INFRA-MATH-ROTATION-VECTOR-001.

use engine_math::{MathError, Quaternion, Vector, from_rotation_vector};

// These bounds apply only to pinned Rust 1.93.1 and aarch64-apple-darwin; the
// norm allowance covers binary32 quaternion normalization rounding.
const MAX_COMPONENT_ERROR: f32 = 1e-7;
const MAX_NORM_ERROR: f64 = 2e-7;

// Oracle inputs and nearest-binary32 formula values are pinned from the
// independent 100/200-digit mpmath fixture set. See generator SHA-256
// c790ca81829651122175bfa6cd982bf487c2d25d68c84cf45ff6cb7fb8e0db77 and
// fixture SHA-256
// 0dd24dea4755d48838aec1011a297583e624be90ffb86d7900558bc57739ab3b.
// The expected bits are accuracy references after the implementation's final
// normalization, not a bitwise-output promise.
const ORACLE_FIXTURES: &[RotationFixture] = &[
    fixture(
        "positive_x",
        [0x3f80_0000, 0, 0],
        [0x3ef5_7744, 0, 0, 0x3f60_a940],
    ),
    fixture(
        "negative_x",
        [0xbf80_0000, 0, 0],
        [0xbef5_7744, 0, 0, 0x3f60_a940],
    ),
    fixture(
        "positive_y",
        [0, 0x3f80_0000, 0],
        [0, 0x3ef5_7744, 0, 0x3f60_a940],
    ),
    fixture(
        "negative_y",
        [0, 0xbf80_0000, 0],
        [0, 0xbef5_7744, 0, 0x3f60_a940],
    ),
    fixture(
        "positive_z",
        [0, 0, 0x3f80_0000],
        [0, 0, 0x3ef5_7744, 0x3f60_a940],
    ),
    fixture(
        "negative_z",
        [0, 0, 0xbf80_0000],
        [0, 0, 0xbef5_7744, 0x3f60_a940],
    ),
    fixture(
        "negative_zero_orthogonal",
        [0x3f80_0000, 0x8000_0000, 0],
        [0x3ef5_7744, 0x8000_0000, 0, 0x3f60_a940],
    ),
    fixture(
        "tiny_positive_x",
        [0x33d6_bf95, 0, 0],
        [0x3356_bf95, 0, 0, 0x3f80_0000],
    ),
    fixture(
        "mixed_axis",
        [0x3e99_999a, 0xbf33_3333, 0x3f8c_cccd],
        [0x3e0e_65ca, 0xbea6_216b, 0x3f02_87f9, 0x3f48_d310],
    ),
    fixture(
        "four_radian_x_no_hemisphere_flip",
        [0x4080_0000, 0, 0],
        [0x3f68_c7b7, 0, 0, 0xbed5_1133],
    ),
    fixture(
        "exact_computed_norm_cap",
        [0x4980_0000, 0, 0],
        [0x3e2b_a40f, 0, 0, 0x3f7c_60cc],
    ),
];

struct RotationFixture {
    name: &'static str,
    input_bits: [u32; 3],
    expected_bits: [u32; 4],
}

const fn fixture(
    name: &'static str,
    input_bits: [u32; 3],
    expected_bits: [u32; 4],
) -> RotationFixture {
    RotationFixture {
        name,
        input_bits,
        expected_bits,
    }
}

fn input_from_bits(bits: [u32; 3]) -> Vector {
    bits.map(f32::from_bits)
}

fn expected_from_bits(bits: [u32; 4]) -> Quaternion {
    bits.map(f32::from_bits)
}

fn assert_component_accuracy(name: &str, actual: Quaternion, expected: Quaternion) {
    for (index, (actual_component, expected_component)) in actual.iter().zip(expected).enumerate() {
        let error = (actual_component - expected_component).abs();
        assert!(
            error <= MAX_COMPONENT_ERROR,
            "{name} component {index}: {actual_component:?} differs from {expected_component:?} by {error:?}"
        );
    }
}

fn assert_finite_and_normalized(quaternion: Quaternion) {
    assert!(quaternion.iter().all(|component| component.is_finite()));

    let squared_length = quaternion
        .iter()
        .map(|component| f64::from(*component).powi(2))
        .sum::<f64>();
    let norm_error = (squared_length.sqrt() - 1.0).abs();

    assert!(norm_error <= MAX_NORM_ERROR, "norm error {norm_error:?}");
}

#[test]
fn matches_independent_high_precision_rotation_fixtures() {
    for fixture in ORACLE_FIXTURES {
        let input = input_from_bits(fixture.input_bits);
        let actual = from_rotation_vector(input)
            .unwrap_or_else(|error| panic!("{} failed: {error:?}", fixture.name));

        assert_component_accuracy(
            fixture.name,
            actual,
            expected_from_bits(fixture.expected_bits),
        );
        assert_finite_and_normalized(actual);
    }
}

#[test]
fn every_signed_zero_vector_returns_canonical_identity() {
    for x_sign in [0, 0x8000_0000] {
        for y_sign in [0, 0x8000_0000] {
            for z_sign in [0, 0x8000_0000] {
                let input = input_from_bits([x_sign, y_sign, z_sign]);
                let actual = from_rotation_vector(input).unwrap();

                assert_eq!(
                    actual.map(f32::to_bits),
                    [0, 0, 0, 0x3f80_0000],
                    "input bits {x_sign:#010x}, {y_sign:#010x}, {z_sign:#010x}"
                );
            }
        }
    }
}

#[test]
fn basis_rotations_keep_structural_zeros_and_axis_sign() {
    for fixture in &ORACLE_FIXTURES[..6] {
        let input = input_from_bits(fixture.input_bits);
        let actual = from_rotation_vector(input).unwrap();

        let nonzero_axis = fixture
            .input_bits
            .iter()
            .position(|bits| bits & 0x7fff_ffff != 0)
            .unwrap();

        for (index, component) in actual[..3].iter().enumerate() {
            if index != nonzero_axis {
                assert_eq!(component.to_bits(), 0);
            }
        }

        let input_axis_is_negative = fixture.input_bits[nonzero_axis] & 0x8000_0000 != 0;
        assert_eq!(
            actual[nonzero_axis].is_sign_negative(),
            input_axis_is_negative
        );
    }

    let negative_zero = from_rotation_vector([1.0, -0.0, 0.0]).unwrap();
    assert_eq!(negative_zero[1].to_bits(), 0x8000_0000);
}

#[test]
fn tiny_rotation_keeps_a_nonzero_positive_axis_component() {
    let input = input_from_bits([0x33d6_bf95, 0, 0]);
    let actual = from_rotation_vector(input).unwrap();
    let expected = f32::from_bits(0x3356_bf95);

    assert!(actual[0] > 0.0);
    assert!((actual[0] - expected).abs() < 5e-9);
    assert_component_accuracy(
        "tiny_positive_x",
        actual,
        expected_from_bits([0x3356_bf95, 0, 0, 0x3f80_0000]),
    );
}

#[test]
fn computed_angle_cap_is_inclusive_and_next_value_is_rejected() {
    let cap = f32::from_bits(0x4980_0000);
    let accepted = from_rotation_vector([cap, 0.0, 0.0]).unwrap();

    assert_component_accuracy(
        "exact_computed_norm_cap",
        accepted,
        expected_from_bits([0x3e2b_a40f, 0, 0, 0x3f7c_60cc]),
    );

    let above_cap = f32::from_bits(0x4980_0001);
    assert_eq!(
        from_rotation_vector([above_cap, 0.0, 0.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn computed_norm_rounding_and_invalid_numeric_inputs_follow_the_contract() {
    let rounded_cap = [f32::from_bits(0x4980_0000), f32::from_bits(1), 0.0];
    let result = from_rotation_vector(rounded_cap).unwrap();

    assert_finite_and_normalized(result);

    assert_eq!(
        from_rotation_vector([f32::MAX, 0.0, 0.0]),
        Err(MathError::Numeric)
    );

    for component in 0..3 {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut input = [1.0, 0.0, 0.0];
            input[component] = invalid;

            assert_eq!(from_rotation_vector(input), Err(MathError::Numeric));
        }
    }
}
