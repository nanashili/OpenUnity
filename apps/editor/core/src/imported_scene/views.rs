//! Borrowed canonical inspection records and supported entity edits.

use engine_scene::{GameObjectPlan, ScriptPlan, TransformPlan};

use crate::scene::EntityId;

/// Supported whole-entity edit; raw rotation components are never converted to Euler angles.
#[derive(Clone, Debug)]
pub struct ImportedEntityEdit {
    /// GameObject name, retaining the imported string bounds.
    pub name: String,
    /// Local GameObject activity state.
    pub active: bool,
    /// Finite local position components.
    pub local_position: [f32; 3],
    /// Finite nonzero raw quaternion components; no normalization.
    pub local_rotation: [f32; 4],
    /// Finite local scale components, including zero or negative values.
    pub local_scale: [f32; 3],
}
/// Borrowed canonical entity records.
#[derive(Clone, Copy, Debug)]
pub struct ImportedEntityView<'a> {
    /// Document-local owner handle.
    pub editor_id: EntityId,
    /// Original GameObject record.
    pub game_object: &'a GameObjectPlan,
    /// Original mandatory Transform record.
    pub transform: &'a TransformPlan,
}
/// Component inspection follows the original GameObject component order.
#[derive(Clone, Copy, Debug)]
pub enum ImportedComponent<'a> {
    /// Original mandatory Transform target.
    Transform(&'a TransformPlan),
    /// Original declaratively bound script target.
    Script(&'a ScriptPlan),
}
/// Kind of a canonical file-local reference target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportedReferenceKind {
    /// Original GameObject target.
    GameObject,
    /// Original mandatory Transform target.
    Transform,
    /// Original declaratively bound script target.
    Script,
}
/// Exact target record, with no native identity.
#[derive(Clone, Copy, Debug)]
pub enum ImportedReferenceTarget<'a> {
    /// Original GameObject target.
    GameObject(&'a GameObjectPlan),
    /// Original mandatory Transform target.
    Transform(&'a TransformPlan),
    /// Original declaratively bound script target.
    Script(&'a ScriptPlan),
}
/// Resolved borrowed reference and its canonical owner handle.
#[derive(Clone, Copy, Debug)]
pub struct ImportedReference<'a> {
    /// Exact local file identity.
    pub file_id: i64,
    /// Canonical owning GameObject editor handle.
    pub owner: EntityId,
    /// Typed target kind.
    pub kind: ImportedReferenceKind,
    /// GameObject name, retaining the imported string bounds.
    pub name: &'a str,
    /// Borrowed canonical target record.
    pub target: ImportedReferenceTarget<'a>,
}
