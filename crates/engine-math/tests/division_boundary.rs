//! Independent finite-input and result boundaries for component division.

use engine_math::{MathError, divide};

#[test]
fn division_rejects_an_infinite_divisor_instead_of_returning_zero() {
    assert_eq!(
        divide([2.0, -6.0, 8.0], [f32::INFINITY, 3.0, -4.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn division_preserves_finite_results_and_existing_failure_categories() {
    assert_eq!(
        divide([2.0, -6.0, 8.0], [0.5, 3.0, -4.0]),
        Ok([4.0, -2.0, -2.0])
    );

    for zero in [0.0, -0.0] {
        assert_eq!(
            divide([2.0, -6.0, 8.0], [zero, 3.0, -4.0]),
            Err(MathError::Unsupported)
        );
    }

    assert_eq!(
        divide([f32::INFINITY, 1.0, 1.0], [0.0, 1.0, 1.0]),
        Err(MathError::Unsupported)
    );

    assert_eq!(
        divide([f32::MAX, 1.0, 1.0], [0.5, 1.0, 1.0]),
        Err(MathError::Numeric)
    );
}
