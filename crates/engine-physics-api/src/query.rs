//! Copied values and errors for the bounded native AABB query capability.

use engine_identifiers::ObjectId;
use engine_math::{Vector, finite};
use std::fmt;

/// Failures reported by the bounded native query capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryError {
    /// The requested query geometry is invalid, such as an inverted bound or zero ray segment.
    InvalidQuery,
    /// An input or computed query value is nonfinite or cannot be represented safely.
    Numeric,
    /// The bounded query world cannot reserve another record or result value.
    Capacity,
    /// A query record already exists for the requested owner identity.
    DuplicateOwner,
    /// The query world's checked mutation revision cannot advance.
    RevisionExhausted,
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidQuery => "invalid query geometry",
            Self::Numeric => "invalid or unrepresentable query number",
            Self::Capacity => "query capacity exhausted",
            Self::DuplicateOwner => "owner already has a query record",
            Self::RevisionExhausted => "query mutation revision exhausted",
        })
    }
}

impl std::error::Error for QueryError {}

/// Copied nearest-hit data returned by a segment query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaySegmentHit {
    owner: ObjectId,
    parameter: f32,
    point: Vector,
}

impl RaySegmentHit {
    /// Construct a copied hit value after validating its finite segment result.
    ///
    /// # Errors
    /// Returns [`QueryError::Numeric`] for nonfinite values and
    /// [`QueryError::InvalidQuery`] when the parameter lies outside the segment.
    pub fn try_new(owner: ObjectId, parameter: f32, point: Vector) -> Result<Self, QueryError> {
        finite(&point).map_err(|_| QueryError::Numeric)?;
        if !parameter.is_finite() {
            return Err(QueryError::Numeric);
        }
        if !(0.0..=1.0).contains(&parameter) {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self {
            owner,
            parameter,
            point,
        })
    }

    /// Return the owner whose query volume was hit.
    pub fn owner(self) -> ObjectId {
        self.owner
    }

    /// Return the least segment parameter in the closed segment.
    pub fn parameter(self) -> f32 {
        self.parameter
    }

    /// Return the copied world-space hit point.
    pub fn point(self) -> Vector {
        self.point
    }
}
