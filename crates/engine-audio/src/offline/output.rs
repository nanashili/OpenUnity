use std::num::{NonZeroU16, NonZeroU32};
use std::sync::Arc;

/// Immutable interleaved output from a completed offline render.
#[derive(Clone, Debug, PartialEq)]
pub struct OfflineOutput {
    sample_rate_hz: NonZeroU32,
    channels: NonZeroU16,
    samples: Box<[f32]>,
}

impl OfflineOutput {
    /// Returns the output sample rate.
    pub fn sample_rate_hz(&self) -> NonZeroU32 {
        self.sample_rate_hz
    }

    /// Returns the interleaved channel count.
    pub fn channels(&self) -> NonZeroU16 {
        self.channels
    }

    /// Returns the exact rendered frame count.
    pub fn frame_count(&self) -> u64 {
        self.samples.len() as u64 / u64::from(self.channels.get())
    }

    /// Borrows the immutable interleaved samples.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

pub(crate) struct OutputSink {
    sample_rate_hz: NonZeroU32,
    channels: NonZeroU16,
    pub(crate) max_frames: u64,
    samples: Vec<f32>,
}

impl OutputSink {
    pub(crate) fn new(rate: NonZeroU32, channels: NonZeroU16, max_frames: u64) -> Self {
        Self {
            sample_rate_hz: rate,
            channels,
            max_frames,
            samples: Vec::new(),
        }
    }

    pub(crate) fn append(&mut self, block: &[f32]) -> Result<(), ()> {
        let channels = usize::from(self.channels.get());
        if block.is_empty() || block.len() % channels != 0 || block.iter().any(|v| !v.is_finite()) {
            return Err(());
        }
        let frames = u64::try_from(block.len() / channels).map_err(|_| ())?;
        let current = u64::try_from(self.samples.len() / channels).map_err(|_| ())?;
        if current
            .checked_add(frames)
            .is_none_or(|total| total > self.max_frames)
        {
            return Err(());
        }
        self.samples.extend_from_slice(block);
        Ok(())
    }

    pub(crate) fn publish(self, frames: u64) -> Result<Arc<OfflineOutput>, ()> {
        if self.samples.len() as u64 / u64::from(self.channels.get()) != frames {
            return Err(());
        }
        Ok(Arc::new(OfflineOutput {
            sample_rate_hz: self.sample_rate_hz,
            channels: self.channels,
            samples: self.samples.into_boxed_slice(),
        }))
    }
}
