//! Typed body identity and finite owned state, without world storage references.

use crate::{BodyConfiguration, PhysicsError};
use engine_identifiers::ObjectId;
use engine_math::{Vector, finite};

/// Native component identity supplied by the object owner; never allocated here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BodyId(ObjectId);

impl From<ObjectId> for BodyId {
    fn from(id: ObjectId) -> Self {
        Self(id)
    }
}

impl From<BodyId> for ObjectId {
    fn from(id: BodyId) -> Self {
        id.0
    }
}

/// A copied finite body snapshot/configuration, not a live storage reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    position: Vector,
    linear_velocity: Vector,
    configuration: BodyConfiguration,
    awake: bool,
}

impl BodyState {
    /// Create an awake body value with independent DESIGN profile defaults.
    /// # Errors
    /// Returns Numeric for a nonfinite initial position.
    pub fn new(position: Vector) -> Result<Self, PhysicsError> {
        finite(&position)?;
        Ok(Self {
            position,
            linear_velocity: [0.0; 3],
            configuration: BodyConfiguration::default(),
            awake: true,
        })
    }

    /// Complete copied native position, independent of any Transform publication.
    pub fn position(self) -> Vector {
        self.position
    }

    /// Complete copied linear velocity.
    pub fn linear_velocity(self) -> Vector {
        self.linear_velocity
    }

    /// Complete validated profile copy.
    pub fn configuration(self) -> BodyConfiguration {
        self.configuration
    }

    /// Whether the stored awake flag is false; this slice has no sleep transition.
    pub fn is_sleeping(self) -> bool {
        !self.awake
    }

    /// Replace this copy's complete position.
    /// # Errors
    /// Returns Numeric for a nonfinite component without changing the copy.
    pub fn set_position(&mut self, position: Vector) -> Result<(), PhysicsError> {
        finite(&position)?;
        self.position = position;
        Ok(())
    }

    /// Replace this copy's complete linear velocity.
    /// # Errors
    /// Returns Numeric for a nonfinite component without changing the copy.
    pub fn set_linear_velocity(&mut self, velocity: Vector) -> Result<(), PhysicsError> {
        finite(&velocity)?;
        self.linear_velocity = velocity;
        Ok(())
    }

    /// Replace this copy's already validated configuration.
    pub fn set_configuration(&mut self, configuration: BodyConfiguration) {
        self.configuration = configuration;
    }

    /// Set this copy's real awake flag; no sleep solver or callback is involved.
    pub fn wake_up(&mut self) {
        self.awake = true;
    }
}
