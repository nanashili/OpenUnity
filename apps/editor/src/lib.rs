//! Native independent authoring tools. Unity compatibility is not verified.

// The disabled branch never expands include_bytes!, so public source builds
// without the private, ignored resource directory.
macro_rules! local_reference_bytes {
    ($path:literal) => {{
        #[cfg(feature = "local-reference-assets")]
        {
            include_bytes!($path) as &'static [u8]
        }
        #[cfg(not(feature = "local-reference-assets"))]
        {
            &[] as &'static [u8]
        }
    }};
}

use openunity_editor_core::scene::{EntityId, Primitive, Transform};

/// Validated model operations requested by a panel; the shell owns execution.
#[derive(Clone, Debug, PartialEq)]
pub enum EditorAction {
    /// Replace selection, or toggle one identity when requested.
    Select {
        /// Live document identity.
        id: EntityId,
        /// Toggle instead of replacing the selection.
        toggle: bool,
    },
    /// Create an independently authored primitive.
    Create {
        /// Independent authoring representation.
        primitive: Primitive,
        /// Optional parent in this document.
        parent: Option<EntityId>,
    },
    /// Remove the selected subtree(s).
    DeleteSelection,
    /// Create an identity-transform parent around the selected direct siblings.
    CreateParentForSelection,
    /// Apply one complete inspector transaction.
    Edit {
        /// Live document identity.
        id: EntityId,
        /// Proposed name.
        name: String,
        /// Proposed active state.
        active: bool,
        /// Proposed finite local transform.
        transform: Transform,
    },
    /// Change parent while preserving the local transform.
    Reparent {
        /// Live child identity.
        id: EntityId,
        /// Proposed parent, or a root.
        parent: Option<EntityId>,
    },
    /// Move one child within its current direct-child group.
    SetSiblingIndex {
        /// Live document identity.
        id: EntityId,
        /// Destination index in the entity's current sibling group.
        sibling_index: usize,
    },
}

pub mod app;
pub mod appearance;
pub mod component_expansion;
pub mod console_detail;
pub mod font_styles;
pub mod inspector_lock;
pub mod panels;
mod resources;

pub mod tab_titles;
pub mod workspace;
