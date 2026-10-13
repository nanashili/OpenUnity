use engine_math::{Quaternion, Vector, lerp, slerp};

use super::{TransformPose, TransformSample, TransformTrack, TransformTrackError};
use crate::clip_data::TrackId;

/// A transform track paired with its requested blend weight.
pub struct WeightedTrack<'a> {
    /// Borrowed transform track to sample and blend.
    pub track: &'a TransformTrack,
    /// Nonnegative weight, normalized when the blend is built.
    pub weight: f32,
}

/// An immutable, sampled blend for one transform target and time.
pub struct TransformBlend<'a> {
    target: TrackId,
    entries: Vec<(WeightedTrack<'a>, TransformSample)>,
}

impl<'a> TransformBlend<'a> {
    /// Samples and globally normalizes an ordered set of weighted tracks.
    pub fn build(
        target: TrackId,
        tracks: &[WeightedTrack<'a>],
        time_seconds: f32,
    ) -> Result<Self, TransformTrackError> {
        super::transform_sampling::validate_time(time_seconds)?;
        let total_weight = validate_weights(target, tracks)?;
        let mut entries = Vec::with_capacity(tracks.len());

        for entry in tracks {
            entries.push((
                WeightedTrack {
                    track: entry.track,
                    weight: entry.weight / total_weight,
                },
                entry.track.sample(time_seconds)?,
            ));
        }

        Ok(Self { target, entries })
    }

    /// Applies the sampled blend atomically to a validated base pose.
    pub fn apply(
        &self,
        target: TrackId,
        base: TransformPose,
    ) -> Result<TransformPose, TransformTrackError> {
        if target != self.target {
            return Err(TransformTrackError::TargetMismatch);
        }
        super::validate_pose(base)?;

        Ok(TransformPose {
            position: blend_vector(base.position, &self.entries, |sample| sample.position)?,
            rotation: blend_rotation(base.rotation, &self.entries)?,
            scale: blend_vector(base.scale, &self.entries, |sample| sample.scale)?,
        })
    }
}

fn validate_weights(
    target: TrackId,
    tracks: &[WeightedTrack<'_>],
) -> Result<f32, TransformTrackError> {
    if tracks.is_empty() {
        return Err(TransformTrackError::NoTracks);
    }

    let mut total = 0.0_f32;
    for entry in tracks {
        if entry.track.target() != target {
            return Err(TransformTrackError::TargetMismatch);
        }
        if !entry.weight.is_finite() {
            return Err(TransformTrackError::NonFiniteWeight);
        }
        if entry.weight < 0.0 {
            return Err(TransformTrackError::NegativeWeight);
        }
        total += entry.weight;
        if !total.is_finite() {
            return Err(TransformTrackError::NonFiniteWeight);
        }
    }

    (total > 0.0)
        .then_some(total)
        .ok_or(TransformTrackError::ZeroWeightSum)
}

fn blend_vector<F>(
    base: Vector,
    entries: &[(WeightedTrack<'_>, TransformSample)],
    select: F,
) -> Result<Vector, TransformTrackError>
where
    F: Fn(TransformSample) -> Option<Vector>,
{
    let mut value = None;
    let mut accumulated = 0.0_f32;

    for (entry, sample) in entries {
        if entry.weight == 0.0 {
            continue;
        }
        let next = select(*sample).unwrap_or(base);
        value = Some(match value {
            None => next,
            Some(current) => {
                let parameter = entry.weight / (accumulated + entry.weight);
                lerp(current, next, parameter).map_err(TransformTrackError::Numeric)?
            }
        });
        accumulated += entry.weight;
    }

    Ok(value.unwrap_or(base))
}

fn blend_rotation(
    base: Quaternion,
    entries: &[(WeightedTrack<'_>, TransformSample)],
) -> Result<Quaternion, TransformTrackError> {
    let mut value = None;
    let mut accumulated = 0.0_f32;

    for (entry, sample) in entries {
        if entry.weight == 0.0 {
            continue;
        }
        let next = sample.rotation.unwrap_or(base);
        let parameter = entry.weight / (accumulated + entry.weight);
        value = Some(
            match value {
                None => slerp(next, next, 0.0),
                Some(current) => slerp(current, next, parameter),
            }
            .map_err(TransformTrackError::Numeric)?,
        );
        accumulated += entry.weight;
    }

    Ok(value.unwrap_or(base))
}
