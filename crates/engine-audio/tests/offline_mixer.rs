//! Independent offline DSP conformance cases.

use std::num::NonZeroU16;
use std::sync::Arc;

use engine_audio::clip::{PcmClip, SampleFormat};
use engine_audio::mixer::{MixFormat, VoiceRequest};
use engine_audio::offline::{
    BlockMixer, DspClock, DspCommand, OfflineError, OfflineRenderer, VoiceAdapterError,
    VoiceMixerAdapter,
};

struct CommandMixer {
    active: bool,
    level: f32,
}

impl BlockMixer for CommandMixer {
    type Error = &'static str;

    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(1).unwrap()
    }

    fn render(&mut self, frames: u32) -> Result<Vec<f32>, Self::Error> {
        let level = if self.active { self.level } else { 0.0 };
        Ok(vec![level; frames as usize])
    }

    fn apply_command(&mut self, command: DspCommand) -> Result<(), Self::Error> {
        match command {
            DspCommand::StartVoice { voice_id } => {
                self.active = true;
                self.level = if voice_id == 7 { 0.25 } else { -0.5 };
            }
            DspCommand::StopVoice { .. } => self.active = false,
        }
        Ok(())
    }
}

fn renderer(max_output_frames: u64) -> OfflineRenderer<CommandMixer> {
    OfflineRenderer::new(
        CommandMixer {
            active: false,
            level: 0.0,
        },
        48_000,
        4,
        8,
        max_output_frames,
    )
    .unwrap()
}

fn voice_format() -> MixFormat {
    MixFormat::new(48_000, 1, SampleFormat::F32).unwrap()
}

fn voice_request(samples: &[f32], start_frame: u64) -> VoiceRequest {
    VoiceRequest {
        clip: Arc::new(PcmClip::from_f32(48_000, 1, samples).unwrap()),
        gain: 1.0,
        start_frame,
        priority: 1.0,
    }
}

#[test]
fn offline_renderer_splits_at_integer_commands_and_finishes_once() {
    let mut renderer = renderer(16);
    renderer
        .schedule(0, DspCommand::StartVoice { voice_id: 7 })
        .unwrap();
    renderer
        .schedule(2, DspCommand::StopVoice { voice_id: 7 })
        .unwrap();
    renderer
        .schedule(2, DspCommand::StartVoice { voice_id: 7 })
        .unwrap();
    renderer
        .schedule(4, DspCommand::StartVoice { voice_id: 9 })
        .unwrap();

    renderer.render(6).unwrap();
    let output = renderer.finish().unwrap();
    assert_eq!(output.frame_count(), 6);
    assert_eq!(output.samples(), &[0.25, 0.25, 0.25, 0.25, -0.5, -0.5]);
    assert!(Arc::ptr_eq(&output, &renderer.finish().unwrap()));
}

#[test]
fn offline_renderer_preflights_output_limit_before_mixer_mutation() {
    let mut renderer = renderer(1);
    renderer
        .schedule(0, DspCommand::StartVoice { voice_id: 7 })
        .unwrap();
    assert_eq!(renderer.render(2), Err(OfflineError::OutputLimitExceeded));
    renderer.render(1).unwrap();
    assert_eq!(renderer.finish().unwrap().samples(), &[0.25]);
}

#[test]
fn offline_renderer_enforces_frame_lifecycle() {
    let mut renderer = renderer(16);
    renderer.render(1).unwrap();
    assert_eq!(
        renderer.schedule(0, DspCommand::StartVoice { voice_id: 1 }),
        Err(OfflineError::PastFrame)
    );
    renderer.finish().unwrap();
    assert_eq!(renderer.render(1), Err(OfflineError::AlreadyFinished));
}

struct FailingMixer;

impl BlockMixer for FailingMixer {
    type Error = &'static str;

    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(1).unwrap()
    }

    fn render(&mut self, _frames: u32) -> Result<Vec<f32>, Self::Error> {
        Err("mixer failed")
    }

    fn apply_command(&mut self, _command: DspCommand) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn offline_renderer_retains_terminal_mixer_cause() {
    let mut renderer = OfflineRenderer::new(FailingMixer, 48_000, 4, 2, 8).unwrap();
    assert_eq!(renderer.render(1), Err(OfflineError::Mixer("mixer failed")));
    assert_eq!(
        renderer.failure(),
        Some(&OfflineError::Mixer("mixer failed"))
    );
    assert_eq!(renderer.render(1), Err(OfflineError::FailedRenderer));
    assert_eq!(renderer.finish(), Err(OfflineError::FailedRenderer));
}

#[test]
fn voice_adapter_renders_scheduled_ids_through_offline_sink() {
    let mut adapter = VoiceMixerAdapter::new(2, 2, voice_format()).unwrap();
    adapter
        .prepare_voice(11, voice_request(&[0.25, 0.5, 0.75, 0.9], 1))
        .unwrap();
    adapter
        .prepare_voice(22, voice_request(&[-0.5, 0.25], 3))
        .unwrap();
    let mut renderer = adapter.into_offline_renderer(4, 8, 8).unwrap();
    renderer
        .schedule(1, DspCommand::StartVoice { voice_id: 11 })
        .unwrap();
    renderer
        .schedule(3, DspCommand::StopVoice { voice_id: 11 })
        .unwrap();
    renderer
        .schedule(3, DspCommand::StartVoice { voice_id: 22 })
        .unwrap();

    renderer.render(5).unwrap();
    let output = renderer.finish().unwrap();
    assert_eq!(output.sample_rate_hz().get(), 48_000);
    assert_eq!(output.channels().get(), 1);
    assert_eq!(output.frame_count(), 5);
    assert_eq!(output.samples(), &[0.0, 0.25, 0.5, -0.5, 0.25]);
}

#[test]
fn voice_adapter_duplicate_stop_fails_when_command_executes() {
    let mut adapter = VoiceMixerAdapter::new(1, 1, voice_format()).unwrap();
    adapter
        .prepare_voice(11, voice_request(&[0.5, 0.75], 0))
        .unwrap();
    let mut renderer = adapter.into_offline_renderer(4, 8, 8).unwrap();
    renderer
        .schedule(0, DspCommand::StartVoice { voice_id: 11 })
        .unwrap();
    renderer.render(1).unwrap();
    renderer
        .schedule(1, DspCommand::StopVoice { voice_id: 11 })
        .unwrap();
    renderer.render(1).unwrap();
    renderer
        .schedule(2, DspCommand::StopVoice { voice_id: 11 })
        .unwrap();

    assert_eq!(
        renderer.render(1),
        Err(OfflineError::Mixer(VoiceAdapterError::VoiceNotActive))
    );
    assert_eq!(
        renderer.failure(),
        Some(&OfflineError::Mixer(VoiceAdapterError::VoiceNotActive))
    );
    assert_eq!(renderer.finish(), Err(OfflineError::FailedRenderer));
}

#[test]
fn voice_adapter_preserves_prepared_request_after_frame_mismatch() {
    let mut adapter = VoiceMixerAdapter::new(1, 1, voice_format()).unwrap();
    adapter
        .prepare_voice(11, voice_request(&[0.5, 0.75], 1))
        .unwrap();

    assert_eq!(
        adapter.apply_command(DspCommand::StartVoice { voice_id: 11 }),
        Err(VoiceAdapterError::StartFrameMismatch)
    );
    assert_eq!(adapter.render(1).unwrap(), vec![0.0]);
    adapter
        .apply_command(DspCommand::StartVoice { voice_id: 11 })
        .unwrap();
    assert_eq!(adapter.render(1).unwrap(), vec![0.5]);
}

#[test]
fn voice_adapter_mismatch_latches_renderer_failure() {
    let mut adapter = VoiceMixerAdapter::new(1, 1, voice_format()).unwrap();
    adapter.prepare_voice(11, voice_request(&[0.5], 1)).unwrap();
    let mut renderer = adapter.into_offline_renderer(4, 8, 8).unwrap();
    renderer
        .schedule(0, DspCommand::StartVoice { voice_id: 11 })
        .unwrap();

    assert_eq!(
        renderer.render(1),
        Err(OfflineError::Mixer(VoiceAdapterError::StartFrameMismatch))
    );
    assert_eq!(
        renderer.failure(),
        Some(&OfflineError::Mixer(VoiceAdapterError::StartFrameMismatch))
    );
    assert_eq!(renderer.render(1), Err(OfflineError::FailedRenderer));
    assert_eq!(
        renderer.schedule(1, DspCommand::StartVoice { voice_id: 11 }),
        Err(OfflineError::FailedRenderer)
    );
    assert_eq!(renderer.finish(), Err(OfflineError::FailedRenderer));
}

#[test]
fn dsp_clock_preserves_pending_boundary_when_advance_would_skip_it() {
    let mut clock = DspClock::<CommandMixer>::new(48_000, 4, 4).unwrap();
    clock
        .schedule(1, DspCommand::StartVoice { voice_id: 7 })
        .unwrap();

    assert_eq!(
        clock.advance(2),
        Err(OfflineError::PendingOperationWouldBeSkipped)
    );
    assert_eq!(clock.next_segment(2).unwrap(), 1);

    clock.advance(1).unwrap();
    assert_eq!(clock.next_segment(1).unwrap(), 0);
    let mut mixer = CommandMixer {
        active: false,
        level: 0.0,
    };
    clock.execute_at_current(&mut mixer).unwrap();
    assert_eq!(mixer.render(1).unwrap(), vec![0.25]);
}

struct MutatingFailureMixer {
    command_attempts: usize,
}

impl BlockMixer for MutatingFailureMixer {
    type Error = &'static str;

    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(1).unwrap()
    }

    fn render(&mut self, frames: u32) -> Result<Vec<f32>, Self::Error> {
        Ok(vec![0.0; frames as usize])
    }

    fn apply_command(&mut self, _command: DspCommand) -> Result<(), Self::Error> {
        self.command_attempts += 1;
        Err("command partially applied")
    }
}

#[test]
fn dsp_clock_latches_partially_applied_command_failure() {
    let mut clock = DspClock::<MutatingFailureMixer>::new(48_000, 4, 4).unwrap();
    let mut mixer = MutatingFailureMixer {
        command_attempts: 0,
    };

    clock
        .schedule(0, DspCommand::StartVoice { voice_id: 7 })
        .unwrap();

    assert_eq!(
        clock.execute_at_current(&mut mixer),
        Err(OfflineError::Mixer("command partially applied"))
    );
    assert_eq!(mixer.command_attempts, 1);
    assert_eq!(clock.next_segment(1), Err(OfflineError::FailedRenderer));
    assert_eq!(clock.advance(1), Err(OfflineError::FailedRenderer));
    assert_eq!(
        clock.schedule(0, DspCommand::StopVoice { voice_id: 7 }),
        Err(OfflineError::FailedRenderer)
    );
    assert_eq!(
        clock.execute_at_current(&mut mixer),
        Err(OfflineError::FailedRenderer)
    );
    assert_eq!(mixer.command_attempts, 1);
}
