use super::errors::{ComponentKind, PairingError};
use engine_scene::{MeshFilterReference, MeshRendererReference, SceneRenderReferences};
use std::collections::{BTreeMap, BTreeSet};

/// One render owner with exactly one MeshFilter and one MeshRenderer.
#[derive(Debug)]
pub(super) struct PairedRenderOwner<'scene> {
    pub(super) owner: i64,
    pub(super) mesh_filter: &'scene MeshFilterReference,
    pub(super) mesh_renderer: &'scene MeshRendererReference,
}

/// Pair extracted components by validated GameObject ID in draw order.
///
/// Component vectors retain parser order for deterministic diagnostics. Draw
/// order determines the returned projection order and is never used to pair
/// records by position.
pub(super) fn pair_render_components<'scene>(
    references: &'scene SceneRenderReferences,
    draw_owners: &[i64],
) -> Result<Vec<PairedRenderOwner<'scene>>, PairingError> {
    let requested = draw_owner_set(draw_owners)?;
    let mesh_filters = index_components(
        &references.mesh_filters,
        ComponentKind::MeshFilter,
        |reference| reference.game_object_file_id,
        |reference| reference.component_file_id,
    )?;
    let mesh_renderers = index_components(
        &references.mesh_renderers,
        ComponentKind::MeshRenderer,
        |reference| reference.game_object_file_id,
        |reference| reference.component_file_id,
    )?;

    let mut paired = Vec::with_capacity(draw_owners.len());
    for owner in draw_owners {
        let mesh_filter = mesh_filters
            .get(owner)
            .ok_or(PairingError::MissingComponent {
                owner: *owner,
                component: ComponentKind::MeshFilter,
            })?;
        let mesh_renderer = mesh_renderers
            .get(owner)
            .ok_or(PairingError::MissingComponent {
                owner: *owner,
                component: ComponentKind::MeshRenderer,
            })?;
        paired.push(PairedRenderOwner {
            owner: *owner,
            mesh_filter,
            mesh_renderer,
        });
    }

    reject_uncovered(
        &references.mesh_filters,
        ComponentKind::MeshFilter,
        &requested,
        |reference| reference.game_object_file_id,
        |reference| reference.component_file_id,
    )?;
    reject_uncovered(
        &references.mesh_renderers,
        ComponentKind::MeshRenderer,
        &requested,
        |reference| reference.game_object_file_id,
        |reference| reference.component_file_id,
    )?;

    Ok(paired)
}

fn draw_owner_set(draw_owners: &[i64]) -> Result<BTreeSet<i64>, PairingError> {
    let mut requested = BTreeSet::new();
    for owner in draw_owners {
        if *owner <= 0 {
            return Err(PairingError::InvalidDrawOwner { owner: *owner });
        }
        if !requested.insert(*owner) {
            return Err(PairingError::DuplicateDrawOwner { owner: *owner });
        }
    }
    Ok(requested)
}

fn index_components<T>(
    components: &[T],
    kind: ComponentKind,
    owner: impl Fn(&T) -> i64,
    component_file_id: impl Fn(&T) -> i64,
) -> Result<BTreeMap<i64, &T>, PairingError> {
    let mut indexed = BTreeMap::new();
    for component in components {
        let owner_id = owner(component);
        if indexed.insert(owner_id, component).is_some() {
            return Err(PairingError::DuplicateComponent {
                owner: owner_id,
                component: kind,
                component_file_id: component_file_id(component),
            });
        }
    }
    Ok(indexed)
}

fn reject_uncovered<T>(
    components: &[T],
    kind: ComponentKind,
    requested: &BTreeSet<i64>,
    owner: impl Fn(&T) -> i64,
    component_file_id: impl Fn(&T) -> i64,
) -> Result<(), PairingError> {
    for component in components {
        let owner_id = owner(component);
        if !requested.contains(&owner_id) {
            return Err(PairingError::UncoveredComponent {
                owner: owner_id,
                component: kind,
                component_file_id: component_file_id(component),
            });
        }
    }
    Ok(())
}
