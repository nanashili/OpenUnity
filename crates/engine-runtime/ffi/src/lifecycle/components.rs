//! Script reservation/publication/destruction requests and committed enumeration.

use super::protocol::status;
use crate::runtime_store::with_runtime;
use crate::status::{INVALID_KIND, object_id, runtime_error};
use engine_object::ObjectId;

/// Reserve a hidden custom MonoBehaviour before managed construction.
// SAFETY: unique reserved export; only fixed-width integers cross the boundary.
#[unsafe(no_mangle)]
pub extern "C" fn ou_component_reserve(
    runtime: i64,
    owner: i64,
    kind: i32,
    execution_order: i32,
) -> i64 {
    if kind != 4 {
        return i64::from(INVALID_KIND);
    }
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|owner| runtime.component_reserve(owner, execution_order))
            .map_or_else(|error| i64::from(runtime_error(error)), ObjectId::to_raw)
    })
    .unwrap_or_else(i64::from)
}

macro_rules! object_mutation {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64, object: i64) -> i32 {
            with_runtime(runtime, |runtime| {
                status(object_id(object).and_then(|id| runtime.$method(id)))
            })
            .unwrap_or_else(|error| error)
        }
    };
}

object_mutation!(
    ou_component_commit,
    component_commit,
    "Publish a constructed reservation atomically."
);
object_mutation!(
    ou_component_abort,
    component_abort,
    "Consume an unpublished reservation after construction fails."
);
object_mutation!(
    ou_object_destroy_deferred,
    object_destroy_deferred,
    "Queue destruction, preserving identities until the destruction phase completes."
);

/// Return Transform-first committed component count.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_component_count(runtime: i64, owner: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|id| runtime.gameobject_component_count(id))
            .unwrap_or_else(runtime_error)
    })
    .unwrap_or_else(|error| error)
}

/// Return a committed component ID at a Transform-first index.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_component_at(runtime: i64, owner: i64, index: i32) -> i64 {
    if index < 0 {
        return i64::from(INVALID_KIND);
    }
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|id| runtime.gameobject_component_at(id, index))
            .map_or_else(|error| i64::from(runtime_error(error)), ObjectId::to_raw)
    })
    .unwrap_or_else(i64::from)
}
