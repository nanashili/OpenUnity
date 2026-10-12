//! Runtime-facing copied Rigidbody properties and explicit Transform sync.
//!
//! This source consumes the lifetime owner's attachment/admission helpers. It
//! keeps `MotionWorld` as the sole body-state owner and acknowledges revisions
//! only after an atomic backend publication succeeds.

use engine_identifiers::ObjectId;
use engine_physics_api::{BodyId, BodyInterpolation, BodyState, SimulationMode};

use super::attachment::PhysicsAttachment;
use crate::physics::body_properties::{BodyProperty, prepare_property};
use crate::physics::transform_sync::prepare_sync_transforms;
use crate::{Runtime, RuntimeError};

impl Runtime {
    /// Return a committed copied body state after identity/publication checks.
    pub fn rigidbody_state(&self, body: ObjectId) -> Result<BodyState, RuntimeError> {
        self.read_physics_body(body)
    }

    /// Replace a body property after lifecycle/owner admission and full
    /// candidate validation. The backend is the only mutation point.
    fn write_rigidbody_property(
        &mut self,
        body: ObjectId,
        property: BodyProperty,
    ) -> Result<(), RuntimeError> {
        let attachment = self.physics_property_admission(body)?;
        let state = self
            .physics
            .world
            .body(BodyId::from(body))
            .map_err(RuntimeError::from)?;
        let candidate = prepare_property(state, property).map_err(RuntimeError::from)?;
        self.physics
            .world
            .set_body(BodyId::from(body), candidate)
            .map_err(RuntimeError::from)?;

        if matches!(property, BodyProperty::Position(_)) {
            self.physics
                .attachment
                .as_mut()
                .expect("admitted body attachment remains live")
                .properties
                .acknowledge_authored_revision();
        }
        debug_assert_eq!(attachment.body_id, body);
        Ok(())
    }

    /// Set world position; errors report admission, non-finite, or unsupported values.
    pub fn rigidbody_set_position(
        &mut self,
        body: ObjectId,
        position: [f32; 3],
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::Position(position))
    }

    /// Set linear velocity; errors report admission, invalid, or unsupported values.
    pub fn rigidbody_set_linear_velocity(
        &mut self,
        body: ObjectId,
        velocity: [f32; 3],
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::LinearVelocity(velocity))
    }

    /// Set angular velocity; errors report admission, invalid, or unsupported values.
    pub fn rigidbody_set_angular_velocity(
        &mut self,
        body: ObjectId,
        velocity: [f32; 3],
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::AngularVelocity(velocity))
    }

    /// Set mass; errors report admission, invalid, or unsupported values.
    pub fn rigidbody_set_mass(&mut self, body: ObjectId, mass: f32) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::Mass(mass))
    }

    /// Enable or disable gravity; errors report admission or unsupported profiles.
    pub fn rigidbody_set_use_gravity(
        &mut self,
        body: ObjectId,
        enabled: bool,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::UseGravity(enabled))
    }

    /// Set kinematic state; errors report admission or unsupported profiles.
    pub fn rigidbody_set_is_kinematic(
        &mut self,
        body: ObjectId,
        enabled: bool,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::IsKinematic(enabled))
    }

    /// Set collision detection participation; errors report admission or unsupported profiles.
    pub fn rigidbody_set_detect_collisions(
        &mut self,
        body: ObjectId,
        enabled: bool,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::DetectCollisions(enabled))
    }

    /// Set linear damping; errors report admission, invalid, or unsupported values.
    pub fn rigidbody_set_linear_damping(
        &mut self,
        body: ObjectId,
        damping: f32,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::LinearDamping(damping))
    }

    /// Set angular damping; errors report admission, invalid, or unsupported values.
    pub fn rigidbody_set_angular_damping(
        &mut self,
        body: ObjectId,
        damping: f32,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::AngularDamping(damping))
    }

    /// Set sleep threshold; errors report admission, invalid, or unsupported values.
    pub fn rigidbody_set_sleep_threshold(
        &mut self,
        body: ObjectId,
        threshold: f32,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::SleepThreshold(threshold))
    }

    /// Set interpolation mode; errors report admission or unsupported profiles.
    pub fn rigidbody_set_interpolation(
        &mut self,
        body: ObjectId,
        interpolation: BodyInterpolation,
    ) -> Result<(), RuntimeError> {
        self.write_rigidbody_property(body, BodyProperty::Interpolation(interpolation))
    }

    /// Wake the copied native body through the same atomic publication path.
    pub fn rigidbody_wake_up(&mut self, body: ObjectId) -> Result<(), RuntimeError> {
        let attachment = self.physics_property_admission(body)?;
        let mut state = self
            .physics
            .world
            .body(BodyId::from(body))
            .map_err(RuntimeError::from)?;
        state.wake_up();
        self.physics
            .world
            .set_body(BodyId::from(body), state)
            .map_err(RuntimeError::from)?;
        debug_assert_eq!(attachment.body_id, body);
        Ok(())
    }

    /// Read the current simulation mode (`0` for Fixed, `1` for Script).
    pub fn physics_mode_read(&self) -> i32 {
        match self.physics.mode {
            SimulationMode::Fixed => 0,
            SimulationMode::Script => 1,
        }
    }

    /// Change mode during callbacks after health and staging checks only.
    pub fn physics_mode_write(&mut self, mode: i32) -> Result<(), RuntimeError> {
        self.physics_mode_admission()?;
        self.physics.mode = match mode {
            0 => SimulationMode::Fixed,
            1 => SimulationMode::Script,
            _ => return Err(RuntimeError::InvalidKind),
        };
        Ok(())
    }

    /// Import a dirty Transform into the committed backend body.
    pub fn physics_sync_transforms(&mut self) -> Result<(), RuntimeError> {
        self.physics_sync_admission()?;
        let Some(attachment) = self.physics.attachment else {
            return Ok(());
        };
        if !attachment.committed {
            return Ok(());
        }
        self.validate_supported_owner_transform(attachment.owner_id, attachment.transform_id)?;
        let state = self
            .physics
            .world
            .body(BodyId::from(attachment.body_id))
            .map_err(RuntimeError::from)?;
        let position = self.transform(attachment.transform_id)?.position;
        let (_, candidate) = prepare_sync_transforms(
            self.physics.mode,
            Some((state, &attachment.properties)),
            position,
        )
        .map_err(RuntimeError::from)?;
        let Some(candidate) = candidate else {
            return Ok(());
        };
        self.physics
            .world
            .set_body(BodyId::from(attachment.body_id), candidate)
            .map_err(RuntimeError::from)?;
        self.physics
            .attachment
            .as_mut()
            .expect("validated committed attachment remains live")
            .properties
            .acknowledge_authored_revision();
        Ok(())
    }

    // Lifetime owner seam: performs identity/publication, lifecycle health,
    // pending body/owner, scene staging, active owner, and supported-root
    // checks before property parsing or backend mutation.
    fn physics_property_admission(
        &self,
        body: ObjectId,
    ) -> Result<PhysicsAttachment, RuntimeError> {
        self.validate_physics_property_access(body)
    }

    // Lifecycle owner seam; mode writes intentionally allow ordinary callbacks.
    fn physics_mode_admission(&self) -> Result<(), RuntimeError> {
        self.validate_physics_mode_access()
    }

    // Lifecycle/staging owner seam used by empty, reserved, and committed sync.
    fn physics_sync_admission(&self) -> Result<(), RuntimeError> {
        self.validate_physics_sync_access()
    }
}
