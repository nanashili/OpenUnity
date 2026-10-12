//! Independently authored bounded editor records; this is not Unity log capture.

use std::collections::VecDeque;
use std::fmt;

/// Independently chosen record limit, bounding retained message data to 64 MiB.
pub const MAX_CONSOLE_CAPACITY: usize = 1024;
/// Independently chosen UTF-8 byte limit for each message (64 KiB).
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// Severity assigned by the record's caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    /// Informational event.
    Info,
    /// Recoverable problem.
    Warning,
    /// Failed operation.
    Error,
}

/// One editor event. Sequence identities are independent of storage addresses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    /// Monotonically increasing positive identity within this console's lifetime.
    pub sequence: u64,
    /// Caller-assigned severity.
    pub severity: Severity,
    /// Unmodified UTF-8 message.
    pub message: String,
}

/// Rejected console operation. Rejection leaves the console unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleError {
    /// Requested retention exceeds [`MAX_CONSOLE_CAPACITY`].
    CapacityTooLarge,
    /// Message exceeds [`MAX_MESSAGE_BYTES`].
    MessageTooLarge,
    /// Every positive `u64` sequence has already been consumed.
    SequenceExhausted,
    /// Recording another dropped event would overflow its lifetime counter.
    DroppedCountExhausted,
}

impl fmt::Display for ConsoleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CapacityTooLarge => "console capacity exceeds 1024 records",
            Self::MessageTooLarge => "console message exceeds 65536 UTF-8 bytes",
            Self::SequenceExhausted => "console sequence identities are exhausted",
            Self::DroppedCountExhausted => "console dropped-record counter is exhausted",
        })
    }
}

impl std::error::Error for ConsoleError {}

/// Bounded records ordered oldest first, with checked lifetime counters.
#[derive(Clone, Debug)]
pub struct Console {
    capacity: usize,
    records: VecDeque<Record>,
    last_sequence: u64,
    dropped_count: u64,
}

impl Console {
    /// Creates an empty console, rejecting capacities above the documented limit.
    /// Zero capacity is valid: pushes consume sequences and count as drops.
    pub fn new(capacity: usize) -> Result<Self, ConsoleError> {
        if capacity > MAX_CONSOLE_CAPACITY {
            return Err(ConsoleError::CapacityTooLarge);
        }
        Ok(Self {
            capacity,
            records: VecDeque::new(),
            last_sequence: 0,
            dropped_count: 0,
        })
    }

    /// Appends a message and returns its sequence, evicting the oldest at capacity.
    /// Message bytes are checked before copying. Counters never wrap; exhaustion
    /// rejects the operation without evicting records or consuming a sequence.
    pub fn push(
        &mut self,
        severity: Severity,
        message: impl AsRef<str>,
    ) -> Result<u64, ConsoleError> {
        let message = message.as_ref();
        if message.len() > MAX_MESSAGE_BYTES {
            return Err(ConsoleError::MessageTooLarge);
        }
        let sequence = self
            .last_sequence
            .checked_add(1)
            .ok_or(ConsoleError::SequenceExhausted)?;
        let dropping = self.records.len() == self.capacity;
        let dropped_count = if dropping {
            self.dropped_count
                .checked_add(1)
                .ok_or(ConsoleError::DroppedCountExhausted)?
        } else {
            self.dropped_count
        };
        if self.capacity != 0 {
            let record = Record {
                sequence,
                severity,
                message: message.to_owned(),
            };
            if dropping {
                self.records.pop_front();
            }
            self.records.push_back(record);
        }
        self.last_sequence = sequence;
        self.dropped_count = dropped_count;
        Ok(sequence)
    }

    /// Iterates retained records in ascending sequence order.
    pub fn records(&self) -> impl ExactSizeIterator<Item = &Record> + DoubleEndedIterator {
        self.records.iter()
    }

    /// Iterates records matching both filters, preserving ascending sequence order.
    /// Substrings are literal and case-sensitive; an empty substring matches all
    /// messages. An empty severity slice matches none. Duplicate severities have
    /// no effect and filtering never changes retention or overflow counts.
    pub fn filtered<'a>(
        &'a self,
        severities: &'a [Severity],
        substring: &'a str,
    ) -> impl Iterator<Item = &'a Record> + 'a {
        self.records.iter().filter(move |record| {
            severities.contains(&record.severity) && record.message.contains(substring)
        })
    }

    /// Removes retained records, preserving sequence and cumulative drop counters.
    /// Explicitly cleared records do not count as capacity drops.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Returns cumulative capacity evictions and zero-capacity pushes since creation.
    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    /// Returns the maximum number of retained records.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
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
}
