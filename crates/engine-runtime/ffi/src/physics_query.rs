//! C transport for the bounded owner-bound AABB query capability.

use crate::runtime_store::with_runtime;
use crate::status::{
    COUNT_OVERFLOW, INSUFFICIENT_CAPACITY, INVALID_BUFFER, object_id, runtime_error,
};
use engine_math::Vector;
use engine_physics_api::RaySegmentHit;

/// Query capability major 1, minor 0.
pub const PHYSICS_QUERY_ABI_VERSION: u32 = 0x0001_0000;
const VECTOR_BYTES: usize = size_of::<Vector>();
const MAX_OVERLAP_CAPACITY: i32 = 256;

#[derive(Clone, Copy)]
struct AddressRange {
    start: usize,
    end: usize,
}

fn range<T>(pointer: *const T, bytes: usize) -> Result<AddressRange, i32> {
    if pointer.is_null() || !pointer.is_aligned() {
        return Err(INVALID_BUFFER);
    }
    let start = pointer.addr();
    let end = start.checked_add(bytes).ok_or(INVALID_BUFFER)?;
    Ok(AddressRange { start, end })
}

fn overlaps(first: AddressRange, second: AddressRange) -> bool {
    first.start < second.end && second.start < first.end
}

fn vector_input(pointer: *const f32) -> Result<AddressRange, i32> {
    range(pointer, VECTOR_BYTES)
}

fn output_range<T>(pointer: *mut T) -> Result<AddressRange, i32> {
    range(pointer.cast_const(), size_of::<T>())
}

fn copy_vector(pointer: *const f32) -> Vector {
    let mut value = [0.0; 3];
    // SAFETY: caller memory passed vector_input and remains readable for the call.
    unsafe { std::ptr::copy_nonoverlapping(pointer, value.as_mut_ptr(), value.len()) };
    value
}

fn copy_pair(min: *const f32, max: *const f32) -> Result<(Vector, Vector), i32> {
    vector_input(min)?;
    vector_input(max)?;
    Ok((copy_vector(min), copy_vector(max)))
}

fn validate_ray_outputs(owner: *mut i64, parameter: *mut f32, point: *mut f32) -> Result<(), i32> {
    let owner = output_range(owner)?;
    let parameter = output_range(parameter)?;
    let point = range(point.cast_const(), VECTOR_BYTES)?;
    if overlaps(owner, parameter) || overlaps(owner, point) || overlaps(parameter, point) {
        return Err(INVALID_BUFFER);
    }
    Ok(())
}

fn validate_overlap_outputs(owners: *mut i64, capacity: i32, count: *mut i32) -> Result<(), i32> {
    if !(0..=MAX_OVERLAP_CAPACITY).contains(&capacity) {
        return Err(INVALID_BUFFER);
    }
    let count = output_range(count)?;
    if capacity == 0 {
        if !owners.is_null() {
            range(owners.cast_const(), 0)?;
        }
        return Ok(());
    }

    let elements = usize::try_from(capacity).map_err(|_| INVALID_BUFFER)?;
    let bytes = elements
        .checked_mul(size_of::<i64>())
        .ok_or(INVALID_BUFFER)?;
    let owners = range(owners.cast_const(), bytes)?;
    if overlaps(owners, count) {
        return Err(INVALID_BUFFER);
    }
    Ok(())
}

/// Return the query ABI version without runtime lookup.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_query_abi_version() -> u32 {
    PHYSICS_QUERY_ABI_VERSION
}

/// Register one explicit world-space AABB for a live GameObject owner.
///
/// # Safety
/// `min` and `max` must each point to three readable, properly aligned `f32`
/// values that remain valid for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_physics_aabb_add(
    runtime: i64,
    owner: i64,
    min: *const f32,
    max: *const f32,
) -> i32 {
    let (min, max) = match copy_pair(min, max) {
        Ok(bounds) => bounds,
        Err(error) => return error,
    };
    with_runtime(runtime, |runtime| {
        let owner = match object_id(owner) {
            Ok(owner) => owner,
            Err(error) => return runtime_error(error),
        };
        runtime
            .physics_aabb_add(owner, min, max)
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Return the nearest closed-segment hit, or an explicit zeroed miss.
///
/// # Safety
/// `origin` and `displacement` must each point to three readable, properly
/// aligned `f32` values. The output pointers must reference writable,
/// properly aligned values for the call's duration, and their ranges must be
/// mutually disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_physics_raycast_segment(
    runtime: i64,
    origin: *const f32,
    displacement: *const f32,
    out_owner: *mut i64,
    out_parameter: *mut f32,
    out_point: *mut f32,
) -> i32 {
    if let Err(error) = validate_ray_outputs(out_owner, out_parameter, out_point) {
        return error;
    }
    let (origin, displacement) = match copy_pair(origin, displacement) {
        Ok(values) => values,
        Err(error) => return error,
    };
    let hit = match with_runtime(runtime, |runtime| {
        runtime.physics_raycast_segment(origin, displacement)
    }) {
        Ok(Ok(hit)) => hit,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };

    let (owner, parameter, point) = hit.map_or((0, 0.0, [0.0; 3]), |hit: RaySegmentHit| {
        (hit.owner().to_raw(), hit.parameter(), hit.point())
    });
    // SAFETY: all output ranges were validated before runtime lookup.
    unsafe {
        std::ptr::write(out_owner, owner);
        std::ptr::write(out_parameter, parameter);
        std::ptr::copy_nonoverlapping(point.as_ptr(), out_point, point.len());
    }
    0
}

/// Return all intersecting owners in ascending ObjectId order.
///
/// # Safety
/// `min` and `max` must each point to three readable, properly aligned `f32`
/// values. `out_count` must point to one writable, properly aligned `i32`.
/// When `capacity` is positive, `out_owners` must point to `capacity`
/// writable, properly aligned `i64` values; the owner and count ranges must
/// be disjoint. All ranges must remain valid for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_physics_overlap_aabb(
    runtime: i64,
    min: *const f32,
    max: *const f32,
    out_owners: *mut i64,
    capacity: i32,
    out_count: *mut i32,
) -> i32 {
    if let Err(error) = validate_overlap_outputs(out_owners, capacity, out_count) {
        return error;
    }
    let (min, max) = match copy_pair(min, max) {
        Ok(bounds) => bounds,
        Err(error) => return error,
    };
    let owners = match with_runtime(runtime, |runtime| runtime.physics_overlap_aabb(min, max)) {
        Ok(Ok(owners)) => owners,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };
    if owners.len() > usize::try_from(capacity).unwrap_or(0) {
        return INSUFFICIENT_CAPACITY;
    }

    // SAFETY: capacity and owner/count disjointness were validated; only n slots are written.
    unsafe {
        for (index, owner) in owners.iter().enumerate() {
            std::ptr::write(out_owners.add(index), owner.to_raw());
        }
        std::ptr::write(
            out_count,
            i32::try_from(owners.len()).unwrap_or(COUNT_OVERFLOW),
        );
    }
    0
}
