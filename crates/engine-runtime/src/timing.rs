//! Independently designed native clock for the reviewed additive timing capability.

mod fixed;

use super::RuntimeError;
use std::time::Instant;

const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
const MAX_FIXED_ITERATIONS: u64 = 1024;
const MIN_STEP: f64 = 0.000_001;
const MAX_STEP: f64 = 60.0;

/// Copied native timing values, with no borrowed runtime storage.
#[derive(Clone, Copy, Debug)]
pub struct TimeSnapshot {
    /// Scale, context delta, fixed interval, capture interval, context time and fixed time.
    pub values: [f64; 6],
    /// The actual native lifecycle frame counter.
    pub frame: i32,
    /// Whether the current callback belongs to a fixed iteration.
    pub fixed_context: bool,
}

#[derive(Clone, Debug)]
enum ClockSource {
    Wall { previous: Option<Instant> },
    Explicit { step: f64 },
}

impl ClockSource {
    fn sample(&mut self) -> f64 {
        match self {
            Self::Explicit { step } => *step,
            Self::Wall { previous } => {
                let now = Instant::now();
                let elapsed =
                    previous.map_or(0.02, |instant| now.duration_since(instant).as_secs_f64());
                *previous = Some(now);
                elapsed
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Settings {
    scale: f32,
    fixed_step: f32,
    capture_step: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            fixed_step: 0.02,
            capture_step: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FixedPlan {
    interval_ns: u64,
    step: f32,
    remaining: u64,
}

#[derive(Clone, Copy, Debug)]
enum FixedState {
    Unplanned,
    Ready(FixedPlan),
    Active(FixedPlan),
    Complete,
}

#[derive(Clone, Copy, Debug)]
struct FrameClock {
    time_ns: u64,
    delta: f32,
    paused: bool,
    fixed: FixedState,
}

/// Private storage; scheduling and callback eligibility remain native-owned.
#[derive(Clone, Debug, Default)]
pub(super) struct Clock {
    settings: Settings,
    source: Option<ClockSource>,
    frame: Option<FrameClock>,
    next_fixed_ns: u64,
    last_fixed_ns: u64,
}

fn validate_elapsed(value: f64) -> Result<(), RuntimeError> {
    if !value.is_finite() || !(0.0..=MAX_STEP).contains(&value) {
        return Err(RuntimeError::Numeric);
    }
    Ok(())
}

fn nanoseconds(value: f64) -> Result<u64, RuntimeError> {
    let converted = (value * NANOSECONDS_PER_SECOND).floor();
    if !converted.is_finite() || converted < 0.0 || converted >= u64::MAX as f64 {
        return Err(RuntimeError::Numeric);
    }
    Ok(converted as u64)
}

fn seconds(value: u64) -> f64 {
    value as f64 / NANOSECONDS_PER_SECOND
}

/// Validate scalar timing configuration before resolving a runtime identity.
/// # Errors
/// Returns kind or numeric failure without accessing runtime storage.
pub fn validate_time_configuration(source: i32, step: f64) -> Result<(), RuntimeError> {
    match source {
        0 if step == 0.0 => Ok(()),
        1 if step.is_finite() && (MIN_STEP..=MAX_STEP).contains(&step) => Ok(()),
        0 | 1 => Err(RuntimeError::Numeric),
        _ => Err(RuntimeError::InvalidKind),
    }
}

/// Validate and normalize a native timing setting before runtime lookup.
/// # Errors
/// Returns kind or numeric failure without mutation.
pub fn validate_time_property(property: i32, value: f32) -> Result<f32, RuntimeError> {
    if !(1..=3).contains(&property) {
        return Err(RuntimeError::InvalidKind);
    }
    if !value.is_finite() {
        return Err(RuntimeError::Numeric);
    }
    let normalized = if value == 0.0 { 0.0 } else { value };
    let valid = match property {
        1 => (0.0..=100.0).contains(&normalized),
        2 => (0.000_001_f32..=60.0_f32).contains(&normalized),
        3 => normalized == 0.0 || (0.000_001_f32..=60.0_f32).contains(&normalized),
        _ => false,
    };
    if !valid {
        return Err(RuntimeError::Numeric);
    }
    Ok(normalized)
}

impl Clock {
    pub(super) fn enabled(&self) -> bool {
        self.source.is_some()
    }

    pub(super) fn configure(&mut self, source: i32, step: f64) -> Result<(), RuntimeError> {
        validate_time_configuration(source, step)?;
        if self.enabled() {
            return Err(RuntimeError::Lifecycle);
        }
        let configured = match source {
            0 => ClockSource::Wall { previous: None },
            1 => ClockSource::Explicit { step },
            _ => return Err(RuntimeError::InvalidKind),
        };
        self.source = Some(configured);
        Ok(())
    }

    pub(super) fn set(&mut self, property: i32, value: f32) -> Result<(), RuntimeError> {
        let normalized = validate_time_property(property, value)?;
        match property {
            1 => self.settings.scale = normalized,
            2 => self.settings.fixed_step = normalized,
            3 => self.settings.capture_step = normalized,
            _ => return Err(RuntimeError::InvalidKind),
        }
        Ok(())
    }

    pub(super) fn frame_candidate(&self) -> Result<Self, RuntimeError> {
        if self
            .frame
            .is_some_and(|frame| !matches!(frame.fixed, FixedState::Complete))
        {
            return Err(RuntimeError::Lifecycle);
        }
        let mut candidate = self.clone();
        let elapsed = candidate
            .source
            .as_mut()
            .ok_or(RuntimeError::Lifecycle)?
            .sample();
        let selected = if self.settings.capture_step > 0.0 {
            f64::from(self.settings.capture_step)
        } else {
            elapsed
        };
        validate_elapsed(selected)?;
        let delta_ns = nanoseconds(selected * f64::from(self.settings.scale))?;
        let time_ns = match self.frame {
            Some(frame) => frame
                .time_ns
                .checked_add(delta_ns)
                .ok_or(RuntimeError::Numeric)?,
            None => 0,
        };
        candidate.frame = Some(FrameClock {
            time_ns,
            delta: seconds(delta_ns) as f32,
            paused: self.settings.scale == 0.0,
            fixed: FixedState::Unplanned,
        });
        Ok(candidate)
    }

    pub(super) fn snapshot(&self, frame_count: i32) -> TimeSnapshot {
        let mut delta = self.frame.map_or(0.02, |frame| frame.delta);
        let mut fixed_step = self.settings.fixed_step;
        let mut time_ns = self.frame.map_or(0, |frame| frame.time_ns);
        let fixed_context = self.fixed_active();
        if let Some(FrameClock {
            fixed: FixedState::Active(plan),
            ..
        }) = self.frame
        {
            delta = plan.step;
            fixed_step = plan.step;
            time_ns = self.last_fixed_ns;
        }
        TimeSnapshot {
            values: [
                f64::from(self.settings.scale),
                f64::from(delta),
                f64::from(fixed_step),
                f64::from(self.settings.capture_step),
                seconds(time_ns),
                seconds(self.last_fixed_ns),
            ],
            frame: frame_count,
            fixed_context,
        }
    }
}
