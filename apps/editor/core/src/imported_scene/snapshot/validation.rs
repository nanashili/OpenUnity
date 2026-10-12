//! Declarative Transform and canonical script-binding validation.

use std::collections::BTreeMap;

use engine_scene::ScenePlan;

use super::super::{FieldValue, ImportedError, Result};

fn field_kind(value: &FieldValue) -> u8 {
    match value {
        FieldValue::I32(_) => 0,
        FieldValue::F32(_) => 1,
        FieldValue::Bool(_) => 2,
        FieldValue::String(_) => 3,
        FieldValue::Vec3(_) => 4,
        FieldValue::Quat(_) => 5,
        FieldValue::Reference(_) => 6,
    }
}
pub(super) fn validate_plan(plan: &ScenePlan) -> Result<()> {
    plan.validate()
        .map_err(|e| ImportedError::new("validate imported plan", e))?;
    validate_transforms(plan)?;
    validate_bindings(plan)
}

fn validate_transforms(plan: &ScenePlan) -> Result<()> {
    let transforms: BTreeMap<_, _> = plan.transforms.iter().map(|t| (t.owner, t)).collect();
    for go in &plan.game_objects {
        if transforms
            .get(&go.file_id)
            .is_none_or(|t| go.components.first() != Some(&t.file_id))
        {
            return Err(ImportedError::new(
                "validate imported plan",
                "mandatory Transform must be first",
            ));
        }
    }
    for t in &plan.transforms {
        if t.local_rotation.iter().all(|v| *v == 0.0) {
            return Err(ImportedError::new(
                "validate imported plan",
                "Transform quaternion must be nonzero",
            ));
        }
    }
    Ok(())
}

fn validate_bindings(plan: &ScenePlan) -> Result<()> {
    let mut guids = BTreeMap::new();
    let mut types = BTreeMap::new();
    for script in &plan.scripts {
        if script.assembly != "Assembly-CSharp" {
            return Err(ImportedError::new(
                "validate imported plan",
                "only Assembly-CSharp bindings are supported",
            ));
        }
        let fields: BTreeMap<_, _> = script
            .fields
            .iter()
            .map(|f| (f.name.as_str(), field_kind(&f.value)))
            .collect();
        let identity = (
            script.assembly.as_str(),
            script.type_name.as_str(),
            script.execution_order,
            fields,
        );
        if guids
            .get(script.guid.as_str())
            .is_some_and(|prior| prior != &identity)
        {
            return Err(ImportedError::new(
                "validate imported plan",
                "conflicting GUID binding declarations",
            ));
        }
        let type_key = (script.assembly.as_str(), script.type_name.as_str());
        if types
            .get(&type_key)
            .is_some_and(|guid| *guid != script.guid.as_str())
        {
            return Err(ImportedError::new(
                "validate imported plan",
                "conflicting type/GUID identities",
            ));
        }
        types.insert(type_key, script.guid.as_str());
        guids.insert(script.guid.as_str(), identity);
    }
    Ok(())
}
