//! GameObject hierarchy activity and committed Behaviour flags.

use super::protocol::{flag, status};
use crate::runtime_store::with_runtime;
use crate::status::{object_id, runtime_error};

/// Query activeSelf (hierarchy=0) or activeInHierarchy (hierarchy=1).
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_active(runtime: i64, owner: i64, hierarchy: i32) -> i32 {
    let hierarchy = match flag(hierarchy) {
        Ok(value) => value,
        Err(error) => return error,
    };
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|id| runtime.gameobject_active(id, hierarchy))
            .map_or_else(runtime_error, i32::from)
    })
    .unwrap_or_else(|error| error)
}

macro_rules! flag_mutation {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64, object: i64, value: i32) -> i32 {
            let value = match flag(value) {
                Ok(value) => value,
                Err(error) => return error,
            };
            with_runtime(runtime, |runtime| {
                status(object_id(object).and_then(|id| runtime.$method(id, value)))
            })
            .unwrap_or_else(|error| error)
        }
    };
}

flag_mutation!(
    ou_gameobject_set_active,
    gameobject_set_active,
    "Set owner activity before a separate mutation callback pump."
);
flag_mutation!(
    ou_behaviour_set_enabled,
    behaviour_set_enabled,
    "Set script enabled state before a separate mutation callback pump."
);

macro_rules! behaviour_query {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64, component: i64) -> i32 {
            with_runtime(runtime, |runtime| {
                object_id(component)
                    .and_then(|id| runtime.$method(id))
                    .map_or_else(runtime_error, i32::from)
            })
            .unwrap_or_else(|error| error)
        }
    };
}

behaviour_query!(
    ou_behaviour_enabled,
    behaviour_enabled,
    "Read the committed script's native enabled flag."
);
behaviour_query!(
    ou_behaviour_active,
    behaviour_active,
    "Read enabled and effective native hierarchy activity."
);
