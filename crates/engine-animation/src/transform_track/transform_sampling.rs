use engine_math::{Quaternion, Vector, finite, slerp};

use super::TransformTrackError;
use crate::curve::{Curve, Extrapolation};

pub(super) fn validate_time(time_seconds: f32) -> Result<(), TransformTrackError> {
    time_seconds
        .is_finite()
        .then_some(())
        .ok_or(TransformTrackError::NonFiniteTime)
}

pub(super) fn sample_vector(
    channel: &Option<Curve<Vector>>,
    time_seconds: f32,
) -> Result<Option<Vector>, TransformTrackError> {
    let Some(channel) = channel else {
        return Ok(None);
    };
    let value = channel
        .sample(time_seconds, Extrapolation::Clamp)
        .map_err(TransformTrackError::Curve)?;
    finite(&value).map_err(TransformTrackError::Numeric)?;
    Ok(Some(value))
}

pub(super) fn sample_rotation(
    channel: &Option<Curve<Quaternion>>,
    time_seconds: f32,
) -> Result<Option<Quaternion>, TransformTrackError> {
    let Some(channel) = channel else {
        return Ok(None);
    };
    let value = channel
        .sample(time_seconds, Extrapolation::Clamp)
        .map_err(TransformTrackError::Curve)?;
    slerp(value, value, 0.0)
        .map(Some)
        .map_err(TransformTrackError::Numeric)
}
