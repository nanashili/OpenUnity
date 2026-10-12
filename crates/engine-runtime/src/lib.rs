//! Independent runtime ownership and reviewed GameObject/Transform subset.
#![forbid(unsafe_code)]

/// Native base/object ABI major 1, minor 0, defined by INFRA-OBJECT-001.
/// This contract is versioned independently of the managed hosting ABI.
pub const NATIVE_ABI_VERSION: u32 = 0x0001_0000;

mod context;
mod error;
mod hierarchy;
mod lifecycle;
mod math;
mod object_access;
mod object_lifetime;
mod physics_gravity;
mod scene;
mod timing;
mod transform;

pub use context::Runtime;
use context::TransformData;
use engine_object::ObjectKind;
pub use error::RuntimeError;
pub use lifecycle::LifecycleEvent;
pub use object_access::MAX_OBJECT_NAME_BYTES;
pub use timing::{TimeSnapshot, validate_time_configuration, validate_time_property};
