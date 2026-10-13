mod interpolation;
mod query;

use crate::clip_data::{Channel, FiniteValue, Keyframe};

use interpolation::CurveInterpolator;

/// Interpolation supported by a validated curve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterpolationMode {
    /// Linearly interpolate supported scalar, vector, and quaternion values.
    Linear,
    /// Reserved mode rejected during construction.
    Step,
    /// Reserved mode rejected during construction.
    Cubic,
}

/// Behavior for finite queries outside the key range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Extrapolation {
    /// Return the nearest endpoint.
    Clamp,
    /// Repeat the finite key interval.
    Loop,
    /// Repeat the interval in alternating directions.
    PingPong,
    /// Return [`CurveError::OutOfDomain`].
    Reject,
}

/// Validation and sampling failures for a finite curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveError {
    /// The channel contains no keys.
    EmptyCurve,
    /// The key domain is nonfinite or has no span.
    InvalidDomain,
    /// A time input is nonfinite.
    NonFiniteTime,
    /// A query is outside the domain and rejection was requested.
    OutOfDomain,
    /// Key times are not strictly increasing.
    UnorderedKeys,
    /// Two adjacent keys have the same time.
    DuplicateTime,
    /// A key value is nonfinite.
    NonFiniteValue,
    /// The requested interpolation mode is unsupported.
    UnsupportedInterpolation,
    /// An interpolation intermediate is invalid.
    Numeric,
}

/// A validated channel with finite piecewise sampling semantics.
pub struct Curve<T> {
    channel: Channel<T>,
}

impl Curve<f32> {
    /// Validate a scalar channel for linear curve sampling.
    pub fn new(
        channel: Channel<f32>,
        interpolation: InterpolationMode,
    ) -> Result<Self, CurveError> {
        new_curve(channel, interpolation)
    }

    /// Sample a scalar curve at a finite time.
    pub fn sample(
        &self,
        time_seconds: f32,
        extrapolation: Extrapolation,
    ) -> Result<f32, CurveError> {
        sample_curve(self, time_seconds, extrapolation)
    }
}

impl Curve<engine_math::Vector> {
    /// Validate a vector channel for linear curve sampling.
    pub fn new(
        channel: Channel<engine_math::Vector>,
        interpolation: InterpolationMode,
    ) -> Result<Self, CurveError> {
        new_curve(channel, interpolation)
    }

    /// Sample a vector curve at a finite time.
    pub fn sample(
        &self,
        time_seconds: f32,
        extrapolation: Extrapolation,
    ) -> Result<engine_math::Vector, CurveError> {
        sample_curve(self, time_seconds, extrapolation)
    }
}

impl Curve<engine_math::Quaternion> {
    /// Validate a quaternion channel for spherical curve sampling.
    pub fn new(
        channel: Channel<engine_math::Quaternion>,
        interpolation: InterpolationMode,
    ) -> Result<Self, CurveError> {
        new_curve(channel, interpolation)
    }

    /// Sample a quaternion curve at a finite time.
    pub fn sample(
        &self,
        time_seconds: f32,
        extrapolation: Extrapolation,
    ) -> Result<engine_math::Quaternion, CurveError> {
        sample_curve(self, time_seconds, extrapolation)
    }
}

fn new_curve<T: FiniteValue>(
    channel: Channel<T>,
    interpolation: InterpolationMode,
) -> Result<Curve<T>, CurveError> {
    validate_keys(channel.keys())?;
    if !matches!(interpolation, InterpolationMode::Linear) {
        return Err(CurveError::UnsupportedInterpolation);
    }
    Ok(Curve { channel })
}

fn sample_curve<T: CurveInterpolator>(
    curve: &Curve<T>,
    time_seconds: f32,
    extrapolation: Extrapolation,
) -> Result<T, CurveError> {
    if !time_seconds.is_finite() {
        return Err(CurveError::NonFiniteTime);
    }

    let keys = curve.channel.keys();
    if keys.len() == 1 {
        return Ok(keys[0].value().clone());
    }

    let start = f64::from(keys[0].time_seconds());
    let end = f64::from(keys[keys.len() - 1].time_seconds());
    let query = query::resolve_query(f64::from(time_seconds), start, end, extrapolation)?;
    query::sample_in_domain(keys, query)
}

fn validate_keys<T: FiniteValue>(keys: &[Keyframe<T>]) -> Result<(), CurveError> {
    if keys.is_empty() {
        return Err(CurveError::EmptyCurve);
    }

    for (index, key) in keys.iter().enumerate() {
        if !key.time_seconds().is_finite() {
            return Err(CurveError::NonFiniteTime);
        }
        if !key.value().is_finite() {
            return Err(CurveError::NonFiniteValue);
        }
        if let Some(previous) = index.checked_sub(1).and_then(|i| keys.get(i)) {
            validate_time_order(key, previous)?;
        }
    }
    Ok(())
}

fn validate_time_order<T: FiniteValue>(
    key: &Keyframe<T>,
    previous: &Keyframe<T>,
) -> Result<(), CurveError> {
    if key.time_seconds() == previous.time_seconds() {
        return Err(CurveError::DuplicateTime);
    }
    if key.time_seconds() < previous.time_seconds() {
        return Err(CurveError::UnorderedKeys);
    }
    Ok(())
}
