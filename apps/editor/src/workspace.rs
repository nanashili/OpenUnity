//! Independent fixed-tab workspace for INFRA-EDITOR-WORKSPACE-001.

mod identity;
mod snapshot;
mod state;

pub use identity::{DockId, SplitId, TabId};
pub use snapshot::{
    ControlSnapshot, DockSnapshot, SplitterSnapshot, TabTitleSnapshot, WorkspaceSnapshot,
};
pub use state::Workspace;
