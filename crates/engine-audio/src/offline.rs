//! Deterministic block-clock scheduling and bounded offline rendering.

use std::num::NonZeroU16;
use std::sync::Arc;

mod clock;
mod output;
mod voice_adapter;
mod wav_playback;
pub use clock::DspClock;
pub use output::OfflineOutput;
use output::OutputSink;
pub use voice_adapter::{VoiceAdapterError, VoiceMixerAdapter};
pub use wav_playback::{WavOfflineError, render_f32_wav_offline};

/// Mixer boundary used by the offline renderer.
pub trait BlockMixer {
    /// Mixer-specific failure type.
    type Error;

    /// Returns the fixed interleaved channel count.
    fn channels(&self) -> NonZeroU16;
    /// Renders one positive block of frames.
    fn render(&mut self, frames: u32) -> Result<Vec<f32>, Self::Error>;
    /// Applies one bounded voice lifecycle command.
    fn apply_command(&mut self, command: DspCommand) -> Result<(), Self::Error>;
}

/// The bounded voice lifecycle commands accepted by the DSP clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DspCommand {
    /// Starts an admitted voice at the scheduled frame.
    StartVoice {
        /// Stable identifier of the voice to start.
        voice_id: u64,
    },
    /// Stops an admitted voice at the scheduled frame.
    StopVoice {
        /// Stable identifier of the voice to stop.
        voice_id: u64,
    },
}

/// Typed scheduling, rendering, sink, and mixer failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OfflineError<E> {
    /// Sample rate was zero.
    ZeroSampleRate,
    /// Block size was zero.
    ZeroBlockSize,
    /// Target frame was before the current clock frame.
    PastFrame,
    /// Advancing would pass an unexecuted scheduled operation.
    PendingOperationWouldBeSkipped,
    /// Frame arithmetic overflowed.
    FrameOverflow,
    /// Render length was zero.
    ZeroRenderLength,
    /// Operation capacity or sequence numbering overflowed.
    OperationOverflow,
    /// The sink rejected a block or publication.
    SinkFailure,
    /// The cumulative output limit would be exceeded.
    OutputLimitExceeded,
    /// Publication already completed.
    AlreadyFinished,
    /// A prior terminal failure prevents further work.
    FailedRenderer,
    /// The mixer rejected a command or render operation.
    Mixer(E),
}

/// Owns the clock, mixer, bounded sink, and publication lifecycle.
pub struct OfflineRenderer<Mixer: BlockMixer> {
    clock: DspClock<Mixer>,
    mixer: Mixer,
    sink: OutputSink,
    rendered_frames: u64,
    published: Option<Arc<OfflineOutput>>,
    failure: Option<OfflineError<Mixer::Error>>,
}

impl<Mixer: BlockMixer> OfflineRenderer<Mixer>
where
    Mixer::Error: Clone,
{
    /// Creates an active renderer with positive format and output bounds.
    pub fn new(
        mixer: Mixer,
        sample_rate_hz: u32,
        block_size: u32,
        max_operations: usize,
        max_output_frames: u64,
    ) -> Result<Self, OfflineError<Mixer::Error>> {
        if max_output_frames == 0 {
            return Err(OfflineError::SinkFailure);
        }
        let clock = DspClock::new(sample_rate_hz, block_size, max_operations)?;
        let sink = OutputSink::new(clock.sample_rate_hz(), mixer.channels(), max_output_frames);
        Ok(Self {
            clock,
            mixer,
            sink,
            rendered_frames: 0,
            published: None,
            failure: None,
        })
    }

    /// Returns the first terminal failure, if one occurred.
    pub fn failure(&self) -> Option<&OfflineError<Mixer::Error>> {
        self.failure.as_ref()
    }

    fn record_failure(&mut self, error: OfflineError<Mixer::Error>) -> OfflineError<Mixer::Error> {
        if self.failure.is_none() {
            self.failure = Some(error.clone());
        }
        error
    }

    /// Schedules one bounded voice lifecycle command at an integer frame.
    pub fn schedule(
        &mut self,
        frame: u64,
        command: DspCommand,
    ) -> Result<(), OfflineError<Mixer::Error>> {
        if self.failure.is_some() {
            return Err(OfflineError::FailedRenderer);
        }
        if self.published.is_some() {
            return Err(OfflineError::AlreadyFinished);
        }
        self.clock.schedule(frame, command)
    }

    /// Renders frames while splitting blocks at scheduled command boundaries.
    pub fn render(&mut self, frames: u64) -> Result<(), OfflineError<Mixer::Error>> {
        self.admit_render(frames)?;
        self.render_segments(frames)
    }

    fn admit_render(&self, frames: u64) -> Result<(), OfflineError<Mixer::Error>> {
        if self.failure.is_some() {
            return Err(OfflineError::FailedRenderer);
        }
        if self.published.is_some() {
            return Err(OfflineError::AlreadyFinished);
        }
        if frames == 0 {
            return Err(OfflineError::ZeroRenderLength);
        }
        let requested_end = self
            .rendered_frames
            .checked_add(frames)
            .ok_or(OfflineError::FrameOverflow)?;
        if requested_end > self.sink.max_frames {
            return Err(OfflineError::OutputLimitExceeded);
        }
        Ok(())
    }

    fn render_segments(&mut self, frames: u64) -> Result<(), OfflineError<Mixer::Error>> {
        let mut remaining = frames;
        while remaining > 0 {
            let segment = self.render_next_segment(remaining)?;
            if segment == 0 {
                continue;
            }
            remaining -= u64::from(segment);
        }
        Ok(())
    }

    fn render_next_segment(&mut self, remaining: u64) -> Result<u32, OfflineError<Mixer::Error>> {
        if let Err(error) = self.clock.execute_at_current(&mut self.mixer) {
            return Err(self.record_failure(error));
        }
        let segment = match self.clock.next_segment(remaining) {
            Ok(segment) => segment,
            Err(error) => return Err(self.record_failure(error)),
        };
        if segment == 0 {
            return Ok(0);
        }

        let block = match self.mixer.render(segment) {
            Ok(block) => block,
            Err(error) => return Err(self.record_failure(OfflineError::Mixer(error))),
        };
        if self.sink.append(&block).is_err() {
            return Err(self.record_failure(OfflineError::SinkFailure));
        }
        if let Err(error) = self.clock.advance(segment) {
            return Err(self.record_failure(error));
        }
        self.rendered_frames = match self.rendered_frames.checked_add(u64::from(segment)) {
            Some(total) => total,
            None => return Err(self.record_failure(OfflineError::FrameOverflow)),
        };
        Ok(segment)
    }

    /// Publishes once and returns the same immutable output handle thereafter.
    pub fn finish(&mut self) -> Result<Arc<OfflineOutput>, OfflineError<Mixer::Error>> {
        if self.failure.is_some() {
            return Err(OfflineError::FailedRenderer);
        }
        if let Some(output) = &self.published {
            return Ok(Arc::clone(output));
        }
        let sink = std::mem::replace(
            &mut self.sink,
            OutputSink::new(
                self.clock.sample_rate_hz(),
                self.mixer.channels(),
                self.rendered_frames,
            ),
        );
        match sink.publish(self.rendered_frames) {
            Ok(output) => {
                self.published = Some(Arc::clone(&output));
                Ok(output)
            }
            Err(()) => Err(self.record_failure(OfflineError::SinkFailure)),
        }
    }
}
