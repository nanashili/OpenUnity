//! Ordinary lifecycle frame and ordered phase transitions.

use crate::{Runtime, RuntimeError, RuntimeInputError};

impl Runtime {
    /// Read the actual per-context frame counter, initially zero.
    pub fn lifecycle_frame_count(&self) -> i32 {
        self.lifecycle.frame_count
    }

    pub(in crate::lifecycle) fn frame_idle(&self) -> bool {
        !self.lifecycle.frame_started || (self.lifecycle.phase == 5 && self.lifecycle.phase_drained)
    }

    /// Increment the native frame counter and prepare the five ordered phases.
    /// # Errors
    /// Rejects unfinished/reentrant frames, faults, and counter overflow without mutation.
    pub fn lifecycle_begin_frame(&mut self) -> Result<(), RuntimeInputError> {
        self.lifecycle_ready().map_err(RuntimeInputError::Runtime)?;
        if self.clock.enabled() {
            return Err(RuntimeInputError::Runtime(RuntimeError::Lifecycle));
        }
        // Reject a committed Fixed body after the existing clock-mode check,
        // but before frame-count commit or any phase state changes.
        self.physics_fixed_admission()
            .map_err(RuntimeInputError::Runtime)?;
        let count = self
            .next_frame_count()
            .map_err(RuntimeInputError::Runtime)?;

        let prepared_input = self.input.prepare_outer_frame()?;

        self.input.publish_outer_frame(prepared_input);
        self.commit_frame(count);
        Ok(())
    }

    pub(in crate::lifecycle) fn next_frame_count(&self) -> Result<i32, RuntimeError> {
        self.lifecycle_ready()?;
        if !self.frame_idle() || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle
            .frame_count
            .checked_add(1)
            .ok_or(RuntimeError::CountOverflow)
    }

    pub(in crate::lifecycle) fn commit_frame(&mut self, count: i32) {
        self.lifecycle.frame_count = count;
        self.lifecycle.frame_started = true;
        self.lifecycle.phase = 0;
        self.lifecycle.phase_drained = true;
        self.lifecycle.standalone_destroy = false;
    }

    /// Begin the next phase: Start, Update, LateUpdate, Start, destruction.
    /// # Errors
    /// Rejects invalid, reentrant, out-of-order, or undrained phases and faults.
    pub fn lifecycle_begin_phase(&mut self, phase: i32) -> Result<(), RuntimeError> {
        if !(1..=5).contains(&phase) {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle_ready()?;
        if !self.lifecycle.frame_started
            || !self.lifecycle.deliveries.is_empty()
            || !self.lifecycle.phase_drained
            || phase != self.lifecycle.phase + 1
            || self.lifecycle.standalone_destroy
            || (self.clock.enabled() && phase == 2 && !self.clock.fixed_complete())
        {
            return Err(RuntimeError::Lifecycle);
        }
        // Start may commit a body or switch to Fixed after frame begin. Keep
        // this check after all existing phase/order prerequisites, before the
        // phase snapshot and before any delivery token.
        if phase == 2 {
            self.physics_fixed_admission()?;
        }
        let snapshot = if phase == 2 || phase == 3 {
            self.ordered_eligible(true)
        } else {
            Vec::new()
        };
        self.lifecycle.phase = phase;
        self.lifecycle.phase_drained = false;
        self.lifecycle.snapshot = snapshot;
        self.lifecycle.cursor = 0;
        Ok(())
    }

    /// Enter an explicit destruction-only boundary without changing Time.
    /// # Errors
    /// Rejects an unfinished/reentrant frame or faulted dispatch.
    pub fn lifecycle_flush_destroy(&mut self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if !self.frame_idle() || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle.phase = 5;
        self.lifecycle.phase_drained = false;
        self.lifecycle.frame_started = true;
        self.lifecycle.standalone_destroy = true;
        Ok(())
    }
}
