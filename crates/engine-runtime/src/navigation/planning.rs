use engine_identifiers::ObjectId;
use engine_navigation::{NavigationGraph, PathError, PathFollower, shortest_path};

use crate::{Runtime, RuntimeError, TransformData};

use super::{NavigationRoute, RuntimeNavigationError};

const MAX_ROUTE_GRAPH_NODES: usize = 512;
const MAX_ROUTE_GRAPH_EDGES: usize = 4_096;

impl Runtime {
    /// Plan the deterministic shortest route for a live, unparented GameObject.
    ///
    /// The graph must fit the runtime's bounded route limits, and the agent's
    /// root world position must equal the selected start node exactly.
    ///
    /// # Errors
    /// Returns an object, kind, hierarchy, physics, graph, path, or position error.
    pub fn plan_navigation_route<'graph>(
        &self,
        agent: ObjectId,
        graph: &'graph NavigationGraph,
        start: usize,
        goal: usize,
    ) -> Result<NavigationRoute<'graph>, RuntimeNavigationError> {
        let (_, transform) = navigation_agent_transform(self, agent)?;

        validate_graph_limits(graph)?;

        let start_position = graph
            .position(start)
            .map_err(|_| RuntimeNavigationError::Path(PathError::InvalidNode))?;
        if transform.position != start_position {
            return Err(RuntimeNavigationError::StartPositionMismatch);
        }

        let path = shortest_path(graph, start, goal)?;
        let follower = PathFollower::new(graph, path)?;

        Ok(NavigationRoute { agent, follower })
    }
}

pub(super) fn navigation_agent_transform(
    runtime: &Runtime,
    agent: ObjectId,
) -> Result<(ObjectId, &TransformData), RuntimeNavigationError> {
    let transform_id = runtime
        .objects
        .gameobject_transform(agent)
        .map_err(RuntimeError::from)?;
    let transform = runtime.transform(transform_id)?;

    if transform.owner != agent {
        return Err(RuntimeError::InvalidObject.into());
    }
    if transform.parent.is_some() {
        return Err(RuntimeNavigationError::ParentTransformUnsupported);
    }
    if runtime.physics_has_attachment_for_owner(agent) {
        return Err(RuntimeNavigationError::PhysicsAttachmentUnsupported);
    }

    Ok((transform_id, transform))
}

fn validate_graph_limits(graph: &NavigationGraph) -> Result<(), RuntimeNavigationError> {
    if graph.node_count() > MAX_ROUTE_GRAPH_NODES {
        return Err(RuntimeNavigationError::ResourceLimit);
    }

    let mut edge_count = 0usize;
    for node in 0..graph.node_count() {
        let neighbors = graph
            .neighbors(node)
            .map_err(|_| RuntimeNavigationError::Path(PathError::InvalidNode))?;
        edge_count = edge_count
            .checked_add(neighbors.len())
            .ok_or(RuntimeNavigationError::ResourceLimit)?;

        if edge_count > MAX_ROUTE_GRAPH_EDGES {
            return Err(RuntimeNavigationError::ResourceLimit);
        }
    }

    Ok(())
}
