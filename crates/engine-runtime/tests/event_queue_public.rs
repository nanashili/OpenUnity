//! Public EventQueue admission and drain conformance candidates.

use engine_identifiers::allocate_next;
use engine_input::{
    ControlDescriptor, ControlKind, DeviceDescriptor, DeviceKind, EventError, EventQueue,
    InputEvent, InputEventKind, InputValue,
};

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
    InputEvent {
        sequence,
        timestamp_ns: sequence * 100,
        kind: InputEventKind::Changed {
            device,
            control: "button".to_owned(),
            value: InputValue::Button(value),
        },
    }
}

fn malformed_change(sequence: u64, device: engine_input::DeviceId) -> InputEvent {
    InputEvent {
        sequence,
        timestamp_ns: sequence * 100,
        kind: InputEventKind::Changed {
            device,
            control: "missing".to_owned(),
            value: InputValue::Button(true),
        },
    }
}

#[test]
fn queue_rejects_duplicate_stale_and_malformed_events_without_mutation() {
    let descriptor = device("queue-validation");
    let connected_event = connected(5, &descriptor);
    let mut queue = EventQueue::with_capacity(3).unwrap();
    queue.push(connected_event.clone()).unwrap();

    assert_eq!(
        queue.push(connected_event),
        Err(EventError::DuplicateSequence)
    );
    assert_eq!(
        queue.push(changed(4, descriptor.id, true)),
        Err(EventError::StaleSequence)
    );
    assert_eq!(
        queue.push(malformed_change(6, descriptor.id)),
        Err(EventError::UnknownControl)
    );
    assert_eq!(queue.len(), 1);

    queue.push(changed(6, descriptor.id, true)).unwrap();
    let mut drained = Vec::new();
    drained.try_reserve_exact(queue.len()).unwrap();
    assert_eq!(queue.drain_into(&mut drained), Ok(2));
    assert_eq!(drained[0], connected(5, &descriptor));
    assert_eq!(drained[1], changed(6, descriptor.id, true));
}

#[test]
fn queue_full_rejection_preserves_the_existing_fifo() {
    let descriptor = device("queue-full");
    let first = connected(1, &descriptor);
    let mut queue = EventQueue::with_capacity(1).unwrap();
    queue.push(first.clone()).unwrap();

    assert_eq!(
        queue.push(changed(2, descriptor.id, true)),
        Err(EventError::QueueFull)
    );
    assert_eq!(queue.len(), 1);

    let mut drained = Vec::new();
    drained.try_reserve_exact(queue.len()).unwrap();
    queue.drain_into(&mut drained).unwrap();
    assert_eq!(drained, vec![first]);
    queue.push(changed(2, descriptor.id, true)).unwrap();
    assert_eq!(queue.len(), 1);
}
