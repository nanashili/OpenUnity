use engine_math::Vector;

#[derive(Debug, Clone, Copy, PartialEq)]
/// Failure returned when an analog value cannot be processed.
pub enum AnalogError {
    /// An input component is not finite.
    NonFinite,
    /// The deadzone is non-finite or outside the half-open interval `[0, 1)`.
    InvalidDeadzone,
    /// The input magnitude is zero where a direction is required.
    ZeroMagnitude,
    /// The processed value cannot be represented with finite `f32` components.
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Settings for radial deadzone processing of a three-dimensional input.
pub struct AnalogSettings {
    /// Inclusive magnitude threshold mapped to zero.
    pub deadzone: f32,
    /// Return unit direction when true, remapped magnitude otherwise.
    pub normalize: bool,
}

/// Apply a radial deadzone and deterministic magnitude mapping.
///
/// Inputs at or below the deadzone return zero. Above it, `normalize = true`
/// returns unit direction; `false` scales the direction by the magnitude
/// remapped from `[deadzone, 1]` to `[0, 1]`.
///
/// Components are widened to `f64` before magnitude calculations to keep the
/// deadzone boundary deterministic across intermediate binary32 rounding.
pub fn process_analog(value: Vector, settings: AnalogSettings) -> Result<Vector, AnalogError> {
    validate_settings(settings.deadzone)?;
    validate_input(value)?;

    let components = widen_components(value);
    let magnitude = magnitude(components);

    if magnitude == 0.0 {
        return Ok([0.0, 0.0, 0.0]);
    }

    let deadzone = f64::from(settings.deadzone);
    if magnitude <= deadzone {
        return Ok([0.0, 0.0, 0.0]);
    }

    let direction = normalized_direction(components, magnitude);
    let output = if settings.normalize {
        direction
    } else {
        let remapped_magnitude = remap_magnitude(magnitude, deadzone);
        scale_direction(direction, remapped_magnitude)
    };

    narrow_components(output)
}

fn validate_settings(deadzone: f32) -> Result<(), AnalogError> {
    if !deadzone.is_finite() || !(0.0..1.0).contains(&deadzone) {
        return Err(AnalogError::InvalidDeadzone);
    }

    Ok(())
}

fn validate_input(value: Vector) -> Result<(), AnalogError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(AnalogError::NonFinite)
    }
}

fn widen_components(value: Vector) -> [f64; 3] {
    value.map(f64::from)
}

fn magnitude(components: [f64; 3]) -> f64 {
    let squared = components[0] * components[0]
        + components[1] * components[1]
        + components[2] * components[2];

    squared.sqrt()
}

fn remap_magnitude(magnitude: f64, deadzone: f64) -> f64 {
    let remapped = (magnitude - deadzone) / (1.0 - deadzone);
    remapped.clamp(0.0, 1.0)
}

fn normalized_direction(components: [f64; 3], magnitude: f64) -> [f64; 3] {
    [
        components[0] / magnitude,
        components[1] / magnitude,
        components[2] / magnitude,
    ]
}

fn scale_direction(direction: [f64; 3], magnitude: f64) -> [f64; 3] {
    [
        direction[0] * magnitude,
        direction[1] * magnitude,
        direction[2] * magnitude,
    ]
}

fn narrow_components(value: [f64; 3]) -> Result<Vector, AnalogError> {
    let mut narrowed = [0.0; 3];

    for (index, component) in value.into_iter().enumerate() {
        if !component.is_finite()
            || component < f64::from(f32::MIN)
            || component > f64::from(f32::MAX)
        {
            return Err(AnalogError::Overflow);
        }

        narrowed[index] = component as f32;
    }

    Ok(narrowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_boundary_is_inclusive() {
        let settings = AnalogSettings {
            deadzone: 0.5,
            normalize: false,
        };

        assert_eq!(process_analog([0.5, 0.0, 0.0], settings), Ok([0.0; 3]));
    }

    #[test]
    fn remaps_magnitude_above_deadzone_without_normalizing() {
        let settings = AnalogSettings {
            deadzone: 0.25,
            normalize: false,
        };

        assert_eq!(
            process_analog([0.625, 0.0, 0.0], settings),
            Ok([0.5, 0.0, 0.0])
        );
    }

    #[test]
    fn normalization_returns_unit_direction() {
        let settings = AnalogSettings {
            deadzone: 0.25,
            normalize: true,
        };

        assert_eq!(
            process_analog([0.625, 0.0, 0.0], settings),
            Ok([1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn zero_input_is_zero_when_normalizing() {
        let settings = AnalogSettings {
            deadzone: 0.0,
            normalize: true,
        };

        assert_eq!(process_analog([0.0; 3], settings), Ok([0.0; 3]));
    }

    #[test]
    fn rejects_nonfinite_input_and_invalid_deadzone() {
        let finite_settings = AnalogSettings {
            deadzone: 0.0,
            normalize: false,
        };
        assert_eq!(
            process_analog([f32::NAN, 0.0, 0.0], finite_settings),
            Err(AnalogError::NonFinite)
        );

        let invalid_settings = AnalogSettings {
            deadzone: 1.0,
            normalize: false,
        };
        assert_eq!(
            process_analog([0.0; 3], invalid_settings),
            Err(AnalogError::InvalidDeadzone)
        );
    }
}
