//! Public input conformance candidates for the approved lane 9 contract.

use engine_identifiers::allocate_next;
use engine_input::{
    ActionName, ActionRouter, ActionSample, AnalogError, AnalogSettings, Binding, BindingError,
    ControlDescriptor, ControlKind, DeviceDescriptor, DeviceKind, EventError, EventQueue,
    FrameError, FrameInput, InputEvent, InputEventKind, InputValue, Trigger, process_analog,
};

fn device(button: &str, axis: &str) -> DeviceDescriptor {
    DeviceDescriptor {
        id: allocate_next().expect("device identity allocation should succeed"),
        name: "controller".to_owned(),
        kind: DeviceKind::Gamepad,
        controls: vec![
            ControlDescriptor {
                path: button.to_owned(),
                kind: ControlKind::Button,
            },
            ControlDescriptor {
                path: axis.to_owned(),
                kind: ControlKind::Axis,
            },
        ],
    }
}

fn connected(sequence: u64, descriptor: &DeviceDescriptor) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence * 10,
        kind: InputEventKind::Connected(descriptor.clone()),
    }
}

fn changed(
    sequence: u64,
    device: engine_input::DeviceId,
    control: &str,
    value: InputValue,
) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence * 10,
        kind: InputEventKind::Changed {
            device,
            control: control.to_owned(),
            value,
        },
    }
}

#[test]
fn queue_preserves_fifo_order_and_rejects_sequence_or_type_errors() {
    let descriptor = device("button", "axis");
    let mut queue = EventQueue::with_capacity(2).unwrap();

    queue.push(connected(4, &descriptor)).unwrap();
    assert_eq!(
        queue.push(connected(4, &descriptor)),
        Err(EventError::DuplicateSequence)
    );
    assert_eq!(
        queue.push(changed(5, descriptor.id, "axis", InputValue::Button(true),)),
        Err(EventError::TypeMismatch)
    );
    assert_eq!(
        queue.push(changed(3, descriptor.id, "axis", InputValue::Scalar(0.5),)),
        Err(EventError::StaleSequence)
    );

    let second = changed(5, descriptor.id, "axis", InputValue::Scalar(0.5));
    queue.push(second.clone()).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(
        queue.push(changed(6, descriptor.id, "axis", InputValue::Scalar(1.0))),
        Err(EventError::QueueFull)
    );

    let mut drained = Vec::new();
    assert_eq!(queue.drain_into(&mut drained), Ok(2));
    assert_eq!(drained[0], connected(4, &descriptor));
    assert_eq!(drained[1], second);
    assert!(queue.is_empty());
}

#[test]
fn queue_disconnect_removes_controls_and_rejects_unknown_devices() {
    let descriptor = device("button", "axis");
    let mut queue = EventQueue::with_capacity(4).unwrap();
    queue.push(connected(1, &descriptor)).unwrap();
    assert_eq!(
        queue.push(changed(
            2,
            descriptor.id,
            "missing",
            InputValue::Button(true),
        )),
        Err(EventError::UnknownControl)
    );
    assert_eq!(
        queue.push(changed(
            3,
            descriptor.id,
            "axis",
            InputValue::Scalar(f32::NAN),
        )),
        Err(EventError::NonFiniteValue)
    );
    queue
        .push(InputEvent {
            sequence: 4,
            timestamp_ns: 40,
            kind: InputEventKind::Disconnected(descriptor.id),
        })
        .unwrap();
    assert_eq!(
        queue.push(changed(5, descriptor.id, "axis", InputValue::Scalar(0.25))),
        Err(EventError::UnknownDevice)
    );
    assert_eq!(
        queue.push(InputEvent {
            sequence: 6,
            timestamp_ns: 60,
            kind: InputEventKind::Disconnected(descriptor.id),
        }),
        Err(EventError::UnknownDevice)
    );
    assert_eq!(
        queue.push(connected(5, &descriptor)),
        Err(EventError::RetiredDevice)
    );
    let mut drained = Vec::new();
    queue.drain_into(&mut drained).unwrap();
    assert_eq!(
        queue.push(connected(6, &descriptor)),
        Err(EventError::RetiredDevice)
    );
}

#[test]
fn frame_edges_carry_forward_and_focus_loss_releases_everything() {
    let descriptor = device("button", "axis");
    let mut state = FrameInput::new();
    state.apply(&connected(1, &descriptor)).unwrap();
    state
        .apply(&changed(
            2,
            descriptor.id,
            "button",
            InputValue::Button(true),
        ))
        .unwrap();
    state
        .apply(&changed(3, descriptor.id, "axis", InputValue::Scalar(0.75)))
        .unwrap();
    assert_eq!(state.pressed(descriptor.id, "button"), Ok(true));
    assert_eq!(state.held(descriptor.id, "button"), Ok(true));

    assert_eq!(
        state.apply(&changed(4, descriptor.id, "axis", InputValue::Button(true),)),
        Err(FrameError::TypeMismatch)
    );
    assert_eq!(state.release_device(descriptor.id), Ok(()));
    assert_eq!(state.released(descriptor.id, "button"), Ok(true));
    assert_eq!(state.release_device(descriptor.id), Ok(()));
    let unknown_device = allocate_next().unwrap();
    assert_eq!(
        state.release_device(unknown_device),
        Err(FrameError::UnknownControl)
    );

    state.begin_frame().unwrap();
    assert_eq!(state.held(descriptor.id, "button"), Ok(false));
    assert_eq!(
        state.value(descriptor.id, "button"),
        Err(FrameError::UnknownControl)
    );
    assert_eq!(state.release_device(descriptor.id), Ok(()));
    assert_eq!(
        state.released(descriptor.id, "button"),
        Err(FrameError::UnknownControl)
    );

    state.reset();
    state.apply(&connected(5, &descriptor)).unwrap();
    state
        .apply(&changed(
            6,
            descriptor.id,
            "button",
            InputValue::Button(true),
        ))
        .unwrap();
    state
        .apply(&changed(7, descriptor.id, "axis", InputValue::Scalar(0.75)))
        .unwrap();
    state.focus_lost();
    assert_eq!(state.held(descriptor.id, "button"), Ok(false));
    assert_eq!(state.released(descriptor.id, "button"), Ok(true));
    assert_eq!(
        state.value(descriptor.id, "axis"),
        Ok(InputValue::Scalar(0.0))
    );
    state.focus_lost();
    state
        .apply(&changed(
            8,
            descriptor.id,
            "button",
            InputValue::Button(true),
        ))
        .unwrap();
    state
        .apply(&InputEvent {
            sequence: 9,
            timestamp_ns: 90,
            kind: InputEventKind::Disconnected(descriptor.id),
        })
        .unwrap();
    assert_eq!(state.released(descriptor.id, "button"), Ok(true));
    assert_eq!(state.held(descriptor.id, "button"), Ok(false));
    assert_eq!(
        state.value(descriptor.id, "button"),
        Err(FrameError::UnknownControl)
    );
    assert_eq!(
        state.apply(&connected(10, &descriptor)),
        Err(FrameError::RetiredDevice)
    );
    state.begin_frame().unwrap();
    assert_eq!(state.release_device(descriptor.id), Ok(()));
    assert_eq!(
        state.released(descriptor.id, "button"),
        Err(FrameError::UnknownControl)
    );
    state.focus_lost();
    state.reset();
    assert_eq!(
        state.held(descriptor.id, "button"),
        Err(FrameError::UnknownControl)
    );
    state.apply(&connected(11, &descriptor)).unwrap();
}

#[test]
fn routing_orders_priority_and_supports_dormant_bindings_and_cancel() {
    let descriptor = device("button", "axis");
    let mut state = FrameInput::new();
    state.apply(&connected(1, &descriptor)).unwrap();
    state
        .apply(&changed(
            2,
            descriptor.id,
            "button",
            InputValue::Button(true),
        ))
        .unwrap();
    state
        .apply(&changed(3, descriptor.id, "axis", InputValue::Scalar(0.75)))
        .unwrap();

    let mut router = ActionRouter::new();
    let jump = ActionName::new("jump".to_owned()).unwrap();
    let move_action = ActionName::new("move".to_owned()).unwrap();
    let dormant = ActionName::new("future".to_owned()).unwrap();
    router
        .bind(Binding {
            action: jump.clone(),
            device: Some(descriptor.id),
            control: "button".to_owned(),
            trigger: Trigger::Pressed,
            priority: 10,
        })
        .unwrap();
    router
        .bind(Binding {
            action: move_action.clone(),
            device: Some(descriptor.id),
            control: "axis".to_owned(),
            trigger: Trigger::Value,
            priority: 20,
        })
        .unwrap();
    router
        .bind(Binding {
            action: dormant.clone(),
            device: None,
            control: "missing".to_owned(),
            trigger: Trigger::Held,
            priority: 100,
        })
        .unwrap();
    assert_eq!(
        router.sample(&state),
        vec![
            ActionSample {
                action: move_action,
                trigger: Trigger::Value,
                value: InputValue::Scalar(0.75),
                device: Some(descriptor.id),
            },
            ActionSample {
                action: jump.clone(),
                trigger: Trigger::Pressed,
                value: InputValue::Button(true),
                device: Some(descriptor.id),
            },
        ]
    );
    assert_eq!(
        router.bind(Binding {
            action: jump.clone(),
            device: Some(descriptor.id),
            control: "button".to_owned(),
            trigger: Trigger::Pressed,
            priority: 10,
        }),
        Err(BindingError::DuplicateBinding)
    );
    assert_eq!(router.cancel(&jump), Ok(1));
    assert_eq!(router.sample(&state).len(), 1);
    assert_eq!(router.unbind(&dormant, "missing", None), Ok(true));
}

#[test]
fn analog_processing_uses_inclusive_deadzone_and_unit_direction() {
    let remapped = process_analog(
        [0.625, 0.0, 0.0],
        AnalogSettings {
            deadzone: 0.25,
            normalize: false,
        },
    )
    .unwrap();
    assert_eq!(remapped, [0.5, 0.0, 0.0]);
    assert_eq!(
        process_analog(
            [0.25, 0.0, 0.0],
            AnalogSettings {
                deadzone: 0.25,
                normalize: false
            }
        ),
        Ok([0.0; 3])
    );
    assert_eq!(
        process_analog(
            [0.625, 0.0, 0.0],
            AnalogSettings {
                deadzone: 0.25,
                normalize: true
            }
        )
        .unwrap(),
        [1.0, 0.0, 0.0]
    );
    assert_eq!(
        process_analog(
            [f32::NAN, 0.0, 0.0],
            AnalogSettings {
                deadzone: 0.0,
                normalize: false
            }
        ),
        Err(AnalogError::NonFinite)
    );
    assert_eq!(
        process_analog(
            [0.0; 3],
            AnalogSettings {
                deadzone: 1.0,
                normalize: false
            }
        ),
        Err(AnalogError::InvalidDeadzone)
    );
    let extreme = process_analog(
        [f32::MAX, f32::MAX, f32::MAX],
        AnalogSettings {
            deadzone: 0.0,
            normalize: true,
        },
    )
    .unwrap();
    assert!(extreme.iter().all(|component| component.is_finite()));
}
