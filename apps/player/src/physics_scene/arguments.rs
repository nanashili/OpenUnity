use engine_render_api::{AdapterPolicy, TargetSize};
use std::path::PathBuf;

use super::error::PhysicsSceneError;

pub(crate) const PHYSICS_SCENE_FLAG: &str = "--physics-scene";

const MIN_SIMULATION_STEPS: usize = 60;
const MAX_SIMULATION_STEPS: usize = 600;

const USAGE: &str = "usage: --physics-scene <2d|3d> <steps:60..600> <width> <height> <hardware|software> <fresh-output-dir>";

/// Dimension selected for the independently-authored native collision scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Dimension {
    TwoD,
    ThreeD,
}

impl Dimension {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::TwoD => "2d",
            Self::ThreeD => "3d",
        }
    }
}

/// Validated values for one bounded physics-scene invocation.
pub(super) struct Arguments {
    pub(super) dimension: Dimension,
    pub(super) steps: usize,
    pub(super) target_size: TargetSize,
    pub(super) policy: AdapterPolicy,
    pub(super) output: PathBuf,
}

impl Arguments {
    pub(super) fn parse(values: &[String]) -> Result<Self, PhysicsSceneError> {
        let [flag, dimension, steps, width, height, policy, output] = values else {
            return Err(PhysicsSceneError::new(USAGE));
        };

        if flag != PHYSICS_SCENE_FLAG {
            return Err(PhysicsSceneError::new(USAGE));
        }

        let dimension = parse_dimension(dimension)?;
        let steps = parse_steps(steps)?;
        let width = parse_unsigned("width", width)?;
        let height = parse_unsigned("height", height)?;
        let target_size = TargetSize::try_new(width, height)
            .map_err(|error| PhysicsSceneError::debug("invalid render size", error))?;

        let policy = parse_policy(policy)?;
        let output = PathBuf::from(output);

        Ok(Self {
            dimension,
            steps,
            target_size,
            policy,
            output,
        })
    }
}

pub(crate) fn dimension_from_token(value: &str) -> Option<Dimension> {
    match value {
        "2d" => Some(Dimension::TwoD),
        "3d" => Some(Dimension::ThreeD),
        _ => None,
    }
}

fn parse_dimension(value: &str) -> Result<Dimension, PhysicsSceneError> {
    dimension_from_token(value).ok_or_else(|| {
        PhysicsSceneError::new(format!("invalid physics dimension `{value}`; {USAGE}"))
    })
}

fn parse_steps(value: &str) -> Result<usize, PhysicsSceneError> {
    let parsed_steps = parse_unsigned("steps", value)?;
    let steps = usize::try_from(parsed_steps)
        .map_err(|_| PhysicsSceneError::new("steps exceed this platform's supported range"))?;
    if !(MIN_SIMULATION_STEPS..=MAX_SIMULATION_STEPS).contains(&steps) {
        return Err(PhysicsSceneError::new(format!(
            "steps must be between {MIN_SIMULATION_STEPS} and {MAX_SIMULATION_STEPS}"
        )));
    }
    Ok(steps)
}

fn parse_unsigned(field: &str, value: &str) -> Result<u32, PhysicsSceneError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(PhysicsSceneError::new(format!(
            "{field} must be an unsigned decimal integer"
        )));
    }

    value
        .parse::<u32>()
        .map_err(|_| PhysicsSceneError::new(format!("{field} is outside the supported range")))
}

pub(crate) fn policy_from_token(value: &str) -> Option<AdapterPolicy> {
    match value {
        "hardware" => Some(AdapterPolicy::Hardware),
        "software" => Some(AdapterPolicy::Software),
        _ => None,
    }
}

fn parse_policy(value: &str) -> Result<AdapterPolicy, PhysicsSceneError> {
    policy_from_token(value).ok_or_else(|| {
        PhysicsSceneError::new(format!(
            "adapter policy must be `hardware` or `software`; {USAGE}"
        ))
    })
}
