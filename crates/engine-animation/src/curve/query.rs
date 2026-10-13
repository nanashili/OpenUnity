use crate::clip_data::{FiniteValue, Keyframe};

use super::{CurveError, Extrapolation, interpolation::CurveInterpolator};

pub(super) fn resolve_query(
    time: f64,
    start: f64,
    end: f64,
    extrapolation: Extrapolation,
) -> Result<f64, CurveError> {
    let period = end - start;
    if !period.is_finite() || period <= 0.0 {
        return Err(CurveError::InvalidDomain);
    }
    if (start..=end).contains(&time) {
        return Ok(time);
    }

    match extrapolation {
        Extrapolation::Clamp => Ok(time.clamp(start, end)),
        Extrapolation::Reject => Err(CurveError::OutOfDomain),
        Extrapolation::Loop => Ok(start + (time - start).rem_euclid(period)),
        Extrapolation::PingPong => resolve_ping_pong(time, start, end, period),
    }
}

fn resolve_ping_pong(time: f64, start: f64, end: f64, period: f64) -> Result<f64, CurveError> {
    let cycle = (time - start).rem_euclid(period * 2.0);
    if cycle <= period {
        Ok(start + cycle)
    } else {
        Ok(end - (cycle - period))
    }
}

pub(super) fn sample_in_domain<T: CurveInterpolator>(
    keys: &[Keyframe<T>],
    time: f64,
) -> Result<T, CurveError> {
    let last = keys.len() - 1;
    if time == f64::from(keys[0].time_seconds()) {
        return Ok(keys[0].value().clone());
    }
    if time == f64::from(keys[last].time_seconds()) {
        return Ok(keys[last].value().clone());
    }

    let upper = first_key_after(keys, time);
    let lower = upper - 1;
    let left = &keys[lower];
    let right = &keys[upper];
    let span = f64::from(right.time_seconds()) - f64::from(left.time_seconds());
    let parameter = (time - f64::from(left.time_seconds())) / span;
    if !parameter.is_finite() || !(0.0..=1.0).contains(&parameter) {
        return Err(CurveError::Numeric);
    }

    T::interpolate(left.value(), right.value(), parameter as f32)
}

fn first_key_after<T: FiniteValue>(keys: &[Keyframe<T>], time: f64) -> usize {
    let mut low = 0;
    let mut high = keys.len();
    while low < high {
        let middle = low + (high - low) / 2;
        if f64::from(keys[middle].time_seconds()) <= time {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}
