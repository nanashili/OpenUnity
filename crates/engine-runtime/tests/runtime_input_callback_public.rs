//! Runtime input publication while lifecycle callbacks are in flight.

use engine_input::{
    ControlDescriptor, ControlKind, DeviceDescriptor, DeviceKind, InputEvent, InputEventKind,
    InputValue,
};
use engine_runtime::{LifecycleEvent, Runtime};

fn descriptor(runtime: &mut Runtime) -> DeviceDescriptor {
    DeviceDescriptor {
        id: runtime.create_object().expect("test device identity"),
        name: "callback-input".to_owned(),
        kind: DeviceKind::Keyboard,
        controls: vec![ControlDescriptor {
            path: "button".to_owned(),
            kind: ControlKind::Button,
        }],
    }
}

fn event(sequence: u64, device: engine_input::DeviceId, value: InputValue) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence * 100,
        kind: InputEventKind::Changed {
            device,
            control: "button".to_owned(),
            value,
        },
    }
}

fn connect(sequence: u64, descriptor: &DeviceDescriptor) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence * 100,
        kind: InputEventKind::Connected(descriptor.clone()),
    }
}

fn install_script(runtime: &mut Runtime) -> engine_object::ObjectId {
    let owner = runtime
        .create_gameobject("callback-owner")
        .expect("gameobject identity");
    let script = runtime
        .component_reserve(owner, 0)
        .expect("script reservation");
    runtime
        .component_commit(script)
        .expect("script publication");
    script
}

fn acknowledge(runtime: &mut Runtime, delivery: LifecycleEvent) {
    runtime
        .lifecycle_ack(delivery.token, 0)
        .expect("callback acknowledgement");
}

fn drain_phase(runtime: &mut Runtime, phase: i32) {
    runtime
        .lifecycle_begin_phase(phase)
        .expect("phase admission");

    drain_current_phase(runtime);
}

fn drain_current_phase(runtime: &mut Runtime) {
    while let Some(delivery) = runtime.lifecycle_next(1).expect("callback delivery") {
        acknowledge(runtime, delivery);
    }
}

#[test]
fn queued_input_waits_behind_an_in_flight_lifecycle_callback() {
    let mut runtime = Runtime::default();
    let descriptor = descriptor(&mut runtime);
    let script = install_script(&mut runtime);

    runtime
        .input_push_event(connect(1, &descriptor))
        .expect("device connection");
    runtime
        .input_push_event(event(2, descriptor.id, InputValue::Button(true)))
        .expect("button press");
    runtime.lifecycle_begin_frame().expect("outer frame");

    runtime.lifecycle_begin_phase(1).expect("start phase");
    let delivery = runtime
        .lifecycle_next(1)
        .expect("callback delivery")
        .expect("awake callback");
    assert_eq!(delivery.component, script);

    runtime
        .input_push_event(event(3, descriptor.id, InputValue::Button(false)))
        .expect("callback-time release");
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );

    acknowledge(&mut runtime, delivery);
    drain_current_phase(&mut runtime);
    for phase in 2..=5 {
        drain_phase(&mut runtime, phase);
    }

    runtime.lifecycle_begin_frame().expect("next outer frame");
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
fn timed_fixed_callback_does_not_publish_queued_input() {
    let mut runtime = Runtime::default();
    let descriptor = descriptor(&mut runtime);
    install_script(&mut runtime);
    runtime
        .time_configure(1, 0.02)
        .expect("clock configuration");
    runtime
        .input_push_event(connect(1, &descriptor))
        .expect("device connection");
    runtime
        .input_push_event(event(2, descriptor.id, InputValue::Button(true)))
        .expect("button press");
    runtime.time_begin_frame().expect("timed outer frame");

    runtime.lifecycle_begin_phase(1).expect("start phase");
    while let Some(delivery) = runtime.time_next(1).expect("timed callback") {
        acknowledge(&mut runtime, delivery);
    }

    runtime
        .input_push_event(event(3, descriptor.id, InputValue::Button(false)))
        .expect("fixed-step release");
    assert!(runtime.time_begin_fixed().expect("fixed-step admission"));
    drain_current_timed_phase(&mut runtime);
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(true)
    );
    assert!(!runtime.time_begin_fixed().expect("fixed-step completion"));

    for phase in 2..=5 {
        runtime
            .lifecycle_begin_phase(phase)
            .expect("phase admission");
        drain_current_timed_phase(&mut runtime);
    }

    runtime.time_begin_frame().expect("next timed frame");
    assert_eq!(
        runtime.input_frame().held(descriptor.id, "button"),
        Ok(false)
    );
    assert_eq!(
        runtime.input_frame().released(descriptor.id, "button"),
        Ok(true)
    );
}

fn drain_current_timed_phase(runtime: &mut Runtime) {
    while let Some(delivery) = runtime.time_next(1).expect("timed callback") {
        acknowledge(runtime, delivery);
    }
}
