//! Existing integer lifecycle frame and phase entrypoints.

use super::protocol::status;
use crate::runtime_store::with_runtime;
use crate::status::INVALID_KIND;

/// Read the actual per-runtime native frame counter.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_frame_count(runtime: i64) -> i32 {
    with_runtime(runtime, |runtime| runtime.lifecycle_frame_count()).unwrap_or_else(|error| error)
}

macro_rules! frame_mutation {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64) -> i32 {
            with_runtime(runtime, |runtime| status(runtime.$method())).unwrap_or_else(|error| error)
        }
    };
}

frame_mutation!(
    ou_lifecycle_begin_frame,
    lifecycle_begin_frame,
    "Begin one frame, rejecting unfinished or recursive scheduling."
);
frame_mutation!(
    ou_lifecycle_flush_destroy,
    lifecycle_flush_destroy,
    "Enter a destruction-only boundary without changing the frame counter."
);

/// Begin the next ordered native frame phase, 1 through 5.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_begin_phase(runtime: i64, phase: i32) -> i32 {
    if !(1..=5).contains(&phase) {
        return INVALID_KIND;
    }
    with_runtime(runtime, |runtime| {
        status(runtime.lifecycle_begin_phase(phase))
    })
    .unwrap_or_else(|error| error)
}
