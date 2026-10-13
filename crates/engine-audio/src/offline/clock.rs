use std::num::NonZeroU32;

use super::{BlockMixer, DspCommand, OfflineError};

struct Scheduled {
    frame: u64,
    sequence: u64,
    command: DspCommand,
}

/// Integer sample clock with ordered bounded voice commands.
///
/// A mixer rejection while executing a command permanently fails the clock.
pub struct DspClock<Mixer: BlockMixer> {
    sample_rate_hz: NonZeroU32,
    block_size: NonZeroU32,
    frame: u64,
    next_sequence: u64,
    max_operations: usize,
    operations: Vec<Scheduled>,
    failure_latched: bool,
    marker: std::marker::PhantomData<Mixer>,
}

impl<Mixer: BlockMixer> DspClock<Mixer> {
    /// Creates a clock with a positive sample rate and block size.
    pub fn new(
        sample_rate_hz: u32,
        block_size: u32,
        max_operations: usize,
    ) -> Result<Self, OfflineError<Mixer::Error>> {
        let sample_rate_hz = NonZeroU32::new(sample_rate_hz).ok_or(OfflineError::ZeroSampleRate)?;
        let block_size = NonZeroU32::new(block_size).ok_or(OfflineError::ZeroBlockSize)?;
        Ok(Self {
            sample_rate_hz,
            block_size,
            frame: 0,
            next_sequence: 0,
            max_operations,
            operations: Vec::new(),
            failure_latched: false,
            marker: std::marker::PhantomData,
        })
    }

    /// Returns the configured sample rate.
    pub fn sample_rate_hz(&self) -> NonZeroU32 {
        self.sample_rate_hz
    }

    /// Queues a bounded command at an integer frame.
    pub fn schedule(
        &mut self,
        target: u64,
        command: DspCommand,
    ) -> Result<(), OfflineError<Mixer::Error>> {
        self.ensure_active()?;

        if target < self.frame {
            return Err(OfflineError::PastFrame);
        }
        if self.operations.len() >= self.max_operations {
            return Err(OfflineError::OperationOverflow);
        }
        let sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(OfflineError::OperationOverflow)?;
        self.next_sequence = sequence;
        self.operations.push(Scheduled {
            frame: target,
            sequence,
            command,
        });
        self.operations
            .sort_by_key(|item| (item.frame, item.sequence));
        Ok(())
    }

    /// Returns the next exact sub-block before a command boundary.
    pub fn next_segment(&self, requested: u64) -> Result<u32, OfflineError<Mixer::Error>> {
        self.ensure_active()?;

        let block_end = self
            .frame
            .checked_add(u64::from(self.block_size.get()))
            .ok_or(OfflineError::FrameOverflow)?;
        let requested_end = self
            .frame
            .checked_add(requested)
            .ok_or(OfflineError::FrameOverflow)?;
        let end = block_end.min(requested_end);
        let operation_end = self
            .operations
            .first()
            .map_or(end, |item| item.frame.min(end));
        u32::try_from(operation_end - self.frame).map_err(|_| OfflineError::FrameOverflow)
    }

    /// Applies all commands whose target is the current frame.
    ///
    /// A mixer rejection terminally fails the clock. The rejected command stays
    /// queued for diagnosis, but later calls cannot replay it or advance past it.
    pub fn execute_at_current(
        &mut self,
        mixer: &mut Mixer,
    ) -> Result<(), OfflineError<Mixer::Error>> {
        self.ensure_active()?;

        while self
            .operations
            .first()
            .is_some_and(|item| item.frame == self.frame)
        {
            let command = self.operations[0].command;

            if let Err(error) = mixer.apply_command(command) {
                self.failure_latched = true;
                return Err(OfflineError::Mixer(error));
            }

            self.operations.remove(0);
        }

        Ok(())
    }

    /// Advances without passing an unexecuted operation boundary.
    pub fn advance(&mut self, frames: u32) -> Result<(), OfflineError<Mixer::Error>> {
        self.ensure_active()?;

        let next_frame = self
            .frame
            .checked_add(u64::from(frames))
            .ok_or(OfflineError::FrameOverflow)?;

        if self
            .operations
            .iter()
            .any(|operation| operation.frame < next_frame)
        {
            return Err(OfflineError::PendingOperationWouldBeSkipped);
        }

        self.frame = next_frame;
        Ok(())
    }

    fn ensure_active(&self) -> Result<(), OfflineError<Mixer::Error>> {
        if self.failure_latched {
            return Err(OfflineError::FailedRenderer);
        }

        Ok(())
    }
}
