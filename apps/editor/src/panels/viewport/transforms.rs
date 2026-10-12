//! Finite local affine composition and cached hierarchy traversal.

use std::collections::BTreeMap;

use openunity_editor_core::scene::{EntityId, MAX_HIERARCHY_DEPTH, Scene, Transform};

#[derive(Clone, Copy)]
pub(super) struct Affine([[f64; 4]; 3]);

impl Affine {
    fn local(transform: Transform) -> Self {
        let [x, y, z] = transform
            .rotation_deg
            .map(|value| f64::from(value).to_radians());
        let (sx, cx) = x.sin_cos();
        let (sy, cy) = y.sin_cos();
        let (sz, cz) = z.sin_cos();
        let [a, b, c] = transform.scale.map(f64::from);
        Self([
            [
                cz * cy * a,
                (cz * sy * sx - sz * cx) * b,
                (cz * sy * cx + sz * sx) * c,
                f64::from(transform.translation[0]),
            ],
            [
                sz * cy * a,
                (sz * sy * sx + cz * cx) * b,
                (sz * sy * cx - cz * sx) * c,
                f64::from(transform.translation[1]),
            ],
            [
                -sy * a,
                cy * sx * b,
                cy * cx * c,
                f64::from(transform.translation[2]),
            ],
        ])
    }

    pub(super) fn apply(self, point: [f64; 3]) -> [f64; 3] {
        self.0
            .map(|row| row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3])
    }

    fn compose(self, child: Self) -> Self {
        let mut result = [[0.0; 4]; 3];
        for (row, target) in result.iter_mut().enumerate() {
            for (column, value) in target.iter_mut().enumerate() {
                *value = (0..3)
                    .map(|k| self.0[row][k] * child.0[k][column])
                    .sum::<f64>();
                if column == 3 {
                    *value += self.0[row][3];
                }
            }
        }
        Self(result)
    }
}

pub(super) fn world_transform(
    scene: &Scene,
    id: EntityId,
    cache: &mut BTreeMap<EntityId, Affine>,
) -> Option<Affine> {
    let mut chain = Vec::new();
    let mut current = id;
    let parent_world = loop {
        if let Some(world) = cache.get(&current) {
            break Some(*world);
        }
        if chain.len() >= MAX_HIERARCHY_DEPTH {
            return None;
        }
        let entity = scene.get(current)?;
        chain.push((current, Affine::local(entity.transform)));
        match entity.parent {
            Some(parent) => current = parent,
            None => break None,
        }
    };
    let mut world = parent_world;
    for (id, local) in chain.into_iter().rev() {
        let next = world.map_or(local, |parent| parent.compose(local));
        if !next.0.iter().flatten().all(|value| value.is_finite()) {
            return None;
        }
        cache.insert(id, next);
        world = Some(next);
    }
    world
}
