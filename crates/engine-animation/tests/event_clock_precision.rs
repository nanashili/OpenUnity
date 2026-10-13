//! Public clock acceptance for exact intervals and binary32 endpoint rounding.

use engine_animation::{AnimationClock, ClockError, PlaybackMode, ScheduledEvent};

fn event(time: f32, key: &str) -> ScheduledEvent {
    ScheduledEvent {
        time,
        key: key.into(),
    }
}

#[test]
fn duration_above_binary64_precision_retains_cycle_and_head_occurrences() {
    let duration = 2.0_f32.powi(120);
    let mut clock =
        AnimationClock::new(duration, PlaybackMode::Loop, vec![event(0.5, "head")], 8, 2).unwrap();

    assert_eq!(clock.advance(1.0).unwrap(), [event(0.5, "head")]);
    assert_eq!(
        clock.advance(duration * 2.0).unwrap(),
        [event(0.5, "head"), event(0.5, "head")]
    );
    assert_eq!(clock.time(), 1.0);
    assert!(clock.advance(0.0).unwrap().is_empty());
    assert_eq!(clock.time(), 1.0);
}

#[test]
fn interior_halfway_endpoints_round_to_the_even_significand() {
    let unit = f32::EPSILON;
    let halfway = unit / 2.0;
    let first = 1.0 + unit;
    let second = 1.0 + 2.0 * unit;
    let mut clock = AnimationClock::new(
        4.0,
        PlaybackMode::Loop,
        vec![event(first, "first"), event(second, "second")],
        8,
        2,
    )
    .unwrap();

    assert!(clock.advance(1.0).unwrap().is_empty());
    assert!(clock.advance(halfway).unwrap().is_empty());
    assert_eq!(clock.time(), 1.0);

    assert_eq!(clock.advance(unit).unwrap(), [event(first, "first")]);
    assert_eq!(clock.time(), first);
    assert_eq!(clock.advance(halfway).unwrap(), [event(second, "second")]);
    assert_eq!(clock.time(), second);
}

#[test]
fn smallest_normal_boundary_preserves_subnormal_events() {
    let unit = f32::from_bits(1);
    let before_normal = f32::from_bits(f32::MIN_POSITIVE.to_bits() - 1);
    let mut clock = AnimationClock::new(
        f32::MIN_POSITIVE,
        PlaybackMode::Loop,
        vec![
            event(0.0, "zero"),
            event(before_normal, "subnormal"),
            event(f32::MIN_POSITIVE, "normal"),
        ],
        8,
        1,
    )
    .unwrap();

    assert_eq!(
        clock.advance(before_normal).unwrap(),
        [event(before_normal, "subnormal")]
    );
    assert_eq!(clock.time(), before_normal);
    assert_eq!(
        clock.advance(unit).unwrap(),
        [event(f32::MIN_POSITIVE, "normal"), event(0.0, "zero")]
    );
    assert_eq!(clock.time(), 0.0);
}

#[test]
fn maximum_finite_duration_retains_residual_with_maximum_crossing_limit() {
    let mut clock = AnimationClock::new(
        f32::MAX,
        PlaybackMode::Loop,
        vec![event(0.5, "head")],
        8,
        usize::MAX,
    )
    .unwrap();

    assert_eq!(clock.advance(1.0).unwrap(), [event(0.5, "head")]);
    assert_eq!(clock.advance(f32::MAX).unwrap(), [event(0.5, "head")]);
    assert_eq!(clock.time(), 1.0);
    assert!(clock.advance(0.0).unwrap().is_empty());
}

#[test]
fn excessive_subnormal_crossings_fail_without_changing_clock_state() {
    let duration = f32::from_bits(1);
    let mut clock = AnimationClock::new(
        duration,
        PlaybackMode::Loop,
        vec![event(0.0, "zero"), event(duration, "end")],
        8,
        1,
    )
    .unwrap();

    assert_eq!(clock.advance(f32::MAX), Err(ClockError::EventOverflow));
    assert_eq!(clock.time(), 0.0);
    assert_eq!(
        clock.advance(duration).unwrap(),
        [event(duration, "end"), event(0.0, "zero")]
    );
    assert_eq!(clock.time(), 0.0);
}
