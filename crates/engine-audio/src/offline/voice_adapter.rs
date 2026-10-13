use crate::clip::SampleFormat;
use crate::mixer::{MixError, MixFormat, MixedOutput, VoiceId, VoiceMixer, VoiceRequest};

use super::{BlockMixer, DspCommand, OfflineError, OfflineRenderer};

/// Adapter failures at the bounded offline voice boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceAdapterError {
    /// The adapter accepts only F32 output.
    UnsupportedOutputFormat,
    /// The request rate or channel count differs from the adapter format.
    SourceFormatMismatch,
    /// The prepared request capacity is zero or full.
    PreparedCapacityExceeded,
    /// The external scheduling identity is already reserved.
    DuplicateScheduleId,
    /// The external identity has no prepared request.
    VoiceNotPrepared,
    /// The external identity has no active admitted voice.
    VoiceNotActive,
    /// A start command frame differs from the request start frame.
    StartFrameMismatch,
    /// Checked adapter frame arithmetic overflowed.
    FrameOverflow,
    /// The mixer rejected a request, command, or render.
    Mixer(MixError),
}

/// Owns a reviewed voice mixer and bounded scheduled identity mappings.
pub struct VoiceMixerAdapter {
    mixer: VoiceMixer,
    format: MixFormat,
    max_prepared_voices: usize,
    prepared: Vec<(u64, VoiceRequest)>,
    active: Vec<(u64, VoiceId)>,
    frame: u64,
}

impl VoiceMixerAdapter {
    /// Creates an F32 adapter with bounded active and prepared voice capacity.
    pub fn new(
        max_voices: usize,
        max_prepared_voices: usize,
        format: MixFormat,
    ) -> Result<Self, VoiceAdapterError> {
        if max_prepared_voices == 0 {
            return Err(VoiceAdapterError::PreparedCapacityExceeded);
        }
        if format.sample_format != SampleFormat::F32 {
            return Err(VoiceAdapterError::UnsupportedOutputFormat);
        }
        let mixer = VoiceMixer::new(max_voices, format).map_err(VoiceAdapterError::Mixer)?;
        Ok(Self {
            mixer,
            format,
            max_prepared_voices,
            prepared: Vec::new(),
            active: Vec::new(),
            frame: 0,
        })
    }

    /// Stores one request until its matching start command executes.
    pub fn prepare_voice(
        &mut self,
        schedule_id: u64,
        request: VoiceRequest,
    ) -> Result<(), VoiceAdapterError> {
        self.ensure_format(&request)?;
        if self.prepared.iter().any(|(id, _)| *id == schedule_id)
            || self.active.iter().any(|(id, _)| *id == schedule_id)
        {
            return Err(VoiceAdapterError::DuplicateScheduleId);
        }
        if self.prepared.len() >= self.max_prepared_voices {
            return Err(VoiceAdapterError::PreparedCapacityExceeded);
        }
        self.prepared.push((schedule_id, request));
        Ok(())
    }

    /// Binds this adapter to a fixed-rate offline renderer.
    pub fn into_offline_renderer(
        self,
        block_size: u32,
        max_operations: usize,
        max_output_frames: u64,
    ) -> Result<OfflineRenderer<Self>, OfflineError<VoiceAdapterError>> {
        let sample_rate_hz = self.format.sample_rate_hz;
        OfflineRenderer::new(
            self,
            sample_rate_hz,
            block_size,
            max_operations,
            max_output_frames,
        )
    }

    fn ensure_format(&self, request: &VoiceRequest) -> Result<(), VoiceAdapterError> {
        if request.clip.sample_format() != SampleFormat::F32 {
            return Err(VoiceAdapterError::UnsupportedOutputFormat);
        }
        if request.clip.sample_rate_hz() != self.format.sample_rate_hz
            || request.clip.channel_count() != self.format.channel_count
        {
            return Err(VoiceAdapterError::SourceFormatMismatch);
        }
        Ok(())
    }

    fn start_voice(&mut self, schedule_id: u64) -> Result<(), VoiceAdapterError> {
        let index = self
            .prepared
            .iter()
            .position(|(id, _)| *id == schedule_id)
            .ok_or(VoiceAdapterError::VoiceNotPrepared)?;
        if self.prepared[index].1.start_frame != self.frame {
            return Err(VoiceAdapterError::StartFrameMismatch);
        }
        let request = self.prepared[index].1.clone();
        let voice = self
            .mixer
            .admit(request)
            .map_err(VoiceAdapterError::Mixer)?;
        self.prepared.remove(index);
        self.active.push((schedule_id, voice));
        self.remove_evicted_mappings();
        Ok(())
    }

    fn stop_voice(&mut self, schedule_id: u64) -> Result<(), VoiceAdapterError> {
        let index = self
            .active
            .iter()
            .position(|(id, _)| *id == schedule_id)
            .ok_or(VoiceAdapterError::VoiceNotActive)?;
        self.mixer
            .remove(self.active[index].1)
            .map_err(VoiceAdapterError::Mixer)?;
        self.active.remove(index);
        Ok(())
    }

    fn remove_evicted_mappings(&mut self) {
        let active = self.mixer.active();
        self.active
            .retain(|(_, voice)| active.iter().any(|snapshot| snapshot.id == *voice));
    }
}

impl BlockMixer for VoiceMixerAdapter {
    type Error = VoiceAdapterError;

    fn channels(&self) -> std::num::NonZeroU16 {
        std::num::NonZeroU16::new(self.format.channel_count).expect("validated format channels")
    }

    fn render(&mut self, frames: u32) -> Result<Vec<f32>, Self::Error> {
        let count = usize::try_from(frames).map_err(|_| VoiceAdapterError::FrameOverflow)?;
        let next_frame = self
            .frame
            .checked_add(u64::from(frames))
            .ok_or(VoiceAdapterError::FrameOverflow)?;
        let output = self.mixer.render(count).map_err(VoiceAdapterError::Mixer)?;
        self.frame = next_frame;
        self.remove_evicted_mappings();
        match output {
            MixedOutput::F32(samples) => Ok(samples.into_vec()),
            MixedOutput::I16(_) => Err(VoiceAdapterError::UnsupportedOutputFormat),
        }
    }

    fn apply_command(&mut self, command: DspCommand) -> Result<(), Self::Error> {
        match command {
            DspCommand::StartVoice { voice_id } => self.start_voice(voice_id),
            DspCommand::StopVoice { voice_id } => self.stop_voice(voice_id),
        }
    }
}
