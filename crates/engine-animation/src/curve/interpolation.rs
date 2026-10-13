use engine_math::{MathError, Quaternion, Vector, lerp, slerp};

use crate::clip_data::FiniteValue;

use super::CurveError;

pub(super) trait CurveInterpolator: FiniteValue + Clone {
    fn interpolate(start: &Self, end: &Self, parameter: f32) -> Result<Self, CurveError>;
}

impl CurveInterpolator for f32 {
    fn interpolate(start: &Self, end: &Self, parameter: f32) -> Result<Self, CurveError> {
        scalar_lerp(*start, *end, parameter)
    }
}

impl CurveInterpolator for Vector {
    fn interpolate(start: &Self, end: &Self, parameter: f32) -> Result<Self, CurveError> {
        lerp(*start, *end, parameter).map_err(map_math_error)
    }
}

impl CurveInterpolator for Quaternion {
    fn interpolate(start: &Self, end: &Self, parameter: f32) -> Result<Self, CurveError> {
        slerp(*start, *end, parameter).map_err(map_math_error)
    }
}

fn scalar_lerp(start: f32, end: f32, parameter: f32) -> Result<f32, CurveError> {
    if !start.is_finite() || !end.is_finite() || !parameter.is_finite() {
        return Err(CurveError::Numeric);
    }
    if parameter == 0.0 {
        return Ok(start);
    }
    if parameter == 1.0 {
        return Ok(end);
    }

    let result = f64::from(start) + (f64::from(end) - f64::from(start)) * f64::from(parameter);
    if result.is_finite() && result >= f64::from(f32::MIN) && result <= f64::from(f32::MAX) {
        Ok(result as f32)
    } else {
        Err(CurveError::Numeric)
    }
}

fn map_math_error(error: MathError) -> CurveError {
    match error {
        MathError::Numeric | MathError::Unsupported => CurveError::Numeric,
    }
}
