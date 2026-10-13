//! Independent PCM and bounded voice mixer conformance cases.

use std::sync::Arc;

use engine_audio::clip::{PcmClip, PcmClipError, SampleFormat};
use engine_audio::mixer::{MixError, MixFormat, MixedOutput, VoiceMixer, VoiceRequest};

#[test]
fn pcm_clip_preserves_interleaved_frames_and_rejects_invalid_inputs() {
    let clip = PcmClip::from_f32(48_000, 2, &[-1.0, 0.25, 0.5, 1.0]).unwrap();
    assert_eq!(clip.sample_rate_hz(), 48_000);
    assert_eq!(clip.channel_count(), 2);
    assert_eq!(clip.sample_format(), SampleFormat::F32);
    assert_eq!(clip.frame_count(), 2);
    assert_eq!(clip.f32_sample(0, 0), Some(-1.0));
    assert_eq!(clip.f32_sample(0, 1), Some(0.25));
    assert_eq!(clip.f32_sample(1, 0), Some(0.5));
    assert_eq!(clip.f32_sample(1, 1), Some(1.0));
    assert_eq!(clip.f32_sample(2, 0), None);
    assert_eq!(clip.f32_sample(0, 2), None);

    assert!(matches!(
        PcmClip::from_f32(0, 1, &[0.0]),
        Err(PcmClipError::ZeroSampleRate)
    ));
    assert!(matches!(
        PcmClip::from_f32(48_000, 0, &[0.0]),
        Err(PcmClipError::ZeroChannels)
    ));
    assert!(matches!(
        PcmClip::from_f32(48_000, 2, &[]),
        Err(PcmClipError::EmptyData)
    ));
    assert!(matches!(
        PcmClip::from_f32(48_000, 2, &[0.0, 0.0, 0.0]),
        Err(PcmClipError::MisalignedInterleave { .. })
    ));
    assert!(matches!(
        PcmClip::from_f32(48_000, 1, &[f32::NAN]),
        Err(PcmClipError::NonFiniteSample { .. })
    ));
    assert!(matches!(
        PcmClip::from_f32(48_000, 1, &[1.0001]),
        Err(PcmClipError::SampleOutOfRange { .. })
    ));
    let integer = PcmClip::from_i16(48_000, 1, &[-32_768, 0, 32_767]).unwrap();
    assert_eq!(integer.i16_samples(), Some(&[-32_768, 0, 32_767][..]));
}

fn mono_f32(samples: &[f32]) -> Arc<PcmClip> {
    Arc::new(PcmClip::from_f32(48_000, 1, samples).unwrap())
}

fn f32_mixer(capacity: usize) -> VoiceMixer {
    VoiceMixer::new(
        capacity,
        MixFormat::new(48_000, 1, SampleFormat::F32).unwrap(),
    )
    .unwrap()
}

fn full_gain_request(clip: Arc<PcmClip>, start_frame: u64, priority: f32) -> VoiceRequest {
    VoiceRequest {
        clip,
        gain: 1.0,
        start_frame,
        priority,
    }
}

#[test]
fn voice_mixer_clips_once_and_retires_at_source_end() {
    let mut mixer = f32_mixer(2);
    let clip = mono_f32(&[0.75, 0.75]);
    mixer
        .admit(VoiceRequest {
            clip: Arc::clone(&clip),
            gain: 1.0,
            start_frame: 0,
            priority: 1.0,
        })
        .unwrap();
    mixer
        .admit(VoiceRequest {
            clip,
            gain: 1.0,
            start_frame: 0,
            priority: 2.0,
        })
        .unwrap();
    assert_eq!(
        mixer.render(2).unwrap(),
        MixedOutput::F32(vec![1.0, 1.0].into_boxed_slice())
    );
    assert!(mixer.active().is_empty());
}

#[test]
fn voice_mixer_supports_future_starts_and_priority_eviction() {
    let mut mixer = f32_mixer(2);
    let clip = mono_f32(&[0.5, 0.5, 0.5]);
    mixer
        .admit(VoiceRequest {
            clip: Arc::clone(&clip),
            gain: 1.0,
            start_frame: 0,
            priority: 1.0,
        })
        .unwrap();
    let second = mixer
        .admit(VoiceRequest {
            clip: Arc::clone(&clip),
            gain: 1.0,
            start_frame: 2,
            priority: 1.0,
        })
        .unwrap();
    assert!(matches!(
        mixer.admit(VoiceRequest {
            clip: Arc::clone(&clip),
            gain: 1.0,
            start_frame: 0,
            priority: 0.5
        }),
        Err(MixError::CapacityExceeded)
    ));
    let replacement = mixer
        .admit(VoiceRequest {
            clip,
            gain: 1.0,
            start_frame: 0,
            priority: 2.0,
        })
        .unwrap();
    assert_eq!(
        mixer
            .active()
            .iter()
            .map(|voice| voice.id)
            .collect::<Vec<_>>(),
        vec![second, replacement]
    );
    assert_eq!(
        mixer.render(3).unwrap(),
        MixedOutput::F32(vec![0.5, 0.5, 1.0].into_boxed_slice())
    );
    assert_eq!(
        mixer
            .active()
            .iter()
            .map(|voice| voice.id)
            .collect::<Vec<_>>(),
        vec![second]
    );
}

#[test]
fn voice_mixer_rejects_invalid_requests_without_mutation() {
    let mut mixer = f32_mixer(1);
    let clip = mono_f32(&[0.25]);
    assert!(matches!(
        mixer.admit(VoiceRequest {
            clip: Arc::clone(&clip),
            gain: -1.0,
            start_frame: 0,
            priority: 1.0
        }),
        Err(MixError::NegativeGain)
    ));
    assert!(matches!(
        mixer.admit(VoiceRequest {
            clip: Arc::clone(&clip),
            gain: f32::NAN,
            start_frame: 0,
            priority: 1.0
        }),
        Err(MixError::NonFiniteGain)
    ));
    assert!(matches!(
        mixer.admit(VoiceRequest {
            clip,
            gain: 1.0,
            start_frame: 0,
            priority: f32::INFINITY
        }),
        Err(MixError::NonFinitePriority)
    ));
    assert!(mixer.active().is_empty());
}

#[test]
fn voice_mixer_rejects_overflowing_clip_lifetime_before_eviction_or_id_use() {
    let clip = mono_f32(&[0.25]);
    let mut mixer = f32_mixer(1);
    let existing = mixer
        .admit(full_gain_request(Arc::clone(&clip), 0, 1.0))
        .unwrap();
    let voices_before = mixer.active();
    let mut reference = f32_mixer(1);
    let reference_existing = reference
        .admit(full_gain_request(Arc::clone(&clip), 0, 1.0))
        .unwrap();
    assert_eq!(existing, reference_existing);

    let invalid_start_frame = u64::MAX;
    let invalid_priority = 2.0;
    assert!(matches!(
        mixer.admit(full_gain_request(
            Arc::clone(&clip),
            invalid_start_frame,
            invalid_priority,
        )),
        Err(MixError::InvalidStartFrame)
    ));
    assert_eq!(mixer.active(), voices_before);

    let expected_output = MixedOutput::F32(vec![0.25].into_boxed_slice());
    assert_eq!(mixer.render(1).unwrap(), expected_output);
    assert_eq!(reference.render(1).unwrap(), expected_output);
    assert!(mixer.active().is_empty());

    let replacement = mixer
        .admit(full_gain_request(Arc::clone(&clip), 1, 2.0))
        .unwrap();
    let reference_replacement = reference.admit(full_gain_request(clip, 1, 2.0)).unwrap();

    assert_eq!(existing, reference_existing);
    assert_eq!(replacement, reference_replacement);
    assert_eq!(mixer.active(), reference.active());
}

#[test]
fn voice_mixer_accepts_maximum_representable_exclusive_voice_end() {
    let clip = mono_f32(&[0.75]);
    let mut mixer = f32_mixer(1);
    let voice = mixer
        .admit(full_gain_request(clip, u64::MAX - 1, 0.0))
        .unwrap();

    assert_eq!(
        mixer.render(1).unwrap(),
        MixedOutput::F32(vec![0.0].into_boxed_slice())
    );
    assert_eq!(mixer.active()[0].id, voice);
}

#[test]
fn voice_mixer_orders_signed_zero_priorities_as_numeric_ties() {
    let clip = mono_f32(&[0.25]);
    let mut mixer = f32_mixer(1);
    let first = mixer
        .admit(full_gain_request(Arc::clone(&clip), 0, 0.0))
        .unwrap();
    let second = mixer.admit(full_gain_request(clip, 0, -0.0)).unwrap();

    assert!(second > first);
    assert_eq!(mixer.active()[0].id, second);
}

#[test]
fn i16_mixing_uses_endpoints_and_ties_to_even() {
    let format = MixFormat::new(48_000, 1, SampleFormat::I16).unwrap();
    let mut mixer = VoiceMixer::new(1, format).unwrap();
    let clip = Arc::new(PcmClip::from_i16(48_000, 1, &[-32_768, 0, 32_767]).unwrap());
    mixer
        .admit(VoiceRequest {
            clip,
            gain: 1.0,
            start_frame: 0,
            priority: 0.0,
        })
        .unwrap();
    assert_eq!(
        mixer.render(3).unwrap(),
        MixedOutput::I16(vec![-32_768, 0, 32_767].into_boxed_slice())
    );

    let mut mixer = VoiceMixer::new(1, format).unwrap();
    let clip = Arc::new(PcmClip::from_i16(48_000, 1, &[32_767]).unwrap());
    mixer
        .admit(VoiceRequest {
            clip,
            gain: 0.5 / 32_767.0,
            start_frame: 0,
            priority: 0.0,
        })
        .unwrap();
    assert_eq!(
        mixer.render(1).unwrap(),
        MixedOutput::I16(vec![0].into_boxed_slice())
    );
}
