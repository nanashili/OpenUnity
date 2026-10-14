//! Selection predicates shared by hierarchy presentations.

use openunity_editor_core::scene::Scene;
use openunity_editor_core::selection::Selection;

pub(super) fn can_group_selection(scene: &Scene, selection: &Selection) -> bool {
    let mut ids = selection.ids();
    let Some(first) = ids.next().and_then(|id| scene.get(id)) else {
        return false;
    };

    ids.all(|id| {
        scene
            .get(id)
            .is_some_and(|entity| entity.parent == first.parent)
    })
}
