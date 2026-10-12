//! Finite vector, quaternion, bounds, ray, and affine numeric primitives.
#![forbid(unsafe_code)]

mod aabb;
mod affine;
mod error;
mod plane_ray;
mod quaternion;
mod quaternion_helpers;
mod snapshot_affine;
mod vector_arithmetic;
mod vector_interpolation;
mod vector_norms;
mod vector_products;

pub use aabb::Aabb;
pub use affine::Affine3x4;
pub use error::MathError;
pub use plane_ray::Plane;
pub use quaternion::{conjugate, multiply, normalize, rotate};
pub use quaternion_helpers::slerp;
pub use snapshot_affine::SnapshotAffine3x4;
pub use vector_arithmetic::{add, divide, finite, scale, subtract, vector};
pub use vector_interpolation::lerp;
pub use vector_norms::{length, normalize_vector};
pub use vector_products::{cross, dot};

/// Three binary32 vector components.
pub type Vector = [f32; 3];
/// Quaternion components in x, y, z, w order.
pub type Quaternion = [f32; 4];
