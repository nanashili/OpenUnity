use super::*;

#[test]
fn eviction_retains_newest_in_sequence_order() {
    let mut console = Console::new(2).unwrap();
    assert_eq!(console.push(Severity::Info, "first"), Ok(1));
    assert_eq!(console.push(Severity::Warning, "second"), Ok(2));
    assert_eq!(console.push(Severity::Error, "third"), Ok(3));
    assert_eq!(console.capacity(), 2);
    assert_eq!(console.dropped_count(), 1);
    let records: Vec<_> = console.records().collect();
    assert_eq!(records[0].sequence, 2);
    assert_eq!(records[0].message, "second");
    assert_eq!(records[1].sequence, 3);
}

#[test]
fn filters_combine_without_reordering_or_mutation() {
    let mut console = Console::new(4).unwrap();
    console.push(Severity::Info, "asset loaded").unwrap();
    console.push(Severity::Error, "asset failed").unwrap();
    console.push(Severity::Warning, "Asset warning").unwrap();
    console.push(Severity::Error, "scene failed").unwrap();
    let sequences: Vec<_> = console
        .filtered(&[Severity::Error, Severity::Warning], "asset")
        .map(|record| record.sequence)
        .collect();
    assert_eq!(sequences, [2]);
    assert_eq!(console.filtered(&[], "").count(), 0);
    assert_eq!(console.filtered(&[Severity::Error], "").count(), 2);
    assert_eq!(console.records().len(), 4);
    assert_eq!(console.dropped_count(), 0);
}

#[test]
fn clear_preserves_lifetime_counters() {
    let mut console = Console::new(1).unwrap();
    console.push(Severity::Info, "one").unwrap();
    console.push(Severity::Info, "two").unwrap();
    console.clear();
    assert_eq!(console.records().len(), 0);
    assert_eq!(console.dropped_count(), 1);
    assert_eq!(console.push(Severity::Info, "three"), Ok(3));
    assert_eq!(console.dropped_count(), 1);
}

#[test]
fn zero_capacity_consumes_sequences_and_counts_drops() {
    let mut console = Console::new(0).unwrap();
    assert_eq!(console.push(Severity::Error, "discarded"), Ok(1));
    console.clear();
    assert_eq!(console.push(Severity::Info, "discarded too"), Ok(2));
    assert_eq!(console.records().len(), 0);
    assert_eq!(console.dropped_count(), 2);
}

#[test]
fn sequence_exhaustion_never_wraps_or_evicts() {
    let mut console = Console::new(1).unwrap();
    console.last_sequence = u64::MAX - 1;
    assert_eq!(console.push(Severity::Info, "last"), Ok(u64::MAX));
    assert_eq!(
        console.push(Severity::Error, "rejected"),
        Err(ConsoleError::SequenceExhausted)
    );
    assert_eq!(console.records().next().unwrap().message, "last");
    assert_eq!(console.dropped_count(), 0);
    console.clear();
    assert_eq!(
        console.push(Severity::Error, "still rejected"),
        Err(ConsoleError::SequenceExhausted)
    );
}

#[test]
fn drop_counter_exhaustion_rejects_atomically() {
    for capacity in [0, 1] {
        let mut console = Console::new(capacity).unwrap();
        console.push(Severity::Info, "retained").unwrap();
        console.dropped_count = u64::MAX;
        assert_eq!(
            console.push(Severity::Error, "rejected"),
            Err(ConsoleError::DroppedCountExhausted)
        );
        assert_eq!(console.dropped_count(), u64::MAX);
        if capacity == 1 {
            assert_eq!(console.records().next().unwrap().message, "retained");
        }
        console.dropped_count = 0;
        console.clear();
        assert_eq!(console.push(Severity::Info, "next"), Ok(2));
    }
}

#[test]
fn capacity_and_utf8_byte_limits_are_enforced_before_mutation() {
    assert!(matches!(
        Console::new(MAX_CONSOLE_CAPACITY + 1),
        Err(ConsoleError::CapacityTooLarge)
    ));
    assert!(Console::new(MAX_CONSOLE_CAPACITY).is_ok());
    let mut console = Console::new(1).unwrap();
    let boundary = "é".repeat(MAX_MESSAGE_BYTES / 2);
    assert_eq!(console.push(Severity::Info, &boundary), Ok(1));
    assert_eq!(
        console.push(Severity::Error, format!("{boundary}x")),
        Err(ConsoleError::MessageTooLarge)
    );
    assert_eq!(console.records().next().unwrap().message, boundary);
    assert_eq!(console.dropped_count(), 0);
    console.clear();
    assert_eq!(console.push(Severity::Info, "next"), Ok(2));
}
