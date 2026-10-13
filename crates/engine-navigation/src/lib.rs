//! Validated navigation graphs, paths, and bounded path following.

mod graph;
mod path_following;
mod shortest_path;

pub use graph::{GraphError, NavigationEdge, NavigationGraph, NavigationNode, NodeIndex};
pub use path_following::{FollowError, FollowResult, PathFollower, PreparedFollow, follow_path};
pub use shortest_path::{Path, PathError, shortest_path};
