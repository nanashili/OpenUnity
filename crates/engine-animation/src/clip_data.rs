//! Validated, immutable-after-construction animation clip samples.

mod errors;
pub use errors::ClipDataError;

/// Values accepted by animation keyframes.
pub trait FiniteValue {
    /// Returns whether every scalar component is finite.
    fn is_finite(&self) -> bool;
}
impl FiniteValue for f32 {
    fn is_finite(&self) -> bool {
        f32::is_finite(*self)
    }
}
impl FiniteValue for [f32; 3] {
    fn is_finite(&self) -> bool {
        self.iter().all(|component| component.is_finite())
    }
}
impl FiniteValue for [f32; 4] {
    fn is_finite(&self) -> bool {
        self.iter().all(|component| component.is_finite())
    }
}
/// A validated positive finite clip duration.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ClipDuration(f32);

impl ClipDuration {
    /// Creates a duration in seconds.
    pub fn new(seconds: f32) -> Result<Self, ClipDataError> {
        if !seconds.is_finite() {
            return Err(ClipDataError::NonFinite { field: "duration" });
        }
        if seconds <= 0.0 {
            return Err(ClipDataError::NonPositiveDuration { value: seconds });
        }

        Ok(Self(seconds))
    }

    /// Returns the duration in seconds.
    pub fn seconds(self) -> f32 {
        self.0
    }
}
/// A finite key time before it is checked against a clip duration.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct KeyTime(f32);

impl KeyTime {
    /// Creates a finite key time in seconds.
    pub fn new(seconds: f32) -> Result<Self, ClipDataError> {
        if seconds.is_finite() {
            Ok(Self(seconds))
        } else {
            Err(ClipDataError::NonFinite { field: "key time" })
        }
    }

    /// Returns the key time in seconds.
    pub fn seconds(self) -> f32 {
        self.0
    }
}
/// One finite sample at a validated key time.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe<T> {
    time_seconds: KeyTime,
    value: T,
}

impl<T: FiniteValue> Keyframe<T> {
    /// Creates a keyframe after checking its time and value.
    pub fn new(time_seconds: f32, value: T) -> Result<Self, ClipDataError> {
        let time_seconds = KeyTime::new(time_seconds)?;
        if !value.is_finite() {
            return Err(ClipDataError::NonFiniteValue { index: 0 });
        }

        Ok(Self {
            time_seconds,
            value,
        })
    }

    /// Returns the key time in seconds.
    pub fn time_seconds(&self) -> f32 {
        self.time_seconds.seconds()
    }

    /// Returns the borrowed sample value.
    pub fn value(&self) -> &T {
        &self.value
    }
}
/// A non-empty, strictly ordered collection of keyframes.
#[derive(Clone, Debug, PartialEq)]
pub struct Channel<T> {
    keys: Box<[Keyframe<T>]>,
}

impl<T: FiniteValue> Channel<T> {
    /// Creates a channel without sorting or changing its input keys.
    pub fn try_new(keys: impl Into<Box<[Keyframe<T>]>>) -> Result<Self, ClipDataError> {
        let keys = keys.into();
        if keys.is_empty() {
            return Err(ClipDataError::EmptyTrack);
        }

        validate_key_order(&keys)?;

        Ok(Self { keys })
    }

    /// Returns the validated keys in input order.
    pub fn keys(&self) -> &[Keyframe<T>] {
        &self.keys
    }
}
/// Stable identity for a track in a clip.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TrackId(u64);

impl TrackId {
    /// Creates a track identity from its stable numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the stable numeric identity.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A named channel retained in input order.
#[derive(Clone, Debug, PartialEq)]
pub struct Track<T> {
    id: TrackId,
    channel: Channel<T>,
}

impl<T: FiniteValue> Track<T> {
    /// Creates a track from an identity and validated channel.
    pub fn new(id: TrackId, channel: Channel<T>) -> Self {
        Self { id, channel }
    }

    /// Returns the track identity.
    pub fn id(&self) -> TrackId {
        self.id
    }

    /// Returns the borrowed validated channel.
    pub fn channel(&self) -> &Channel<T> {
        &self.channel
    }
}

/// A finite clip with validated duration, tracks, times, and values.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClip<T> {
    duration: ClipDuration,
    tracks: Box<[Track<T>]>,
}

impl<T: FiniteValue> AnimationClip<T> {
    /// Creates a clip without sorting, clamping, or dropping tracks.
    pub fn try_new(
        duration_seconds: f32,
        tracks: impl Into<Box<[Track<T>]>>,
    ) -> Result<Self, ClipDataError> {
        let duration = ClipDuration::new(duration_seconds)?;
        let tracks = tracks.into();
        if tracks.is_empty() {
            return Err(ClipDataError::EmptyTrack);
        }

        for track in &tracks {
            validate_clip_bounds(duration, track.channel.keys())?;
        }

        Ok(Self { duration, tracks })
    }

    /// Returns the validated clip duration.
    pub fn duration(&self) -> ClipDuration {
        self.duration
    }

    /// Returns tracks in their validated input order.
    pub fn tracks(&self) -> &[Track<T>] {
        &self.tracks
    }
}

fn validate_key_order<T: FiniteValue>(keys: &[Keyframe<T>]) -> Result<(), ClipDataError> {
    for (index, pair) in keys.windows(2).enumerate() {
        let previous = pair[0].time_seconds();
        let current = pair[1].time_seconds();
        if current == previous {
            return Err(ClipDataError::DuplicateTime {
                index: index + 1,
                time_seconds: current,
            });
        }
        if current < previous {
            return Err(ClipDataError::UnorderedKeys { previous, current });
        }
    }

    Ok(())
}

fn validate_clip_bounds<T: FiniteValue>(
    duration: ClipDuration,
    keys: &[Keyframe<T>],
) -> Result<(), ClipDataError> {
    for (index, key) in keys.iter().enumerate() {
        let time_seconds = key.time_seconds();
        if time_seconds < 0.0 || time_seconds > duration.seconds() {
            return Err(ClipDataError::TimeOutsideClip {
                index,
                time_seconds,
            });
        }
    }

    Ok(())
}
