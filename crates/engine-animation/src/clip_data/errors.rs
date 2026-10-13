use std::fmt;

/// Typed failures when clip data cannot be admitted.
#[derive(Clone, Debug, PartialEq)]
pub enum ClipDataError {
    /// A duration or key time was not finite.
    NonFinite {
        /// Name of the finite field that failed validation.
        field: &'static str,
    },
    /// A duration was finite but not positive.
    NonPositiveDuration {
        /// The rejected duration value.
        value: f32,
    },
    /// A clip or channel contained no entries.
    EmptyTrack,
    /// A key time was outside the clip domain.
    TimeOutsideClip {
        /// Position of the out-of-domain key.
        index: usize,
        /// Rejected key time in seconds.
        time_seconds: f32,
    },
    /// Key times decreased between adjacent entries.
    UnorderedKeys {
        /// Earlier key time in the invalid pair.
        previous: f32,
        /// Later key time in the invalid pair.
        current: f32,
    },
    /// Adjacent keys shared a timestamp.
    DuplicateTime {
        /// Position of the duplicate key.
        index: usize,
        /// Repeated key time in seconds.
        time_seconds: f32,
    },
    /// A key value contained a non-finite component.
    NonFiniteValue {
        /// Position associated with the invalid value.
        index: usize,
    },
}

impl fmt::Display for ClipDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(formatter, "{field} must be finite"),
            Self::NonPositiveDuration { value } => {
                write!(formatter, "duration must be positive: {value:?}")
            }
            Self::EmptyTrack => formatter.write_str("clip and channels must not be empty"),
            Self::TimeOutsideClip {
                index,
                time_seconds,
            } => write!(
                formatter,
                "key {index} is outside the clip at {time_seconds:?}"
            ),
            Self::UnorderedKeys { previous, current } => write!(
                formatter,
                "key times are unordered: {previous:?} then {current:?}"
            ),
            Self::DuplicateTime {
                index,
                time_seconds,
            } => write!(formatter, "key {index} duplicates time {time_seconds:?}"),
            Self::NonFiniteValue { index } => {
                write!(formatter, "key {index} has a non-finite value")
            }
        }
    }
}

impl std::error::Error for ClipDataError {}
