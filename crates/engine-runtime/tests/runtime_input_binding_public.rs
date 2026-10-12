//! Public Runtime binding mutation conformance candidates.

use engine_identifiers::allocate_next;
use engine_input::{
    ActionName, Binding, ControlDescriptor, ControlKind, DeviceDescriptor, DeviceKind, InputEvent,
    InputEventKind, InputValue, Trigger,
};
use engine_runtime::{Runtime, RuntimeError, RuntimeInputError};

fn descriptor() -> DeviceDescriptor {
    DeviceDescriptor {
        id: allocate_next().expect("test device identity allocation"),
        name: "binding-runtime".to_owned(),
        kind: DeviceKind::Keyboard,
        controls: vec![ControlDescriptor {
            path: "button".to_owned(),
            kind: ControlKind::Button,
        }],
    }
}

fn action(name: &str) -> ActionName {
    ActionName::new(name.to_owned()).expect("valid action name")
}

fn binding(name: &str, device: engine_input::DeviceId, trigger: Trigger) -> Binding {
    Binding {
        action: action(name),
        device: Some(device),
        control: "button".to_owned(),
        trigger,
        priority: 1,
    }
}

fn connected(device: &DeviceDescriptor) -> InputEvent {
    InputEvent {
        sequence: 1,
        timestamp_ns: 100,
        kind: InputEventKind::Connected(device.clone()),
    }
}

fn pressed(device: engine_input::DeviceId) -> InputEvent {
    InputEvent {
        sequence: 2,
        timestamp_ns: 200,
        kind: InputEventKind::Changed {
            device,
            control: "button".to_owned(),
            value: InputValue::Button(true),
        },
    }
}

fn finish_frame(runtime: &mut Runtime) {
    for phase in 1..=5 {
        runtime.lifecycle_begin_phase(phase).unwrap();
        assert!(runtime.lifecycle_next(1).unwrap().is_none());
    }
}

fn assert_lifecycle_rejection<T>(result: Result<T, RuntimeInputError>)
where
    T: std::fmt::Debug + PartialEq,
{
    assert_eq!(
        result,
        Err(RuntimeInputError::Runtime(RuntimeError::Lifecycle))
    );
}

#[test]
fn active_binding_edits_are_rejected_without_changing_action_samples() {
    let device = descriptor();
    let jump = action("jump");
    let held = action("held");
    let mut runtime = Runtime::default();
    runtime
        .input_bind(binding("jump", device.id, Trigger::Pressed))
        .unwrap();
    runtime
        .input_bind(binding("held", device.id, Trigger::Held))
        .unwrap();
    runtime.input_push_event(connected(&device)).unwrap();
    runtime.input_push_event(pressed(device.id)).unwrap();
    runtime.lifecycle_begin_frame().unwrap();

    let before = runtime.input_actions();
    assert_eq!(before.len(), 2);
    assert_lifecycle_rejection(runtime.input_unbind(&jump, "button", Some(device.id)));
    assert_eq!(runtime.input_actions(), before);
    assert_lifecycle_rejection(runtime.input_cancel(&held));
    assert_eq!(runtime.input_actions(), before);
}

#[test]
fn idle_unbind_and_cancel_remove_only_the_requested_bindings() {
    let device = descriptor();
    let jump = action("jump");
    let held = action("held");
    let mut runtime = Runtime::default();
    runtime
        .input_bind(binding("jump", device.id, Trigger::Pressed))
        .unwrap();
    runtime
        .input_bind(binding("held", device.id, Trigger::Held))
        .unwrap();
    runtime.input_push_event(connected(&device)).unwrap();
    runtime.input_push_event(pressed(device.id)).unwrap();
    runtime.lifecycle_begin_frame().unwrap();
    finish_frame(&mut runtime);

    assert_eq!(
        runtime.input_unbind(&jump, "button", Some(device.id)),
        Ok(true)
    );
    let remaining = runtime.input_actions();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].action, held);
    assert_eq!(runtime.input_cancel(&held), Ok(1));
    assert!(runtime.input_actions().is_empty());
}
