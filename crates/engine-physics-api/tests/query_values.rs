//! Public-value and error acceptance for the bounded native query contract.

use engine_identifiers::allocate_next;
use engine_physics_api::{QueryError, RaySegmentHit};

#[test]
fn ray_segment_hit_copies_owner_parameter_and_point() {
    let owner = allocate_next().unwrap();
    let hit = RaySegmentHit::try_new(owner, 0.25, [1.0, 2.0, 3.0]).unwrap();
    let copied = hit;

    assert_eq!(copied.owner(), owner);
    assert_eq!(copied.parameter(), 0.25);
    assert_eq!(copied.point(), [1.0, 2.0, 3.0]);
    assert_eq!(hit, copied);
}

#[test]
fn ray_segment_hit_rejects_nonfinite_and_out_of_range_values() {
    let owner = allocate_next().unwrap();

    for parameter in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            RaySegmentHit::try_new(owner, parameter, [1.0, 2.0, 3.0]),
            Err(QueryError::Numeric)
        );
    }
    assert_eq!(
        RaySegmentHit::try_new(owner, -0.01, [1.0, 2.0, 3.0]),
        Err(QueryError::InvalidQuery)
    );
    assert_eq!(
        RaySegmentHit::try_new(owner, 1.01, [1.0, 2.0, 3.0]),
        Err(QueryError::InvalidQuery)
    );
    assert_eq!(
        RaySegmentHit::try_new(owner, 0.5, [1.0, f32::NAN, 3.0]),
        Err(QueryError::Numeric)
    );
}

#[test]
fn query_errors_are_distinct_and_descriptive() {
    let errors = [
        QueryError::InvalidQuery,
        QueryError::Numeric,
        QueryError::Capacity,
        QueryError::DuplicateOwner,
        QueryError::RevisionExhausted,
    ];

    for (index, error) in errors.iter().enumerate() {
        assert!(!errors[..index].contains(error));
        assert!(!error.to_string().is_empty());
    }
}
