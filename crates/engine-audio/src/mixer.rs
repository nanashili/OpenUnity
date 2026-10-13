//! Bounded, deterministic offline mixing of validated PCM clips.
#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::sync::Arc;

use crate::clip::PcmClip;

mod output;
mod types;
pub use output::MixedOutput;
pub use types::{MixError, MixFormat, VoiceId, VoiceRequest, VoiceSnapshot};

struct Voice {
    id: VoiceId,
    clip: Arc<PcmClip>,
    gain: f32,
    start_frame: u64,
    end_frame_exclusive: u64,
    priority: f32,
}
/// Bounded mixer for deterministic, interleaved PCM voice rendering.
pub struct VoiceMixer {
    format: MixFormat,
    max_voices: usize,
    next_id: u64,
    current_frame: u64,
    voices: Vec<Voice>,
}

impl VoiceMixer {
    /// Creates an empty mixer with a fixed output format and voice capacity.
    pub fn new(max_voices: usize, format: MixFormat) -> Result<Self, MixError> {
        if max_voices == 0 {
            return Err(MixError::ZeroCapacity);
        }
        Ok(Self {
            format,
            max_voices,
            next_id: 1,
            current_frame: 0,
            voices: Vec::with_capacity(max_voices),
        })
    }

    /// Admits a voice, evicting the lowest-ranked active voice when needed.
    pub fn admit(&mut self, request: VoiceRequest) -> Result<VoiceId, MixError> {
        let end_frame_exclusive = validate_request(&request, self.format, self.current_frame)?;
        let id = VoiceId(self.next_id);
        let eviction = self.eviction_index(id, request.priority)?;
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or(MixError::FrameIndexOverflow)?;

        if let Some(index) = eviction {
            self.voices.remove(index);
        }
        self.next_id = next_id;
        self.voices.push(Voice {
            id,
            clip: request.clip,
            gain: request.gain,
            start_frame: request.start_frame,
            end_frame_exclusive,
            priority: request.priority,
        });
        Ok(id)
    }

    /// Removes an active voice by identity.
    pub fn remove(&mut self, id: VoiceId) -> Result<(), MixError> {
        let Some(index) = self.voices.iter().position(|voice| voice.id == id) else {
            return Err(MixError::UnknownVoice(id));
        };
        self.voices.remove(index);
        Ok(())
    }

    /// Returns active voices ordered by their stable identity.
    pub fn active(&self) -> Vec<VoiceSnapshot> {
        let mut active: Vec<_> = self.voices.iter().map(Voice::snapshot).collect();
        active.sort_by_key(|voice| voice.id);
        active
    }

    /// Mixes the next block of frames and advances the mixer clock.
    pub fn render(&mut self, frame_count: usize) -> Result<MixedOutput, MixError> {
        if frame_count == 0 {
            return Err(MixError::ZeroRenderLength);
        }
        let end_frame = self
            .current_frame
            .checked_add(u64::try_from(frame_count).map_err(|_| MixError::FrameIndexOverflow)?)
            .ok_or(MixError::FrameIndexOverflow)?;
        let sample_count = frame_count
            .checked_mul(usize::from(self.format.channel_count))
            .ok_or(MixError::FrameIndexOverflow)?;
        let mut sums = vec![0.0_f64; sample_count];

        for offset in 0..frame_count {
            let frame = self
                .current_frame
                .checked_add(u64::try_from(offset).map_err(|_| MixError::FrameIndexOverflow)?)
                .ok_or(MixError::FrameIndexOverflow)?;
            self.mix_frame(frame, offset, &mut sums)?;
        }
        self.retire_finished(end_frame);
        self.current_frame = end_frame;
        output::finalize(sums, self.format.sample_format)
    }

    fn mix_frame(&self, frame: u64, offset: usize, sums: &mut [f64]) -> Result<(), MixError> {
        let channels = usize::from(self.format.channel_count);
        let mut ordered: Vec<&Voice> = self.voices.iter().collect();
        ordered.sort_by_key(|voice| voice.id);
        for voice in ordered {
            if frame < voice.start_frame {
                continue;
            }
            let source_frame = usize::try_from(frame - voice.start_frame)
                .map_err(|_| MixError::FrameIndexOverflow)?;
            if source_frame >= voice.clip.frame_count() {
                continue;
            }
            for channel in 0..channels {
                let sample = output::source_sample(&voice.clip, source_frame, channel)?;
                let index = offset * channels + channel;
                sums[index] += sample * f64::from(voice.gain);
                if !sums[index].is_finite() {
                    return Err(MixError::NonFiniteOutput);
                }
            }
        }
        Ok(())
    }

    fn retire_finished(&mut self, end_frame: u64) {
        self.voices
            .retain(|voice| voice.end_frame_exclusive > end_frame);
    }

    fn eviction_index(&self, id: VoiceId, priority: f32) -> Result<Option<usize>, MixError> {
        if self.voices.len() < self.max_voices {
            return Ok(None);
        }
        let (index, lowest) = self
            .voices
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                cmp_rank(left.priority, left.id, right.priority, right.id)
            })
            .ok_or(MixError::CapacityExceeded)?;
        let candidate = (priority, id);
        if cmp_rank(lowest.priority, lowest.id, candidate.0, candidate.1) == Ordering::Less {
            Ok(Some(index))
        } else {
            Err(MixError::CapacityExceeded)
        }
    }
}

impl Voice {
    fn snapshot(&self) -> VoiceSnapshot {
        VoiceSnapshot {
            id: self.id,
            start_frame: self.start_frame,
            priority: self.priority,
        }
    }
}

fn cmp_rank(
    left_priority: f32,
    left_id: VoiceId,
    right_priority: f32,
    right_id: VoiceId,
) -> Ordering {
    // Finite numeric equality makes +0.0 and -0.0 tie before identity ordering.
    let priority_order = if left_priority == right_priority {
        Ordering::Equal
    } else {
        left_priority.total_cmp(&right_priority)
    };

    priority_order.then(left_id.cmp(&right_id))
}

fn validate_request(
    request: &VoiceRequest,
    format: MixFormat,
    current_frame: u64,
) -> Result<u64, MixError> {
    if !request.gain.is_finite() {
        return Err(MixError::NonFiniteGain);
    }
    if request.gain < 0.0 {
        return Err(MixError::NegativeGain);
    }
    if !request.priority.is_finite() {
        return Err(MixError::NonFinitePriority);
    }
    if request.start_frame < current_frame {
        return Err(MixError::PastFrame);
    }
    if request.clip.sample_rate_hz() != format.sample_rate_hz
        || request.clip.channel_count() != format.channel_count
        || request.clip.sample_format() != format.sample_format
    {
        return Err(MixError::SourceFormatMismatch);
    }

    let clip_frame_count =
        u64::try_from(request.clip.frame_count()).map_err(|_| MixError::InvalidStartFrame)?;

    request
        .start_frame
        .checked_add(clip_frame_count)
        .ok_or(MixError::InvalidStartFrame)
}
