use super::super::{MAX_BYTES, MAX_DEPTH, RenderManifestError};
use serde::Deserialize;
use serde::de::{self, MapAccess, Visitor};
use serde_json::value::RawValue;
use std::fmt;

pub(in crate::render_manifest) struct ManifestWire<'a> {
    pub(in crate::render_manifest) schema_version: u32,
    pub(in crate::render_manifest) meshes: &'a RawValue,
    pub(in crate::render_manifest) materials: &'a RawValue,
    pub(in crate::render_manifest) camera: &'a RawValue,
    pub(in crate::render_manifest) draws: &'a RawValue,
}

pub(in crate::render_manifest) fn parse_manifest(
    bytes: &[u8],
) -> Result<ManifestWire<'_>, RenderManifestError> {
    validate_document_bounds(bytes)?;
    serde_json::from_slice(bytes).map_err(|error| {
        if error.to_string().contains("duplicate JSON key") {
            RenderManifestError::DuplicateKey
        } else {
            RenderManifestError::Parse(error.to_string())
        }
    })
}

pub(super) fn validate_document_bounds(bytes: &[u8]) -> Result<(), RenderManifestError> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(RenderManifestError::Limit("manifest bytes"));
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        if *byte == b'"' {
            in_string = true;
            continue;
        }
        match byte {
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(RenderManifestError::Limit("JSON depth"));
                }
            }
            b'}' | b']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| RenderManifestError::Parse("unbalanced JSON".into()))?;
            }
            _ => {}
        }
    }

    if depth != 0 || in_string {
        return Err(RenderManifestError::Parse("unbalanced JSON".into()));
    }
    Ok(())
}

impl<'de> Deserialize<'de> for ManifestWire<'de> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ManifestVisitor)
    }
}

struct ManifestVisitor;

impl<'de> Visitor<'de> for ManifestVisitor {
    type Value = ManifestWire<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a render manifest object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut schema_version = None;
        let mut meshes = None;
        let mut materials = None;
        let mut camera = None;
        let mut draws = None;

        while let Some(key) = access.next_key::<String>()? {
            match key.as_str() {
                "schema_version" => assign(&mut schema_version, access.next_value()?)?,
                "meshes" => assign(&mut meshes, access.next_value()?)?,
                "materials" => assign(&mut materials, access.next_value()?)?,
                "camera" => assign(&mut camera, access.next_value()?)?,
                "draws" => assign(&mut draws, access.next_value()?)?,
                _ => return Err(de::Error::unknown_field(&key, FIELDS)),
            }
        }

        Ok(ManifestWire {
            schema_version: required(schema_version, "schema_version")?,
            meshes: required(meshes, "meshes")?,
            materials: required(materials, "materials")?,
            camera: required(camera, "camera")?,
            draws: required(draws, "draws")?,
        })
    }
}

const FIELDS: &[&str] = &["schema_version", "meshes", "materials", "camera", "draws"];

fn assign<T, E: de::Error>(slot: &mut Option<T>, value: T) -> Result<(), E> {
    if slot.replace(value).is_some() {
        return Err(E::custom("duplicate JSON key"));
    }
    Ok(())
}

fn required<T, E: de::Error>(value: Option<T>, field: &'static str) -> Result<T, E> {
    value.ok_or_else(|| E::missing_field(field))
}
