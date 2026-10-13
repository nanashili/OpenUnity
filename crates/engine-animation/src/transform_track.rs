//! Finite transform-channel sampling and ordered blending.

use engine_math::{MathError, Quaternion, Vector, finite};

use crate::clip_data::{Channel, TrackId};
use crate::curve::{Curve, CurveError, InterpolationMode};

mod transform_blend;
mod transform_sampling;

/// Weighted transform blending primitives exported by this module.
pub use transform_blend::{TransformBlend, WeightedTrack};

#[derive(Clone, Copy, Debug, PartialEq)]
/// A complete transform used as the base for blend application.
pub struct TransformPose {
    /// World or local position, according to the owning animation contract.
    pub position: Vector,
    /// Normalized rotation quaternion.
    pub rotation: Quaternion,
    /// Scale vector.
    pub scale: Vector,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// The optional channel values sampled from one transform track.
pub struct TransformSample {
    /// Sampled position, when the track contains a position channel.
    pub position: Option<Vector>,
    /// Sampled rotation, when the track contains a rotation channel.
    pub rotation: Option<Quaternion>,
    /// Sampled scale, when the track contains a scale channel.
    pub scale: Option<Vector>,
}

/// A validated set of optional transform channels for one animation target.
pub struct TransformTrack {
    target: TrackId,
    position: Option<Curve<Vector>>,
    rotation: Option<Curve<Quaternion>>,
    scale: Option<Curve<Vector>>,
}

#[derive(Clone, Debug, PartialEq)]
/// Errors raised while constructing, sampling, or applying transform tracks.
pub enum TransformTrackError {
    /// No position, rotation, or scale channel was supplied.
    NoTracks,
    /// A blend weight or its running sum was not finite.
    NonFiniteWeight,
    /// A blend weight was negative.
    NegativeWeight,
    /// All supplied blend weights were zero.
    ZeroWeightSum,
    /// A base or sampled transform contained a non-finite component.
    NonFinitePose,
    /// A track and blend application target did not match.
    TargetMismatch,
    /// A requested sample time was not finite.
    NonFiniteTime,
    /// A curve rejected construction or sampling.
    Curve(CurveError),
    /// The math layer rejected an interpolation or normalization operation.
    Numeric(MathError),
}

impl TransformTrack {
    /// Builds a track from validated clip-data channels.
    pub fn new(
        target: TrackId,
        position: Option<Channel<Vector>>,
        rotation: Option<Channel<Quaternion>>,
        scale: Option<Channel<Vector>>,
    ) -> Result<Self, TransformTrackError> {
        if position.is_none() && rotation.is_none() && scale.is_none() {
            return Err(TransformTrackError::NoTracks);
        }

        Ok(Self {
            target,
            position: build_vector_curve(position)?,
            rotation: build_rotation_curve(rotation)?,
            scale: build_vector_curve(scale)?,
        })
    }

    /// Returns the target identity associated with this track.
    pub fn target(&self) -> TrackId {
        self.target
    }

    /// Samples all present channels at a seconds-based, clamped time.
    pub fn sample(&self, time_seconds: f32) -> Result<TransformSample, TransformTrackError> {
        transform_sampling::validate_time(time_seconds)?;

        Ok(TransformSample {
            position: transform_sampling::sample_vector(&self.position, time_seconds)?,
            rotation: transform_sampling::sample_rotation(&self.rotation, time_seconds)?,
            scale: transform_sampling::sample_vector(&self.scale, time_seconds)?,
        })
    }
}

fn build_vector_curve(
    channel: Option<Channel<Vector>>,
) -> Result<Option<Curve<Vector>>, TransformTrackError> {
    channel
        .map(|channel| Curve::<Vector>::new(channel, InterpolationMode::Linear))
        .transpose()
        .map_err(TransformTrackError::Curve)
}

fn build_rotation_curve(
    channel: Option<Channel<Quaternion>>,
) -> Result<Option<Curve<Quaternion>>, TransformTrackError> {
    channel
        .map(|channel| Curve::<Quaternion>::new(channel, InterpolationMode::Linear))
        .transpose()
        .map_err(TransformTrackError::Curve)
}

pub(super) fn validate_pose(pose: TransformPose) -> Result<(), TransformTrackError> {
    finite(&pose.position).map_err(|_| TransformTrackError::NonFinitePose)?;
    finite(&pose.rotation).map_err(|_| TransformTrackError::NonFinitePose)?;
    finite(&pose.scale).map_err(|_| TransformTrackError::NonFinitePose)?;
    Ok(())
}
