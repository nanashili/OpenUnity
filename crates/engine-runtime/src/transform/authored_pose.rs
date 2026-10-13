//! Authored Transform revision bookkeeping for the native motion attachment.
//!
//! The attachment owns these revisions beside its body and Transform IDs.
//! This module owns only the checked arithmetic and mutation classification;
//! the physics composition module supplies Runtime's attachment lookup and
//! native publication operations.

/// Pose edits that can affect an attached native body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransformEdit {
    Position,
    Rotation,
    Scale,
}
