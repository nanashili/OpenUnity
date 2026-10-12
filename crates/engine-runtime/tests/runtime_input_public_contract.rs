//! Public conformance cases for the reviewed Runtime input boundary.
//!
//! This file is intended to become an integration test after the approved
//! `Runtime` input methods land. It deliberately imports only public APIs.

use engine_identifiers::allocate_next;
use engine_input::{
    ActionName, Binding, ControlDescriptor, ControlKind, DeviceDescriptor, DeviceKind, EventError,
    InputEvent, InputEventKind, InputValue, Trigger,
};
use engine_runtime::{Runtime, RuntimeError, RuntimeInputError};

fn device(name: &str) -> DeviceDescriptor {
    DeviceDescriptor {
        id: allocate_next().expect("test device identity allocation"),
        name: name.to_owned(),
        kind: DeviceKind::Keyboard,
        controls: vec![ControlDescriptor {
            path: "button".to_owned(),
            kind: ControlKind::Button,
        }],
    }
}
fn connected(sequence: u64, descriptor: &DeviceDescriptor) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence * 100,
        kind: InputEventKind::Connected(descriptor.clone()),
    }
}
fn changed(sequence: u64, device: engine_input::DeviceId, value: bool) -> InputEvent {
    changed_at(sequence, sequence * 100, device, value)
}
fn changed_at(
    sequence: u64,
    timestamp_ns: u64,
    device: engine_input::DeviceId,
    value: bool,
) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns,
        kind: InputEventKind::Changed {
            device,
            control: "button".to_owned(),
            value: InputValue::Button(value),
        },
    }
}
fn finish_ordinary_frame(runtime: &mut Runtime) {
    for phase in 1..=5 {
        runtime.lifecycle_begin_phase(phase).unwrap();
        assert!(runtime.lifecycle_next(1).unwrap().is_none());
    }
}

fn press_binding(action: &str, device: engine_input::DeviceId) -> Binding {
    Binding {
        action: ActionName::new(action.to_owned()).expect("valid action name"),
        device: Some(device),
        control: "button".to_owned(),
        trigger: Trigger::Pressed,
        priority: 1,
    }
}

fn assert_lifecycle_rejection(result: Result<(), RuntimeInputError>) {
    assert_eq!(
        result,
        Err(RuntimeInputError::Runtime(RuntimeError::Lifecycle))
    );
}

fn fill_runtime_queue(runtime: &mut Runtime, descriptor: &DeviceDescriptor) {
    runtime.input_push_event(connected(1, descriptor)).unwrap();
    for sequence in 2..=4096 {
        runtime
            .input_push_event(changed(sequence, descriptor.id, true))
            .unwrap();
    }
}

#[test]
fn runtimes_have_independent_input_lifetimes_and_device_retirement() {
    let descriptor = device("first-runtime");
    let mut first = Runtime::default();
    first.input_push_event(connected(1, &descriptor)).unwrap();
    first
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    first.lifecycle_begin_frame().unwrap();
    assert_eq!(first.input_frame().held(descriptor.id, "button"), Ok(true));

    let second = Runtime::default();
    assert_eq!(
        second.input_frame().held(descriptor.id, "button"),
        Err(engine_input::FrameError::UnknownControl)
    );

    finish_ordinary_frame(&mut first);
    first
        .input_push_event(InputEvent {
            sequence: 3,
            timestamp_ns: 300,
            kind: InputEventKind::Disconnected(descriptor.id),
        })
        .unwrap();
    first.lifecycle_begin_frame().unwrap();
    assert_eq!(
        first.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
    assert!(first.input_push_event(connected(4, &descriptor)).is_err());
}

#[test]
fn runtime_queue_accepts_exact_capacity_and_retries_the_next_sequence() {
    let descriptor = device("runtime-capacity");
    let mut runtime = Runtime::default();
    fill_runtime_queue(&mut runtime, &descriptor);

    assert_eq!(
        runtime.input_push_event(changed(4097, descriptor.id, false)),
        Err(RuntimeInputError::Event(EventError::QueueFull))
    );
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );

    finish_ordinary_frame(&mut runtime);
    runtime
        .input_push_event(changed(4097, descriptor.id, false))
        .unwrap();
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(false)
    );
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn ordinary_frame_commits_fifo_edges_and_defers_late_events() {
    let descriptor = device("ordinary-frame");
    let mut runtime = Runtime::default();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed_at(2, 50, descriptor.id, true))
        .unwrap();
    runtime.lifecycle_begin_frame().unwrap();

    let frame = runtime.input_frame();
    assert_eq!(frame.pressed(descriptor.id, "button"), Ok(true));
    assert_eq!(frame.held(descriptor.id, "button"), Ok(true));

    // This is post-commit ingress while the frame is active; it models the
    // event a callback could submit without invoking a callback in this test.
    runtime
        .input_push_event(changed(3, descriptor.id, false))
        .unwrap();
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(false)
    );

    finish_ordinary_frame(&mut runtime);
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(false)
    );
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn action_reads_are_stable_and_binding_edits_require_idle_lifecycle() {
    let descriptor = device("actions");
    let mut runtime = Runtime::default();
    runtime
        .input_bind(press_binding("jump", descriptor.id))
        .unwrap();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    runtime.lifecycle_begin_frame().unwrap();

    let first = runtime.input_actions();
    let second = runtime.input_actions();
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);

    assert_lifecycle_rejection(runtime.input_bind(press_binding("blocked", descriptor.id)));
}

#[test]
fn failed_outer_frame_preflight_preserves_accepted_input_and_snapshot() {
    let descriptor = device("preflight");
    let mut runtime = Runtime::default();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();

    runtime.lifecycle_begin_frame().unwrap();
    let before = runtime.input_frame().value(descriptor.id, "button");
    assert_eq!(before, Ok(InputValue::Button(true)));

    runtime
        .input_push_event(changed(3, descriptor.id, false))
        .unwrap();
    let failure = runtime.lifecycle_begin_frame();
    assert_lifecycle_rejection(failure);
    assert_eq!(runtime.input_frame().value(descriptor.id, "button"), before);

    finish_ordinary_frame(&mut runtime);
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn queued_disconnect_survives_failed_frame_and_publishes_a_tombstone_on_retry() {
    let descriptor = device("disconnect-retry");
    let mut runtime = Runtime::default();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    runtime.lifecycle_begin_frame().unwrap();

    runtime
        .input_push_event(InputEvent {
            sequence: 3,
            timestamp_ns: 300,
            kind: InputEventKind::Disconnected(descriptor.id),
        })
        .unwrap();
    assert_lifecycle_rejection(runtime.lifecycle_begin_frame());
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );

    finish_ordinary_frame(&mut runtime);
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn newer_events_join_the_fifo_after_a_failed_attempt_and_keep_order() {
    let descriptor = device("retry-order");
    let mut runtime = Runtime::default();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    runtime.lifecycle_begin_frame().unwrap();

    runtime
        .input_push_event(changed(3, descriptor.id, false))
        .unwrap();
    assert_lifecycle_rejection(runtime.lifecycle_begin_frame());
    runtime
        .input_push_event(changed(4, descriptor.id, true))
        .unwrap();
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );

    finish_ordinary_frame(&mut runtime);
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn destruction_only_flush_does_not_advance_the_published_input_frame() {
    let descriptor = device("destruction-only");
    let mut runtime = Runtime::default();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    runtime.lifecycle_begin_frame().unwrap();
    finish_ordinary_frame(&mut runtime);

    runtime
        .input_push_event(changed(3, descriptor.id, false))
        .unwrap();
    runtime.lifecycle_flush_destroy().unwrap();
    assert!(runtime.lifecycle_next(1).unwrap().is_none());
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );

    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn timed_outer_frame_advances_once_and_fixed_or_phase_reads_do_not_advance_input() {
    let descriptor = device("timed");
    let mut runtime = Runtime::default();
    runtime.time_configure(1, 0.02).unwrap();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    runtime.time_begin_frame().unwrap();

    assert_eq!(
        runtime.input_frame().pressed(descriptor.id, "button"),
        Ok(true)
    );
    runtime.lifecycle_begin_phase(1).unwrap();
    assert!(runtime.time_next(1).unwrap().is_none());
    assert_eq!(
        runtime.input_frame().pressed(descriptor.id, "button"),
        Ok(true)
    );
    assert!(runtime.time_begin_fixed().unwrap());
    assert_eq!(
        runtime.input_frame().pressed(descriptor.id, "button"),
        Ok(true)
    );

    runtime
        .input_push_event(changed(3, descriptor.id, false))
        .unwrap();
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );
}

#[test]
fn timed_preflight_failure_preserves_clock_and_queued_input() {
    let descriptor = device("timed-preflight");
    let mut runtime = Runtime::default();
    runtime.time_configure(1, 0.02).unwrap();
    runtime.input_push_event(connected(1, &descriptor)).unwrap();
    runtime.time_begin_frame().unwrap();
    let before_clock = runtime.time_snapshot().unwrap();

    runtime
        .input_push_event(changed(2, descriptor.id, true))
        .unwrap();
    assert_lifecycle_rejection(runtime.time_begin_frame());
    assert_eq!(runtime.time_snapshot().unwrap().values, before_clock.values);
    assert_eq!(runtime.time_snapshot().unwrap().frame, before_clock.frame);
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(false)
    );

    runtime.lifecycle_begin_phase(1).unwrap();
    assert!(runtime.time_next(1).unwrap().is_none());
    while runtime.time_begin_fixed().unwrap() {
        assert!(runtime.time_next(1).unwrap().is_none());
    }
    for phase in 2..=5 {
        runtime.lifecycle_begin_phase(phase).unwrap();
        assert!(runtime.time_next(1).unwrap().is_none());
    }
    runtime.time_begin_frame().unwrap();
    assert!(runtime.input_frame().held(descriptor.id, "button").unwrap());
    assert_eq!(
        runtime.time_snapshot().unwrap().frame,
        before_clock.frame + 1
    );
}
