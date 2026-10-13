use engine_render_api::AdapterPolicy;

use crate::physics_scene::arguments::{Dimension, dimension_from_token, policy_from_token};

use super::error::LiveSceneError;

pub(crate) const LIVE_PHYSICS_SCENE_FLAG: &str = "--live-physics-scene";

const FRAMES_OPTION: &str = "--frames";
const USAGE: &str =
    "usage: --live-physics-scene <2d|3d> <hardware|software> [--frames <count:1..>]";

pub(crate) struct Arguments {
    pub(crate) dimension: Dimension,
    pub(crate) policy: AdapterPolicy,
    pub(crate) frame_limit: Option<u64>,
}

impl Arguments {
    pub(crate) fn parse(values: &[String]) -> Result<Self, LiveSceneError> {
        let (dimension, policy, frame_limit) = match values {
            [flag, dimension, policy] if flag == LIVE_PHYSICS_SCENE_FLAG => {
                (dimension, policy, None)
            }
            [flag, dimension, policy, option, frames]
                if flag == LIVE_PHYSICS_SCENE_FLAG && option == FRAMES_OPTION =>
            {
                (dimension, policy, Some(parse_frame_limit(frames)?))
            }
            _ => return Err(LiveSceneError::new(USAGE)),
        };

        let dimension = dimension_from_token(dimension).ok_or_else(|| {
            LiveSceneError::new(format!("invalid physics dimension `{dimension}`; {USAGE}"))
        })?;
        let policy = policy_from_token(policy).ok_or_else(|| {
            LiveSceneError::new(format!(
                "adapter policy must be `hardware` or `software`; {USAGE}"
            ))
        })?;

        Ok(Self {
            dimension,
            policy,
            frame_limit,
        })
    }
}

fn parse_frame_limit(value: &str) -> Result<u64, LiveSceneError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(LiveSceneError::new(format!(
            "frame count must be a positive unsigned integer; {USAGE}"
        )));
    }

    let frame_limit = value.parse::<u64>().map_err(|_| {
        LiveSceneError::new(format!(
            "frame count must be a positive unsigned integer; {USAGE}"
        ))
    })?;

    if frame_limit == 0 {
        return Err(LiveSceneError::new(format!(
            "frame count must be at least one; {USAGE}"
        )));
    }

    Ok(frame_limit)
}
