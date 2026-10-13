//! Runtime-owned body property validation and Transform revision metadata.
//!
//! `MotionWorld` remains the only owner of live body state. Runtime keeps only
//! the authored/acknowledged revision pair and prepares owned `BodyState`
//! candidates for the world's atomic `set_body` operation.

use engine_math::{Vector, finite};
use engine_physics_api::{BodyConfiguration, BodyInterpolation, BodyState, PhysicsError};

/// A supported copied property update.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BodyProperty {
    Position(Vector),
    LinearVelocity(Vector),
    AngularVelocity(Vector),
    Mass(f32),
    UseGravity(bool),
    IsKinematic(bool),
    DetectCollisions(bool),
    LinearDamping(f32),
    AngularDamping(f32),
    SleepThreshold(f32),
    Interpolation(BodyInterpolation),
}

/// Runtime metadata that coordinates an attached Transform with the backend.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BodyProperties {
    authored_position_revision: u64,
    acknowledged_position_revision: u64,
}

impl BodyProperties {
    pub(crate) fn is_position_dirty(self) -> bool {
        self.authored_position_revision != self.acknowledged_position_revision
    }

    /// Prepare the next Transform-authored revision without changing metadata.
    pub(crate) fn next_authored_revision(self) -> Result<u64, PhysicsError> {
        self.authored_position_revision
            .checked_add(1)
            .ok_or(PhysicsError::RevisionExhausted)
    }

    /// Commit a previously checked revision while holding the Runtime borrow.
    pub(crate) fn commit_authored_revision(&mut self, prepared: u64) {
        self.authored_position_revision = prepared;
    }

    /// Acknowledge only after `MotionWorld::set_body` has succeeded.
    pub(crate) fn acknowledge_authored_revision(&mut self) {
        self.acknowledged_position_revision = self.authored_position_revision;
    }

    /// Prepare a backend candidate only when the Transform revision is dirty.
    pub(crate) fn sync_authored_position(
        self,
        body: BodyState,
        position: Vector,
    ) -> Result<Option<(BodyState, u64)>, PhysicsError> {
        finite(&position)?;
        validate_profile(body.configuration())?;
        if !self.is_position_dirty() {
            return Ok(None);
        }
        let mut candidate = body;
        candidate.set_position(position)?;
        Ok(Some((candidate, self.authored_position_revision)))
    }
}

/// Prepare one body property as an owned candidate without mutating storage.
pub(crate) fn prepare_property(
    body: BodyState,
    property: BodyProperty,
) -> Result<BodyState, PhysicsError> {
    let mut candidate = body;

    match property {
        BodyProperty::Position(position) => candidate.set_position(position)?,
        BodyProperty::LinearVelocity(velocity) => candidate.set_linear_velocity(velocity)?,
        BodyProperty::AngularVelocity(velocity) => {
            let mut profile = candidate.configuration();
            profile.set_angular_velocity(velocity)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::Mass(mass) => {
            let mut profile = candidate.configuration();
            profile.set_mass(mass)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::UseGravity(enabled) => {
            let mut profile = candidate.configuration();
            profile.set_use_gravity(enabled);
            candidate.set_configuration(profile);
        }
        BodyProperty::IsKinematic(enabled) => {
            let mut profile = candidate.configuration();
            profile.set_is_kinematic(enabled)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::DetectCollisions(enabled) => {
            let mut profile = candidate.configuration();
            profile.set_detect_collisions(enabled)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::LinearDamping(damping) => {
            let mut profile = candidate.configuration();
            profile.set_linear_damping(damping)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::AngularDamping(damping) => {
            let mut profile = candidate.configuration();
            profile.set_angular_damping(damping)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::SleepThreshold(threshold) => {
            let mut profile = candidate.configuration();
            profile.set_sleep_threshold(threshold)?;
            candidate.set_configuration(profile);
        }
        BodyProperty::Interpolation(interpolation) => {
            let mut profile = candidate.configuration();
            profile.set_interpolation(interpolation);
            candidate.set_configuration(profile);
        }
    }

    validate_profile(candidate.configuration())?;
    Ok(candidate)
}

/// Validate the complete collider-free profile before backend publication.
pub(crate) fn validate_profile(profile: BodyConfiguration) -> Result<(), PhysicsError> {
    finite(&[
        profile.mass(),
        profile.linear_damping(),
        profile.angular_damping(),
        profile.sleep_threshold(),
    ])?;
    finite(&profile.angular_velocity())?;
    if profile.mass() <= 0.0
        || profile.is_kinematic()
        || profile.detect_collisions()
        || profile.linear_damping() != 0.0
        || profile.angular_damping() != 0.0
        || profile.sleep_threshold() != 0.0
        || profile.interpolation() != BodyInterpolation::None
    {
        return Err(PhysicsError::Unsupported);
    }
    Ok(())
}
