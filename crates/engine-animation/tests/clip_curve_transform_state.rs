//! Animation clip, curve, transform, state, and clock conformance.

use std::panic::{AssertUnwindSafe, catch_unwind};

use engine_animation::transform_track::{
    TransformBlend, TransformPose, TransformTrack, TransformTrackError, WeightedTrack,
};
use engine_animation::{
    AnimationClip, AnimationClock, AnimationStateId, Channel, ClipDataError, ClockError,
    ConditionKey, Curve, CurveError, Extrapolation, InterpolationMode, InterruptionRule, Keyframe,
    PlaybackMode, ScheduledEvent, StateCommand, StateError, StateGraph, StateMachine, StateRecord,
    Track, TrackId, Transition,
};
use engine_math::Vector;

fn scalar_channel(keys: &[(f32, f32)]) -> Channel<f32> {
    let frames = keys
        .iter()
        .map(|(time, value)| Keyframe::new(*time, *value).expect("fixture key is finite"))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Channel::try_new(frames).expect("fixture channel is ordered and non-empty")
}

fn vector_channel(keys: &[(f32, Vector)]) -> Channel<Vector> {
    let frames = keys
        .iter()
        .map(|(time, value)| Keyframe::new(*time, *value).expect("fixture vector is finite"))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Channel::try_new(frames).expect("fixture channel is ordered and non-empty")
}

#[test]
fn clip_validation_keeps_order_and_rejects_invalid_data() {
    let channel = scalar_channel(&[(0.0, 2.0), (1.0, 8.0)]);
    let clip = AnimationClip::try_new(
        1.0,
        vec![Track::new(TrackId::new(7), channel)].into_boxed_slice(),
    )
    .expect("duration and key interval are valid");
    assert_eq!(clip.duration().seconds(), 1.0);
    assert_eq!(clip.tracks()[0].id().value(), 7);
    assert_eq!(clip.tracks()[0].channel().keys()[1].time_seconds(), 1.0);

    assert!(matches!(
        Channel::try_new(
            vec![
                Keyframe::new(0.5, 1.0).unwrap(),
                Keyframe::new(0.5, 2.0).unwrap(),
            ]
            .into_boxed_slice(),
        ),
        Err(ClipDataError::DuplicateTime { .. })
    ));
    assert!(matches!(
        AnimationClip::<f32>::try_new(1.0, Vec::<Track<f32>>::new().into_boxed_slice()),
        Err(ClipDataError::EmptyTrack)
    ));
    assert!(matches!(
        Keyframe::new(0.0, f32::NAN),
        Err(ClipDataError::NonFiniteValue { .. })
    ));
    assert!(matches!(
        AnimationClip::try_new(
            1.0,
            vec![Track::new(TrackId::new(1), scalar_channel(&[(-0.1, 1.0)]))].into_boxed_slice(),
        ),
        Err(ClipDataError::TimeOutsideClip { .. })
    ));
}

#[test]
fn scalar_curve_has_exact_endpoints_and_explicit_extrapolation() {
    let curve = Curve::<f32>::new(
        scalar_channel(&[(0.0, 0.0), (1.0, 10.0)]),
        InterpolationMode::Linear,
    )
    .expect("linear scalar curve is supported");
    assert_eq!(curve.sample(0.0, Extrapolation::Reject).unwrap(), 0.0);
    assert_eq!(curve.sample(1.0, Extrapolation::Reject).unwrap(), 10.0);
    assert_eq!(curve.sample(0.25, Extrapolation::Reject).unwrap(), 2.5);
    assert_eq!(curve.sample(2.0, Extrapolation::Clamp).unwrap(), 10.0);
    assert_eq!(curve.sample(1.25, Extrapolation::Loop).unwrap(), 2.5);
    assert_eq!(curve.sample(1.25, Extrapolation::PingPong).unwrap(), 7.5);
    assert_eq!(
        curve.sample(1.25, Extrapolation::Reject),
        Err(CurveError::OutOfDomain)
    );
    assert_eq!(
        curve.sample(f32::NAN, Extrapolation::Clamp),
        Err(CurveError::NonFiniteTime)
    );
}

#[test]
fn transform_blend_normalizes_weights_and_preserves_missing_channels() {
    let first = TransformTrack::new(
        TrackId::new(9),
        Some(vector_channel(&[
            (0.0, [0.0, 0.0, 0.0]),
            (1.0, [4.0, 0.0, 0.0]),
        ])),
        None,
        None,
    )
    .unwrap();
    let second = TransformTrack::new(
        TrackId::new(9),
        Some(vector_channel(&[
            (0.0, [10.0, 0.0, 0.0]),
            (1.0, [10.0, 0.0, 0.0]),
        ])),
        None,
        None,
    )
    .unwrap();
    let entries = [
        WeightedTrack {
            track: &first,
            weight: 1.0,
        },
        WeightedTrack {
            track: &second,
            weight: 3.0,
        },
    ];
    let blend = TransformBlend::build(TrackId::new(9), &entries, 1.0).unwrap();
    let base = TransformPose {
        position: [99.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [2.0, 2.0, 2.0],
    };
    let result = blend.apply(TrackId::new(9), base).unwrap();
    assert!((result.position[0] - 8.5).abs() < 1.0e-6);
    assert_eq!(result.scale, base.scale);
    assert_eq!(result.rotation, base.rotation);

    assert!(matches!(
        TransformBlend::build(TrackId::new(9), &[], 0.0),
        Err(TransformTrackError::NoTracks)
    ));
    let invalid = [WeightedTrack {
        track: &first,
        weight: -1.0,
    }];
    assert!(matches!(
        TransformBlend::build(TrackId::new(9), &invalid, 0.0),
        Err(TransformTrackError::NegativeWeight)
    ));
    let foreign = [WeightedTrack {
        track: &first,
        weight: 1.0,
    }];
    let blend = TransformBlend::build(TrackId::new(9), &foreign, 0.0).unwrap();
    assert!(matches!(
        blend.apply(TrackId::new(10), base),
        Err(TransformTrackError::TargetMismatch)
    ));
}

#[test]
fn state_machine_reports_exit_enter_transition_in_order() {
    let idle = AnimationStateId::new(1);
    let run = AnimationStateId::new(2);
    let graph = StateGraph::new(
        vec![idle, run],
        Some(idle),
        vec![Transition {
            source: idle,
            destination: run,
            condition: Some(ConditionKey::new("go")),
            duration: 0.25,
            interruption: InterruptionRule::Allow,
        }],
    )
    .unwrap();
    let mut machine = StateMachine::new(&graph);
    machine
        .command(StateCommand::TransitionTo {
            destination: run,
            condition: Some(ConditionKey::new("go")),
        })
        .unwrap();

    assert_eq!(
        machine.update(),
        vec![
            StateRecord::Exited(idle),
            StateRecord::Entered(run),
            StateRecord::Transitioned {
                source: idle,
                destination: run,
                duration: 0.25
            },
        ]
    );
    assert_eq!(machine.current(), run);
    assert!(matches!(
        StateGraph::new(
            vec![idle],
            Some(idle),
            vec![Transition {
                source: idle,
                destination: AnimationStateId::new(99),
                condition: None,
                duration: 0.0,
                interruption: InterruptionRule::Allow,
            }]
        ),
        Err(StateError::UnknownState(_))
    ));
}

#[test]
fn animation_clock_preserves_boundary_order_and_failed_update_atomicity() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::Loop,
        vec![
            ScheduledEvent {
                time: 1.0,
                key: "tail".into(),
            },
            ScheduledEvent {
                time: 0.0,
                key: "head".into(),
            },
            ScheduledEvent {
                time: 0.5,
                key: "middle".into(),
            },
        ],
        8,
        1,
    )
    .unwrap();
    assert_eq!(clock.advance(0.5).unwrap()[0].key, "middle");
    assert_eq!(
        clock
            .advance(0.6)
            .unwrap()
            .iter()
            .map(|event| event.key.as_str())
            .collect::<Vec<_>>(),
        vec!["tail", "head"]
    );
    assert!((clock.time() - 0.1).abs() < 1.0e-6);

    assert_eq!(clock.advance(2.0), Err(ClockError::EventOverflow));
    assert!((clock.time() - 0.1).abs() < 1.0e-6);
    assert_eq!(clock.advance(-1.0), Err(ClockError::NegativeDelta));
    assert_eq!(clock.advance(f32::NAN), Err(ClockError::NonFiniteDelta));
}

#[test]
fn animation_clock_caps_finite_large_delta_without_partial_delivery() {
    let mut clock = AnimationClock::new(
        0.000001,
        PlaybackMode::Loop,
        vec![ScheduledEvent {
            time: 0.0000005,
            key: "halfway".into(),
        }],
        4,
        1,
    )
    .unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| clock.advance(f32::MAX)));
    assert!(matches!(result, Ok(Err(ClockError::EventOverflow))));
    assert_eq!(clock.time(), 0.0);
    assert_eq!(clock.advance(0.0000005).unwrap()[0].key, "halfway");
}

#[test]
fn animation_clock_rejects_overfull_batches_atomically() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::Loop,
        vec![
            ScheduledEvent {
                time: 0.1,
                key: "first".into(),
            },
            ScheduledEvent {
                time: 0.2,
                key: "second".into(),
            },
            ScheduledEvent {
                time: 0.3,
                key: "third".into(),
            },
        ],
        2,
        1,
    )
    .unwrap();

    assert_eq!(clock.advance(0.5), Err(ClockError::EventOverflow));
    assert_eq!(clock.time(), 0.0);
    assert_eq!(clock.advance(0.1).unwrap()[0].key, "first");
    assert!((clock.time() - 0.1).abs() < 1.0e-6);
}

#[test]
fn animation_clock_repeats_identical_boundary_keys_in_tail_head_order() {
    let mut clock =
        AnimationClock::new(1.0, PlaybackMode::Loop, loop_boundary_events(), 4, 1).unwrap();

    assert_eq!(
        clock
            .advance(1.0)
            .unwrap()
            .iter()
            .map(|event| event.key.as_str())
            .collect::<Vec<_>>(),
        vec!["boundary", "boundary"]
    );
    assert_eq!(
        clock
            .advance(1.0)
            .unwrap()
            .iter()
            .map(|event| event.key.as_str())
            .collect::<Vec<_>>(),
        vec!["boundary", "boundary"]
    );
}

#[test]
fn animation_clock_preserves_authored_order_for_duplicate_timestamps() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::Once,
        vec![
            ScheduledEvent {
                time: 0.5,
                key: "first".into(),
            },
            ScheduledEvent {
                time: 0.5,
                key: "second".into(),
            },
        ],
        4,
        1,
    )
    .unwrap();

    assert_eq!(
        clock
            .advance(0.5)
            .unwrap()
            .iter()
            .map(|event| event.key.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
}

#[test]
fn animation_clock_enforces_exact_crossing_limits_without_mutation() {
    let mut one_crossing = AnimationClock::new(1.0, PlaybackMode::Loop, vec![], 4, 1).unwrap();
    assert!(one_crossing.advance(1.0).is_ok());
    assert_eq!(one_crossing.time(), 0.0);
    assert_eq!(one_crossing.advance(2.0), Err(ClockError::EventOverflow));
    assert_eq!(one_crossing.time(), 0.0);

    let mut two_crossings = AnimationClock::new(1.0, PlaybackMode::Loop, vec![], 4, 2).unwrap();
    assert!(two_crossings.advance(2.0).is_ok());
    assert_eq!(two_crossings.time(), 0.0);

    let mut aggregate =
        AnimationClock::new(1.0, PlaybackMode::Loop, loop_boundary_events(), 1, 1).unwrap();
    assert_eq!(aggregate.advance(1.0), Err(ClockError::EventOverflow));
    assert_eq!(aggregate.time(), 0.0);
}

fn loop_boundary_events() -> Vec<ScheduledEvent> {
    vec![
        ScheduledEvent {
            time: 1.0,
            key: "boundary".into(),
        },
        ScheduledEvent {
            time: 0.0,
            key: "boundary".into(),
        },
    ]
}
