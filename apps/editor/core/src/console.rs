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
mod tests;
