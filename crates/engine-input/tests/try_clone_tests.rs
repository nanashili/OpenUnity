//! Tests for independent, fallible input state snapshots.

use engine_identifiers::allocate_next;
use engine_input::{
    ControlDescriptor, ControlKind, DeviceDescriptor, DeviceKind, EventError, EventQueue,
    FrameError, FrameInput, InputEvent, InputEventKind, InputValue,
};

fn descriptor() -> DeviceDescriptor {
    DeviceDescriptor {
        id: allocate_next().unwrap(),
        name: "clone-test".to_owned(),
        kind: DeviceKind::Custom("clone-device".to_owned()),
        controls: vec![
            ControlDescriptor {
                path: "button".to_owned(),
                kind: ControlKind::Button,
            },
            ControlDescriptor {
                path: "axis".to_owned(),
                kind: ControlKind::Axis,
            },
        ],
    }
}

fn connected(sequence: u64, descriptor: &DeviceDescriptor) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence,
        kind: InputEventKind::Connected(descriptor.clone()),
    }
}

fn changed(
    sequence: u64,
    device: &DeviceDescriptor,
    control: &str,
    value: InputValue,
) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence,
        kind: InputEventKind::Changed {
            device: device.id,
            control: control.to_owned(),
            value,
        },
    }
}

#[test]
fn event_queue_clone_preserves_fifo_and_sequence_cursor() {
    let device = descriptor();
    let mut queue = EventQueue::with_capacity(4).unwrap();
    queue.push(connected(1, &device)).unwrap();
    queue
        .push(changed(2, &device, "button", InputValue::Button(true)))
        .unwrap();

    let mut clone = queue.try_clone().unwrap();
    let sentinel = changed(0, &device, "axis", InputValue::Scalar(-0.25));
    let mut events = vec![sentinel.clone()];
    assert_eq!(clone.drain_into(&mut events), Ok(2));
    assert_eq!(events[0], sentinel);
    assert_eq!(events[1], connected(1, &device));
    assert_eq!(
        events[2],
        changed(2, &device, "button", InputValue::Button(true))
    );

    assert!(matches!(
        clone.push(changed(2, &device, "axis", InputValue::Scalar(0.5))),
        Err(EventError::DuplicateSequence)
    ));
    clone
        .push(changed(3, &device, "axis", InputValue::Scalar(0.5)))
        .unwrap();
    for sequence in 4..=6 {
        clone
            .push(changed(sequence, &device, "axis", InputValue::Scalar(0.5)))
            .unwrap();
    }
    assert_eq!(clone.len(), 4);
    assert_eq!(
        clone.push(changed(7, &device, "axis", InputValue::Scalar(0.5))),
        Err(EventError::QueueFull)
    );

    assert!(matches!(
        queue.push(changed(2, &device, "axis", InputValue::Scalar(0.5))),
        Err(EventError::DuplicateSequence)
    ));
    queue
        .push(changed(3, &device, "axis", InputValue::Scalar(0.5)))
        .unwrap();
}

#[test]
fn event_queue_clone_preserves_retired_device_lifecycle() {
    let device = descriptor();
    let mut clone = EventQueue::with_capacity(4).unwrap();
    clone.push(connected(1, &device)).unwrap();

    let mut original = clone.try_clone().unwrap();
    clone
        .push(InputEvent {
            sequence: 2,
            timestamp_ns: 2,
            kind: InputEventKind::Disconnected(device.id),
        })
        .unwrap();
    assert!(matches!(
        clone.push(connected(3, &device)),
        Err(EventError::RetiredDevice)
    ));
    let mut retired_snapshot = clone.try_clone().unwrap();
    assert_eq!(
        retired_snapshot.push(connected(3, &device)),
        Err(EventError::RetiredDevice)
    );

    original
        .push(changed(2, &device, "button", InputValue::Button(true)))
        .unwrap();
}

#[test]
fn rejected_connection_does_not_consume_sequence_or_change_device_registry() {
    let active_device = descriptor();
    let second_device = descriptor();
    let mut queue = EventQueue::with_capacity(4).unwrap();

    queue.push(connected(1, &active_device)).unwrap();
    assert_eq!(
        queue.push(connected(2, &active_device)),
        Err(EventError::InvalidDescriptor)
    );

    assert_eq!(queue.len(), 1);
    queue
        .push(changed(
            2,
            &active_device,
            "button",
            InputValue::Button(true),
        ))
        .unwrap();
    queue.push(connected(3, &second_device)).unwrap();

    assert_eq!(queue.len(), 3);
}

#[test]
fn frame_clone_preserves_nested_values_and_edges_independently() {
    let device = descriptor();
    let mut frame = FrameInput::new();
    frame.apply(&connected(1, &device)).unwrap();
    frame
        .apply(&changed(2, &device, "button", InputValue::Button(true)))
        .unwrap();
    frame
        .apply(&changed(3, &device, "axis", InputValue::Scalar(0.25)))
        .unwrap();

    let mut clone = frame.try_clone().unwrap();
    clone
        .apply(&changed(4, &device, "button", InputValue::Button(false)))
        .unwrap();
    clone
        .apply(&changed(5, &device, "axis", InputValue::Scalar(0.75)))
        .unwrap();

    assert_eq!(frame.held(device.id, "button"), Ok(true));
    assert_eq!(frame.value(device.id, "axis"), Ok(InputValue::Scalar(0.25)));
    assert_eq!(clone.held(device.id, "button"), Ok(false));
    assert_eq!(clone.released(device.id, "button"), Ok(true));
    assert_eq!(clone.value(device.id, "axis"), Ok(InputValue::Scalar(0.75)));
}

#[test]
fn frame_clone_preserves_release_tombstones_until_begin_frame() {
    let device = descriptor();
    let mut frame = FrameInput::new();
    frame.apply(&connected(1, &device)).unwrap();
    frame
        .apply(&changed(2, &device, "button", InputValue::Button(true)))
        .unwrap();

    frame
        .apply(&InputEvent {
            sequence: 3,
            timestamp_ns: 3,
            kind: InputEventKind::Disconnected(device.id),
        })
        .unwrap();
    let mut clone = frame.try_clone().unwrap();

    frame.begin_frame().unwrap();
    assert_eq!(
        frame.released(device.id, "button"),
        Err(FrameError::UnknownControl)
    );
    assert_eq!(clone.released(device.id, "button"), Ok(true));
    assert_eq!(
        clone.apply(&connected(4, &device)),
        Err(FrameError::RetiredDevice)
    );
    assert_eq!(clone.released(device.id, "button"), Ok(true));
}
