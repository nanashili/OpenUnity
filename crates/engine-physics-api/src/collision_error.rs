//! Shared failures for bounded collision worlds.

use std::fmt;

/// Typed failures returned by the independent 2D and 3D collision backends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionError {
    /// Gravity, initial pose, or a solver result is nonfinite or out of range.
    InvalidNumber,
    /// A box half-extent is nonfinite or outside the bounded supported range.
    InvalidDimensions,
    /// The caller timestep is nonfinite or outside the supported positive range.
    InvalidTimestep,
    /// The engine identity does not name a body in this world.
    InvalidBody,
    /// The engine identity already names a body in this world.
    BodyAlreadyExists,
    /// The bounded world body limit or storage capacity is exhausted.
    Capacity,
}

impl fmt::Display for CollisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidNumber => {
                "collision number is nonfinite or outside the supported numeric range"
            }
            Self::InvalidDimensions => {
                "collision box dimensions must be finite and within the supported range"
            }
            Self::InvalidTimestep => {
                "collision timestep must be finite and within the supported positive range"
            }
            Self::InvalidBody => "engine body identity is not owned by this collision world",
            Self::BodyAlreadyExists => {
                "engine body identity already exists in this collision world"
            }
            Self::Capacity => "collision world body capacity is exhausted",
        })
    }
}

impl std::error::Error for CollisionError {}
