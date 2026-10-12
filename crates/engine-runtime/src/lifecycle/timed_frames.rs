//! Additive clock configuration and timed frame/fixed-step admission.

use crate::{Runtime, RuntimeError, RuntimeInputError, TimeSnapshot};

impl Runtime {
    /// Configure the additive native clock before any frame has run.
    /// # Errors
    /// Rejects invalid scalars, staging, faults, used contexts and reconfiguration.
    pub fn time_configure(&mut self, source: i32, step: f64) -> Result<(), RuntimeError> {
        crate::validate_time_configuration(source, step)?;
        self.time_ready()?;
        if self.lifecycle.frame_count != 0
            || !self.frame_idle()
            || !self.lifecycle.deliveries.is_empty()
        {
            return Err(RuntimeError::Lifecycle);
        }
        self.clock.configure(source, step)
    }

    pub(in crate::lifecycle) fn time_ready(&self) -> Result<(), RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        self.lifecycle_ready()
    }

    /// Update one configured setting without changing an already published frame.
    /// # Errors
    /// Rejects invalid scalars, staging and faulted dispatch without mutation.
    pub fn time_set(&mut self, property: i32, value: f32) -> Result<(), RuntimeError> {
        crate::validate_time_property(property, value)?;
        self.time_ready()?;
        self.clock.set(property, value)
    }

    /// Copy timing values without exposing native storage.
    /// # Errors
    /// Rejects scene staging or faulted dispatch.
    pub fn time_snapshot(&self) -> Result<TimeSnapshot, RuntimeError> {
        self.time_ready()?;
        Ok(self.clock.snapshot(self.lifecycle.frame_count))
    }

    /// Atomically publish one timed frame and increment the actual native counter.
    /// # Errors
    /// Rejects unfinished frames, fixed-body admission, clock arithmetic failure,
    /// faults or count overflow.
    pub fn time_begin_frame(&mut self) -> Result<(), RuntimeInputError> {
        self.time_ready().map_err(RuntimeInputError::Runtime)?;

        // Admission must precede frame-count and wall-clock sampling. A
        // committed dynamic body in Fixed mode therefore leaves every frame
        // and clock field untouched, including the wall-clock source cursor.
        self.physics_fixed_admission()
            .map_err(RuntimeInputError::Runtime)?;

        let count = self
            .next_frame_count()
            .map_err(RuntimeInputError::Runtime)?;
        let clock = self
            .clock
            .frame_candidate()
            .map_err(RuntimeInputError::Runtime)?;

        let prepared_input = self.input.prepare_outer_frame()?;

        self.clock = clock;
        self.input.publish_outer_frame(prepared_input);
        self.commit_frame(count);
        Ok(())
    }

    /// Install one due fixed iteration after the first Start phase has drained.
    /// # Errors
    /// Rejects invalid order, reentry, fixed-body admission, arithmetic failure
    /// or an excessive step budget.
    pub fn time_begin_fixed(&mut self) -> Result<bool, RuntimeError> {
        self.time_ready()?;
        if !self.lifecycle.frame_started
            || self.lifecycle.phase != 1
            || !self.lifecycle.phase_drained
            || !self.lifecycle.deliveries.is_empty()
            || self.lifecycle.standalone_destroy
        {
            return Err(RuntimeError::Lifecycle);
        }

        // Plan validation is pure and checks the complete due-count/budget
        // arithmetic before admission. Admission itself is still required
        // when the validated plan has no due callback, so an empty fixed slot
        // cannot silently advance a committed dynamic body.
        self.clock.fixed_preflight()?;
        self.physics_fixed_admission()?;

        let (clock, active) = self.clock.fixed_candidate()?;
        let snapshot = if active {
            self.ordered_eligible(true)
        } else {
            Vec::new()
        };
        self.clock = clock;
        self.lifecycle.snapshot = snapshot;
        self.lifecycle.cursor = 0;
        Ok(active)
    }
}
