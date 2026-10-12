//! Storage-independent capability versions for the original object/name/Transform surface.

pub use engine_runtime::NATIVE_ABI_VERSION as ABI_VERSION;

/// Independent object name ABI major 1, minor 0.
pub const OBJECT_NAME_ABI_VERSION: u32 = 0x0001_0000;

/// Independent Transform extension major 1, minor 0.
pub const TRANSFORM_ABI_VERSION: u32 = 0x0001_0000;

/// Return the integer-only ABI version without accessing the runtime store.
// SAFETY: the ou_ namespace is reserved for this ABI and has one definition per symbol.
#[unsafe(no_mangle)]
pub extern "C" fn ou_abi_version() -> u32 {
    ABI_VERSION
}

/// Return the independent name-extension version without accessing storage.
/// Clients must check this version and [`ou_abi_version`] before name calls.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_name_abi_version() -> u32 {
    OBJECT_NAME_ABI_VERSION
}

/// Return the independent typed-extension version without storage access.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_abi_version() -> u32 {
    TRANSFORM_ABI_VERSION
}
