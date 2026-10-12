//! Callback token reservation, delivery ownership and acknowledgement.

use super::LifecycleEvent;
use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    fn validate_delivery_request(&self, mode: i32) -> Result<(), RuntimeError> {
        if !(0..=1).contains(&mode) {
            return Err(RuntimeError::InvalidKind);
        }
        self.lifecycle_ready()?;
        if mode == 1
            && (!self.lifecycle.deliveries.is_empty()
                || !self.lifecycle.frame_started
                || self.lifecycle.phase == 0)
        {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }

    fn next_callback(&mut self, mode: i32) -> Result<Option<(ObjectId, i32)>, RuntimeError> {
        if mode == 1 && self.lifecycle.phase == 5 {
            // Incoming enabled scripts must disable before their destruction.
            self.prepare_destruction()?;
        }
        if let Some(event) = self.mutation_event() {
            return Ok(Some(event));
        }
        if mode == 1 {
            if self.clock.fixed_active() {
                return Ok(self.snapshot_event(8));
            }
            self.phase_event()
        } else {
            Ok(None)
        }
    }

    fn complete_drained_phase(&mut self) -> Result<(), RuntimeError> {
        if self.clock.fixed_active() {
            return self.clock.finish_fixed();
        }
        if self.lifecycle.phase == 5 {
            self.finalize_destruction()?;
        }
        self.lifecycle.phase_drained = true;
        if self.lifecycle.standalone_destroy {
            self.lifecycle.frame_started = false;
            self.lifecycle.standalone_destroy = false;
        }
        Ok(())
    }

    fn commit_delivery(
        &mut self,
        token: ObjectId,
        component: ObjectId,
        event: i32,
    ) -> Result<LifecycleEvent, RuntimeError> {
        let script = self
            .lifecycle
            .scripts
            .get_mut(&component.to_raw())
            .ok_or(RuntimeError::InvalidObject)?;
        match event {
            1 => script.awakened = true,
            2 => script.notified_enabled = true,
            3 => script.started = true,
            6 => script.notified_enabled = false,
            7 => script.notified_destroy = true,
            _ => {}
        }
        script.in_flight = true;
        let delivery = LifecycleEvent {
            token: token.to_raw(),
            component,
            event,
        };
        self.lifecycle.deliveries.push(delivery);
        Ok(delivery)
    }

    fn deliver_reserved_token(
        &mut self,
        mode: i32,
        token: ObjectId,
    ) -> Result<Option<LifecycleEvent>, RuntimeError> {
        let Some((component, event)) = self.next_callback(mode)? else {
            if mode == 1 {
                self.complete_drained_phase()?;
            }
            return Ok(None);
        };
        self.commit_delivery(token, component, event).map(Some)
    }

    fn release_undelivered_token(
        &mut self,
        token: ObjectId,
        outcome: Result<Option<LifecycleEvent>, RuntimeError>,
    ) -> Result<Option<LifecycleEvent>, RuntimeError> {
        if self.lifecycle.tokens.remove(token).is_none() {
            // Only this call owns an undelivered token. A missing token means
            // an internal ownership violation: fault future dispatch while
            // preserving the original operation error when one already exists.
            self.lifecycle.faulted = true;
            return Err(outcome.err().unwrap_or(RuntimeError::Lifecycle));
        }
        outcome
    }

    /// Pull a value-only delivery; mode 0 never advances frame callbacks.
    /// # Errors
    /// Rejects invalid/reentrant frame mode, faults, or exhausted tokens.
    pub fn lifecycle_next(&mut self, mode: i32) -> Result<Option<LifecycleEvent>, RuntimeError> {
        self.validate_delivery_request(mode)?;
        if mode == 1 && self.clock.enabled() {
            return Err(RuntimeError::Lifecycle);
        }
        self.pull_delivery(mode)
    }

    /// Pull a native timed delivery through the existing token ownership protocol.
    /// # Errors
    /// Rejects disabled timing, invalid/reentrant polling, staging and dispatch faults.
    pub fn time_next(&mut self, mode: i32) -> Result<Option<LifecycleEvent>, RuntimeError> {
        self.validate_delivery_request(mode)?;
        self.time_ready()?;
        if !self.clock.enabled() {
            return Err(RuntimeError::Lifecycle);
        }
        self.pull_delivery(mode)
    }

    fn pull_delivery(&mut self, mode: i32) -> Result<Option<LifecycleEvent>, RuntimeError> {
        // Reserve before any phase cursor or destruction mutation. Ownership
        // transfers only when commit_delivery records a delivery for ack.
        let token = self
            .lifecycle
            .tokens
            .insert(())
            .map_err(|_| RuntimeError::IdentityExhausted)?;
        let outcome = self.deliver_reserved_token(mode, token);
        if matches!(outcome, Ok(Some(_))) {
            outcome
        } else {
            self.release_undelivered_token(token, outcome)
        }
    }

    /// Acknowledge the top token once; callback failure permanently faults dispatch.
    /// # Errors
    /// Rejects invalid status, stale/foreign/out-of-order tokens without mutation.
    pub fn lifecycle_ack(&mut self, token: i64, callback_status: i32) -> Result<(), RuntimeError> {
        if callback_status != 0 && callback_status != -6 {
            return Err(RuntimeError::Lifecycle);
        }
        let delivery = self
            .lifecycle
            .deliveries
            .last()
            .filter(|delivery| delivery.token == token)
            .copied()
            .ok_or(RuntimeError::Lifecycle)?;
        let script = self
            .lifecycle
            .scripts
            .get_mut(&delivery.component.to_raw())
            .ok_or(RuntimeError::Lifecycle)?;
        let token_id = ObjectId::from_raw(token).ok_or(RuntimeError::Lifecycle)?;
        self.lifecycle
            .tokens
            .remove(token_id)
            .ok_or(RuntimeError::Lifecycle)?;
        script.in_flight = false;
        self.lifecycle
            .deliveries
            .truncate(self.lifecycle.deliveries.len() - 1);
        self.lifecycle.faulted |= callback_status == -6;
        Ok(())
    }
}
