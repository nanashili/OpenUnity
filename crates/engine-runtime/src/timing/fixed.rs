//! Fixed-step planning and candidate clock transitions.

use super::{
    Clock, FixedPlan, FixedState, FrameClock, MAX_FIXED_ITERATIONS, RuntimeError, nanoseconds,
};

impl Clock {
    fn plan_fixed(&self, frame: FrameClock) -> Result<FixedPlan, RuntimeError> {
        let interval_ns = nanoseconds(f64::from(self.settings.fixed_step))?;
        if interval_ns == 0 {
            return Err(RuntimeError::Numeric);
        }
        let remaining = if frame.paused || self.next_fixed_ns > frame.time_ns {
            0
        } else {
            (frame.time_ns - self.next_fixed_ns)
                .checked_div(interval_ns)
                .and_then(|count| count.checked_add(1))
                .ok_or(RuntimeError::Numeric)?
        };
        if remaining > MAX_FIXED_ITERATIONS {
            return Err(RuntimeError::Lifecycle);
        }
        remaining
            .checked_mul(interval_ns)
            .and_then(|advance| self.next_fixed_ns.checked_add(advance))
            .ok_or(RuntimeError::Numeric)?;
        Ok(FixedPlan {
            interval_ns,
            step: self.settings.fixed_step,
            remaining,
        })
    }

    pub(crate) fn fixed_candidate(&self) -> Result<(Self, bool), RuntimeError> {
        let frame = self.frame.ok_or(RuntimeError::Lifecycle)?;
        let plan = match frame.fixed {
            FixedState::Unplanned => self.plan_fixed(frame)?,
            FixedState::Ready(plan) => plan,
            FixedState::Complete => return Ok((self.clone(), false)),
            FixedState::Active(_) => return Err(RuntimeError::Lifecycle),
        };
        let mut candidate = self.clone();
        let mut next_frame = frame;
        if plan.remaining == 0 {
            next_frame.fixed = FixedState::Complete;
            candidate.frame = Some(next_frame);
            return Ok((candidate, false));
        }
        candidate.next_fixed_ns = self
            .next_fixed_ns
            .checked_add(plan.interval_ns)
            .ok_or(RuntimeError::Numeric)?;
        candidate.last_fixed_ns = self.next_fixed_ns;
        next_frame.fixed = FixedState::Active(FixedPlan {
            remaining: plan.remaining - 1,
            ..plan
        });
        candidate.frame = Some(next_frame);
        Ok((candidate, true))
    }

    pub(crate) fn finish_fixed(&mut self) -> Result<(), RuntimeError> {
        let frame = self.frame.as_mut().ok_or(RuntimeError::Lifecycle)?;
        let FixedState::Active(plan) = frame.fixed else {
            return Err(RuntimeError::Lifecycle);
        };
        frame.fixed = FixedState::Ready(plan);
        Ok(())
    }

    pub(crate) fn fixed_active(&self) -> bool {
        self.frame
            .is_some_and(|frame| matches!(frame.fixed, FixedState::Active(_)))
    }

    pub(crate) fn fixed_complete(&self) -> bool {
        self.frame
            .is_some_and(|frame| matches!(frame.fixed, FixedState::Complete))
    }
}
