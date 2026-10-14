//! Independent runtime ownership and reviewed GameObject/Transform subset.
#![forbid(unsafe_code)]

/// Native base/object ABI major 1, minor 0, defined by INFRA-OBJECT-001.
/// This contract is versioned independently of the managed hosting ABI.
pub const NATIVE_ABI_VERSION: u32 = 0x0001_0000;

mod context;
mod error;
mod hierarchy;
mod hierarchy_order;
mod input;
mod input_api;
mod lifecycle;
mod math;
mod navigation;
mod object_access;
mod object_lifetime;
mod physics;
mod physics_gravity;
mod physics_scene;
mod scene;
mod scene_snapshot;
mod timing;
mod transform;
mod transform_detach;
mod transform_vectors;

pub use context::Runtime;
use context::TransformData;
use engine_object::ObjectKind;
pub use error::RuntimeError;
pub use input::{InputPreparationError, RuntimeInputError};
pub use lifecycle::LifecycleEvent;
pub use navigation::{NavigationRoute, RuntimeNavigationError};
pub use object_access::MAX_OBJECT_NAME_BYTES;
pub use physics_scene::{PhysicsSceneBodyState, PhysicsSceneDimension};
pub use scene_snapshot::{
    SNAPSHOT_MAX_ROWS, SNAPSHOT_ROW_STRIDE, SNAPSHOT_VERSION, SceneSnapshot, SceneSnapshotRow,
    SnapshotState,
};
pub use timing::{TimeSnapshot, validate_time_configuration, validate_time_property};
