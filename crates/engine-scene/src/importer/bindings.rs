use super::filesystem::read;
use crate::{MAX_FIELDS, MAX_OBJECTS, Result, SceneError, parse_json, text};
use engine_assets::{Guid, MetadataIndex};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    schema_version: u32,
    scripts: Vec<Binding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub(super) guid: String,
    pub(super) assembly: String,
    #[serde(rename = "type")]
    pub(super) type_name: String,
    pub(super) execution_order: i32,
    pub(super) fields: Vec<FieldBinding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FieldBinding {
    pub(super) name: String,
    pub(super) kind: String,
}

pub(super) fn load(
    binding_path: &Path,
    metadata: &MetadataIndex,
) -> Result<BTreeMap<Guid, Binding>> {
    let bindings: Bindings =
        serde_json::from_value(parse_json(&read(binding_path)?)?).map_err(SceneError::new)?;
    if bindings.schema_version != 1
        || bindings.scripts.is_empty()
        || bindings.scripts.len() > MAX_OBJECTS
    {
        return Err(SceneError::new("unsupported binding manifest"));
    }
    let mut scripts = BTreeMap::new();
    let mut types = BTreeSet::new();
    for binding in bindings.scripts {
        text(&binding.assembly)?;
        text(&binding.type_name)?;
        let guid = binding.guid.parse::<Guid>().map_err(SceneError::new)?;
        let script_asset = metadata
            .asset_path(guid)
            .ok_or_else(|| SceneError::new("unresolved script GUID"))?;
        if !script_asset.is_file()
            || script_asset.extension().is_none_or(|e| e != "cs")
            || binding.guid != guid.to_string()
            || binding.assembly != "Assembly-CSharp"
            || binding.type_name.is_empty()
            || binding.fields.len() > MAX_FIELDS
            || !types.insert((binding.assembly.clone(), binding.type_name.clone()))
        {
            return Err(SceneError::new("unresolved or ambiguous script binding"));
        }
        let mut fields = BTreeSet::new();
        for field in &binding.fields {
            text(&field.name)?;
            if field.name.is_empty()
                || !fields.insert(&field.name)
                || !["i32", "f32", "bool", "string", "vec3", "quat", "reference"]
                    .contains(&field.kind.as_str())
            {
                return Err(SceneError::new("invalid binding field"));
            }
        }
        if scripts.insert(guid, binding).is_some() {
            return Err(SceneError::new("duplicate binding GUID"));
        }
    }
    Ok(scripts)
}
