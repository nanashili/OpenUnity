use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

use super::mesh_asset::{MAX_MESH_INDICES, MAX_MESH_POSITIONS};
use super::render_assets::{MAX_INDICES, MAX_MESHES, MAX_POSITIONS};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawManifest {
    pub schema_version: u32,
    pub meshes: RawMeshes,
    pub materials: BoundedVec<RawMaterial, MAX_MESHES>,
}

pub(super) struct RawMesh {
    pub id: u32,
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawMaterial {
    pub id: u32,
    pub rgba8: [u8; 4],
}

pub(super) struct RawMeshes(Vec<RawMesh>);

impl RawMeshes {
    pub fn into_vec(self) -> Vec<RawMesh> {
        self.0
    }
}

impl<'de> Deserialize<'de> for RawMeshes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MeshesVisitor;

        impl<'de> Visitor<'de> for MeshesVisitor {
            type Value = RawMeshes;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded mesh array")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let capacity = access.size_hint().unwrap_or(0).min(MAX_MESHES);
                let mut meshes = Vec::with_capacity(capacity);
                let mut remaining_positions = MAX_POSITIONS;
                let mut remaining_indices = MAX_INDICES;

                while meshes.len() < MAX_MESHES {
                    let Some(mesh) = access.next_element_seed(RawMeshSeed {
                        remaining_positions,
                        remaining_indices,
                    })?
                    else {
                        return Ok(RawMeshes(meshes));
                    };

                    remaining_positions -= mesh.positions.len();
                    remaining_indices -= mesh.indices.len();
                    meshes.push(mesh);
                }

                if access.next_element::<IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom("asset-limit: render asset count"));
                }

                Ok(RawMeshes(meshes))
            }
        }

        deserializer.deserialize_seq(MeshesVisitor)
    }
}

pub(super) struct BoundedVec<T, const MAX: usize>(Vec<T>);

impl<T, const MAX: usize> BoundedVec<T, MAX> {
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<'de, T, const MAX: usize> Deserialize<'de> for BoundedVec<T, MAX>
where
    T: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        BoundedVecSeed::new(MAX, "render asset count")
            .deserialize(deserializer)
            .map(BoundedVec)
    }
}

struct RawMeshSeed {
    remaining_positions: usize,
    remaining_indices: usize,
}

impl<'de> DeserializeSeed<'de> for RawMeshSeed {
    type Value = RawMesh;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(RawMeshVisitor {
            remaining_positions: self.remaining_positions,
            remaining_indices: self.remaining_indices,
        })
    }
}

struct RawMeshVisitor {
    remaining_positions: usize,
    remaining_indices: usize,
}

impl<'de> Visitor<'de> for RawMeshVisitor {
    type Value = RawMesh;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a mesh object with id, positions, and indices")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<RawMesh, A::Error> {
        let mut id = None;
        let mut positions = None;
        let mut indices = None;

        while let Some(key) = access.next_key::<String>()? {
            match key.as_str() {
                "id" => {
                    reject_duplicate(&id)?;
                    id = Some(access.next_value()?);
                }
                "positions" => {
                    reject_duplicate(&positions)?;
                    let limit = self.remaining_positions.min(MAX_MESH_POSITIONS);
                    positions =
                        Some(access.next_value_seed(BoundedVecSeed::new(limit, "mesh positions"))?);
                }
                "indices" => {
                    reject_duplicate(&indices)?;
                    let limit = self.remaining_indices.min(MAX_MESH_INDICES);
                    indices =
                        Some(access.next_value_seed(BoundedVecSeed::new(limit, "mesh indices"))?);
                }
                _ => {
                    return Err(de::Error::unknown_field(
                        &key,
                        &["id", "positions", "indices"],
                    ));
                }
            }
        }

        let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
        let positions = positions.ok_or_else(|| de::Error::missing_field("positions"))?;
        let indices = indices.ok_or_else(|| de::Error::missing_field("indices"))?;
        validate_mesh_buffer_lengths::<A::Error>(positions.len(), indices.len())?;

        Ok(RawMesh {
            id,
            positions,
            indices,
        })
    }
}

fn validate_mesh_buffer_lengths<E: de::Error>(positions: usize, indices: usize) -> Result<(), E> {
    if !(3..=MAX_MESH_POSITIONS).contains(&positions) {
        return Err(E::custom("asset-limit: mesh positions"));
    }
    if !(3..=MAX_MESH_INDICES).contains(&indices) || indices % 3 != 0 {
        return Err(E::custom("asset-limit: mesh indices"));
    }

    Ok(())
}

fn reject_duplicate<T, E: de::Error>(slot: &Option<T>) -> Result<(), E> {
    if slot.is_some() {
        return Err(E::custom("duplicate field"));
    }

    Ok(())
}

struct BoundedVecSeed<T> {
    limit: usize,
    category: &'static str,
    marker: PhantomData<T>,
}

impl<T> BoundedVecSeed<T> {
    fn new(limit: usize, category: &'static str) -> Self {
        Self {
            limit,
            category,
            marker: PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for BoundedVecSeed<T> {
    type Value = Vec<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(BoundedVecVisitor {
            limit: self.limit,
            category: self.category,
            marker: PhantomData,
        })
    }
}

struct BoundedVecVisitor<T> {
    limit: usize,
    category: &'static str,
    marker: PhantomData<T>,
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for BoundedVecVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "an array with at most {} entries", self.limit)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Vec<T>, A::Error> {
        let capacity = access.size_hint().unwrap_or(0).min(self.limit);
        let mut values = Vec::with_capacity(capacity);

        while let Some(value) = access.next_element()? {
            if values.len() == self.limit {
                return Err(de::Error::custom(format!("asset-limit: {}", self.category)));
            }
            values.push(value);
        }

        Ok(values)
    }
}
