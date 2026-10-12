use super::{
    bindings::Binding,
    document_values::{common, constant, flag, get, keys, reference, value},
};
use crate::{FieldPlan, Result, SceneError, ScriptPlan, yaml::Node};
use engine_assets::Guid;
use std::collections::BTreeMap;

pub(super) fn convert(
    file_id: i64,
    m: &BTreeMap<String, Node>,
    scripts: &BTreeMap<Guid, Binding>,
) -> Result<ScriptPlan> {
    common(m)?;
    constant(m, "m_EditorHideFlags", 0)?;
    if !get(m, "m_Name")?.scalar()?.is_empty() {
        return Err(SceneError::new("unsupported component name"));
    }
    let script = get(m, "m_Script")?.map()?;
    keys(script, &["fileID", "guid", "type"])?;
    constant(script, "fileID", 11500000)?;
    constant(script, "type", 3)?;
    let raw_guid = get(script, "guid")?.scalar()?;
    let guid = raw_guid.parse::<Guid>().map_err(SceneError::new)?;
    if raw_guid != guid.to_string() {
        return Err(SceneError::new("noncanonical script GUID"));
    }
    let b = scripts
        .get(&guid)
        .ok_or_else(|| SceneError::new("missing script binding"))?;
    let mut expected = vec![
        "m_ObjectHideFlags",
        "m_CorrespondingSourceObject",
        "m_PrefabInstance",
        "m_PrefabAsset",
        "m_GameObject",
        "m_Enabled",
        "m_EditorHideFlags",
        "m_Script",
        "m_Name",
        "m_EditorClassIdentifier",
    ];
    expected.extend(b.fields.iter().map(|f| f.name.as_str()));
    keys(m, &expected)?;
    if get(m, "m_EditorClassIdentifier")?.scalar()? != format!("{}::{}", b.assembly, b.type_name) {
        return Err(SceneError::new("script identity disagreement"));
    }
    let fields = b
        .fields
        .iter()
        .map(|f| {
            Ok(FieldPlan {
                name: f.name.clone(),
                value: value(get(m, &f.name)?, &f.kind)?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(ScriptPlan {
        file_id,
        owner: reference(get(m, "m_GameObject")?)?,
        guid: guid.to_string(),
        assembly: b.assembly.clone(),
        type_name: b.type_name.clone(),
        enabled: flag(get(m, "m_Enabled")?)?,
        execution_order: b.execution_order,
        fields,
    })
}
