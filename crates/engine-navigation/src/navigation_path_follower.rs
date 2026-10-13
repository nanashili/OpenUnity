//! Owned, resumable progress along one validated graph path.

use engine_math::Vector;

use super::super::{NavigationGraph, Path};
use super::traversal::{FollowProgress, advance_from_progress};
use super::validate_path_nodes;
use super::{FollowError, FollowResult, node_position, validate_distance, validate_path_edges};

/// Owns resumable progress along one path in an immutably borrowed graph.
///
/// Progress is measured from original graph segment endpoints. A rounded
/// emitted position is never used as a later interpolation origin.
pub struct PathFollower<'graph> {
    graph: &'graph NavigationGraph,
    path: Path,
    progress: FollowProgress,
}

impl<'graph> PathFollower<'graph> {
    /// Validate and take ownership of a path for resumable traversal.
    pub fn new(graph: &'graph NavigationGraph, path: Path) -> Result<Self, FollowError> {
        if path.nodes.is_empty() {
            return Err(FollowError::EmptyPath);
        }

        if !path.total_cost.is_finite() || path.total_cost < 0.0 {
            return Err(FollowError::InvalidPathCost);
        }

        validate_path_nodes(graph, &path.nodes)?;
        validate_path_edges(graph, &path.nodes)?;

        let initial_position = node_position(graph, path.nodes[0])?;
        let progress = FollowProgress::at_position(0, 0.0, initial_position);

        Ok(Self {
            graph,
            path,
            progress,
        })
    }

    /// Return the validated path owned by this follower.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return whether traversal reached the final path node.
    pub fn is_complete(&self) -> bool {
        self.progress.segment_ordinal == self.path.nodes.len() - 1
    }

    /// Return the last position emitted by a committed step.
    pub fn expected_position(&self) -> Vector {
        self.progress.position
    }

    /// Prepare a bounded step without changing owned progress.
    ///
    /// The returned guard exclusively borrows this follower. Committing the
    /// guard publishes its candidate; dropping it leaves progress unchanged.
    pub fn prepare_advance(
        &mut self,
        max_distance: f32,
    ) -> Result<PreparedFollow<'_, 'graph>, FollowError> {
        validate_distance(max_distance)?;

        let (candidate, result) = advance_from_progress(
            self.graph,
            &self.path.nodes,
            self.progress,
            f64::from(max_distance),
        )?;

        Ok(PreparedFollow {
            follower: self,
            candidate,
            result,
        })
    }
}

/// A prepared follower step that commits only when explicitly consumed.
#[must_use = "dropping a prepared follow aborts its candidate progress"]
pub struct PreparedFollow<'follower, 'graph> {
    follower: &'follower mut PathFollower<'graph>,
    candidate: FollowProgress,
    result: FollowResult,
}

impl PreparedFollow<'_, '_> {
    /// Return the result that would be published by committing this step.
    pub fn result(&self) -> FollowResult {
        self.result
    }

    /// Publish the progress calculated during preparation.
    pub fn commit(self) {
        self.follower.progress = self.candidate;
    }
}
