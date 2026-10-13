use std::time::Instant;

const FIXED_STEP_SECONDS: f64 = 1.0 / 60.0;
const MAX_FRAME_DELTA_SECONDS: f64 = 0.25;
const MAX_CATCHUP_STEPS: usize = 4;

/// Converts redraw timestamps into bounded fixed physics steps.
pub(super) struct FixedStepClock {
    previous_redraw: Option<Instant>,
    accumulated_seconds: f64,
}

impl FixedStepClock {
    pub(super) const fn new() -> Self {
        Self {
            previous_redraw: None,
            accumulated_seconds: 0.0,
        }
    }

    pub(super) fn steps_due(&mut self, at: Instant) -> usize {
        let Some(previous_redraw) = self.previous_redraw.replace(at) else {
            return 0;
        };

        self.accumulated_seconds += at
            .saturating_duration_since(previous_redraw)
            .as_secs_f64()
            .min(MAX_FRAME_DELTA_SECONDS);

        let mut step_count = 0;
        while self.accumulated_seconds >= FIXED_STEP_SECONDS && step_count < MAX_CATCHUP_STEPS {
            self.accumulated_seconds -= FIXED_STEP_SECONDS;
            step_count += 1;
        }

        if self.accumulated_seconds >= FIXED_STEP_SECONDS {
            self.accumulated_seconds %= FIXED_STEP_SECONDS;
        }

        step_count
    }

    pub(super) fn reset(&mut self) {
        self.previous_redraw = None;
        self.accumulated_seconds = 0.0;
    }
}
