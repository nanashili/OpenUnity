//! Public regression cases for rounded Loop traversal planning.

use engine_animation::{AnimationClock, ClockError, PlaybackMode, ScheduledEvent};

#[test]
fn rounded_endpoint_delivers_tail_then_head_once() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::Loop,
        vec![
            ScheduledEvent {
                time: 0.0,
                key: "zero".into(),
            },
            ScheduledEvent {
                time: 1.0,
                key: "end".into(),
            },
        ],
        8,
        8,
    )
    .unwrap();

    assert!(clock.advance(0.1).unwrap().is_empty());

    let crossed = clock.advance(0.9).unwrap();
    let keys: Vec<_> = crossed.iter().map(|event| event.key.as_str()).collect();
    assert_eq!(keys, ["end", "zero"]);
    assert_eq!(clock.time(), 0.0);

    assert!(clock.advance(0.0).unwrap().is_empty());
    assert_eq!(clock.time(), 0.0);
    assert!(clock.advance(0.0).unwrap().is_empty());
    assert_eq!(clock.time(), 0.0);
}

#[test]
fn equal_zero_times_preserve_authored_order() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::Loop,
        vec![
            ScheduledEvent {
                time: 0.0,
                key: "positive-zero".into(),
            },
            ScheduledEvent {
                time: 1.0,
                key: "end".into(),
            },
            ScheduledEvent {
                time: -0.0_f32,
                key: "negative-zero".into(),
            },
        ],
        8,
        8,
    )
    .unwrap();

    let crossed = clock.advance(1.0).unwrap();
    let keys: Vec<_> = crossed.iter().map(|event| event.key.as_str()).collect();
    assert_eq!(keys, ["end", "positive-zero", "negative-zero"]);
}

#[test]
fn rejected_multi_boundary_update_preserves_rounded_boundary_delivery() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::Loop,
        vec![
            ScheduledEvent {
                time: 0.0,
                key: "zero".into(),
            },
            ScheduledEvent {
                time: 1.0,
                key: "end".into(),
            },
        ],
        8,
        1,
    )
    .unwrap();

    assert!(clock.advance(0.1).unwrap().is_empty());
    assert_eq!(clock.advance(1.9), Err(ClockError::EventOverflow));
    assert_eq!(clock.time(), 0.1);

    let crossed = clock.advance(0.9).unwrap();
    let keys: Vec<_> = crossed.iter().map(|event| event.key.as_str()).collect();
    assert_eq!(keys, ["end", "zero"]);
    assert_eq!(clock.time(), 0.0);
}

#[test]
fn large_duration_preserves_the_post_boundary_head_interval() {
    let duration = 1_073_741_824.0_f32;
    let mut clock = AnimationClock::new(
        duration,
        PlaybackMode::Loop,
        vec![ScheduledEvent {
            time: 0.5,
            key: "head".into(),
        }],
        8,
        1,
    )
    .unwrap();

    let initial_events = clock.advance(1.0).unwrap();

    assert_eq!(initial_events.len(), 1);
    assert_eq!(initial_events[0].time, 0.5);
    assert_eq!(initial_events[0].key, "head");
    assert_eq!(clock.time(), 1.0);

    let crossed = clock.advance(duration).unwrap();

    assert_eq!(crossed.len(), 1);
    assert_eq!(crossed[0].key, "head");
    assert_eq!(crossed[0].time, 0.5);
    assert_eq!(clock.time(), 1.0);
}

#[test]
fn exact_large_residual_delivers_full_cycle_and_head_events() {
    let duration = 2.0_f32.powi(100);
    let mut clock = AnimationClock::new(
        duration,
        PlaybackMode::Loop,
        vec![ScheduledEvent {
            time: 0.5,
            key: "half".into(),
        }],
        8,
        2,
    )
    .unwrap();

    let initial_events = clock.advance(1.0).unwrap();

    assert_eq!(initial_events.len(), 1);
    assert_eq!(initial_events[0].time, 0.5);
    assert_eq!(initial_events[0].key, "half");
    assert_eq!(clock.time(), 1.0);

    let traversed = clock.advance(duration * 2.0).unwrap();
    let occurrences: Vec<_> = traversed
        .iter()
        .map(|event| (event.time, event.key.as_str()))
        .collect();

    assert_eq!(occurrences, [(0.5, "half"), (0.5, "half")]);
    assert_eq!(clock.time(), 1.0);
}

#[test]
fn ping_pong_multi_turnaround_order_and_failure_are_atomic() {
    let mut clock = AnimationClock::new(
        1.0,
        PlaybackMode::PingPong,
        vec![
            ScheduledEvent {
                time: 0.0,
                key: "zero".into(),
            },
            ScheduledEvent {
                time: 0.25,
                key: "quarter".into(),
            },
            ScheduledEvent {
                time: 0.75,
                key: "three-quarter".into(),
            },
            ScheduledEvent {
                time: 1.0,
                key: "end".into(),
            },
        ],
        16,
        2,
    )
    .unwrap();

    let traversed = clock.advance(2.25).unwrap();
    let occurrences: Vec<_> = traversed
        .iter()
        .map(|event| (event.time, event.key.as_str()))
        .collect();

    assert_eq!(
        occurrences,
        [
            (0.25, "quarter"),
            (0.75, "three-quarter"),
            (1.0, "end"),
            (0.75, "three-quarter"),
            (0.25, "quarter"),
            (0.0, "zero"),
            (0.25, "quarter"),
        ]
    );
    assert_eq!(clock.time(), 0.25);

    assert_eq!(clock.advance(3.0), Err(ClockError::EventOverflow));
    assert_eq!(clock.time(), 0.25);

    let after_failure = clock.advance(0.5).unwrap();
    assert_eq!(after_failure.len(), 1);
    assert_eq!(after_failure[0].time, 0.75);
    assert_eq!(after_failure[0].key, "three-quarter");
    assert_eq!(clock.time(), 0.75);
}
