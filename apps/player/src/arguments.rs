//! Typed positional arguments for the bounded `--render-scene` player route.
//!
//! This module parses route inputs only. The JSON render manifest remains
//! owned by its separate player module, while scene import,
//! output admission, session capture, rendering, and cleanup retain their
//! reviewed owners.

use engine_render_api::{AdapterPolicy, TargetSize};
use std::path::{Path, PathBuf};

/// The executable flag selecting the render route.
pub const RENDER_SCENE_FLAG: &str = "--render-scene";

/// The maximum number of explicit same-session ticks accepted by the route.
pub const MAX_TICKS: usize = 255;

/// Typed inputs consumed by the render-route orchestrator.
///
/// The parser accepts exactly thirteen values after the route flag, followed
/// by at most [`MAX_TICKS`] signed decimal callback inputs. Dimensions must
/// each be between 1 and 2048, and policy must be `hardware` or `software`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderArguments {
    root: PathBuf,
    hostfxr: PathBuf,
    bridge: PathBuf,
    config: PathBuf,
    assembly: PathBuf,
    project: PathBuf,
    scene: PathBuf,
    bindings: PathBuf,
    manifest: PathBuf,
    target_size: TargetSize,
    policy: AdapterPolicy,
    output: PathBuf,
    ticks: Vec<i64>,
}

/// Failures while converting the fixed positional route shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgumentsError {
    /// The flag or fixed argument count is invalid.
    InvalidShape,
    /// A width, height, or tick is not a signed/unsigned decimal integer.
    InvalidInteger { field: &'static str, value: String },
    /// The adapter policy is outside the exact admitted vocabulary.
    InvalidPolicy { value: String },
    /// The renderer rejected the requested dimensions.
    InvalidTargetSize { width: u32, height: u32 },
    /// More than the admitted number of ticks was supplied.
    TooManyTicks { count: usize },
}

impl RenderArguments {
    /// Parse the fourteen fixed values followed by zero to 255 ticks.
    ///
    /// The fixed order is: flag, root, hostfxr, bridge, config, assembly,
    /// project, scene, bindings, manifest, width, height, policy, and output.
    /// The managed session is initialized with the contract's fixed value
    /// `0`; no initial argument is accepted by this route. Unsigned dimensions
    /// require decimal digits only, while each tick may have one leading sign.
    pub fn parse(args: &[String]) -> Result<Self, ArgumentsError> {
        let [
            flag,
            root,
            hostfxr,
            bridge,
            config,
            assembly,
            project,
            scene,
            bindings,
            manifest,
            width,
            height,
            policy,
            output,
            ticks @ ..,
        ] = args
        else {
            return Err(ArgumentsError::InvalidShape);
        };

        if flag != RENDER_SCENE_FLAG {
            return Err(ArgumentsError::InvalidShape);
        }
        if ticks.len() > MAX_TICKS {
            return Err(ArgumentsError::TooManyTicks { count: ticks.len() });
        }

        let width = parse_u32("width", width)?;
        let height = parse_u32("height", height)?;
        let target_size = TargetSize::try_new(width, height)
            .map_err(|_| ArgumentsError::InvalidTargetSize { width, height })?;
        let policy = parse_policy(policy)?;
        let ticks = ticks
            .iter()
            .map(|value| parse_i64("tick", value))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            root: PathBuf::from(root),
            hostfxr: PathBuf::from(hostfxr),
            bridge: PathBuf::from(bridge),
            config: PathBuf::from(config),
            assembly: PathBuf::from(assembly),
            project: PathBuf::from(project),
            scene: PathBuf::from(scene),
            bindings: PathBuf::from(bindings),
            manifest: PathBuf::from(manifest),
            target_size,
            policy,
            output: PathBuf::from(output),
            ticks,
        })
    }

    /// Return the explicit .NET installation root used by hostfxr.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the explicit hostfxr path.
    pub fn hostfxr(&self) -> &Path {
        &self.hostfxr
    }

    /// Return the managed bridge assembly path.
    pub fn bridge(&self) -> &Path {
        &self.bridge
    }

    /// Return the managed runtime configuration path.
    pub fn config(&self) -> &Path {
        &self.config
    }

    /// Return the user assembly path.
    pub fn assembly(&self) -> &Path {
        &self.assembly
    }

    /// Return the scene import root.
    pub fn project(&self) -> &Path {
        &self.project
    }

    /// Return the project-relative saved scene path.
    pub fn scene(&self) -> &Path {
        &self.scene
    }

    /// Return the project-relative binding manifest path.
    pub fn bindings(&self) -> &Path {
        &self.bindings
    }

    /// Return the bounded render manifest path.
    pub fn manifest(&self) -> &Path {
        &self.manifest
    }

    /// Return the validated render target dimensions.
    pub const fn target_size(&self) -> TargetSize {
        self.target_size
    }

    /// Return the exact adapter policy requested by the caller.
    pub const fn policy(&self) -> AdapterPolicy {
        self.policy
    }

    /// Return the fresh destination directory token.
    pub fn output(&self) -> &Path {
        &self.output
    }

    /// Return tick callback inputs in their authored command-line order.
    pub fn ticks(&self) -> &[i64] {
        &self.ticks
    }
}

fn parse_u32(field: &'static str, value: &str) -> Result<u32, ArgumentsError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ArgumentsError::InvalidInteger {
            field,
            value: value.into(),
        });
    }

    value.parse().map_err(|_| ArgumentsError::InvalidInteger {
        field,
        value: value.into(),
    })
}

fn parse_i64(field: &'static str, value: &str) -> Result<i64, ArgumentsError> {
    let digits = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ArgumentsError::InvalidInteger {
            field,
            value: value.into(),
        });
    }

    value.parse().map_err(|_| ArgumentsError::InvalidInteger {
        field,
        value: value.into(),
    })
}

fn parse_policy(value: &str) -> Result<AdapterPolicy, ArgumentsError> {
    match value {
        "hardware" => Ok(AdapterPolicy::Hardware),
        "software" => Ok(AdapterPolicy::Software),
        _ => Err(ArgumentsError::InvalidPolicy {
            value: value.into(),
        }),
    }
}
