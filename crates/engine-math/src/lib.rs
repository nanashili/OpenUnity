//! Independently authored affine and Hamilton quaternion numeric primitives.
#![forbid(unsafe_code)]

mod error;
mod quaternion;
mod vector_arithmetic;

pub use error::MathError;
pub use quaternion::{conjugate, multiply, normalize, rotate};
pub use vector_arithmetic::{add, divide, finite, scale, subtract, vector};

/// Three binary32 vector components.
pub type Vector = [f32; 3];
/// Quaternion components in x, y, z, w order.
pub type Quaternion = [f32; 4];
