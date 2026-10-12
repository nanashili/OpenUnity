//! Additive clock configuration and timed frame/fixed-step admission.

use crate::{Runtime, RuntimeError, TimeSnapshot};

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
    /// Rejects unfinished frames, clock arithmetic failure, faults or count overflow.
    pub fn time_begin_frame(&mut self) -> Result<(), RuntimeError> {
        self.time_ready()?;
        let count = self.next_frame_count()?;
        let clock = self.clock.frame_candidate()?;
        self.clock = clock;
        self.commit_frame(count);
        Ok(())
    }

    /// Install one due fixed iteration after the first Start phase has drained.
    /// # Errors
    /// Rejects invalid order, reentry, arithmetic failure or an excessive step budget.
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
