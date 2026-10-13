//! Parses the narrow PPtr subset used by scene render-reference extraction.

use super::{MaterialField, PointerField, SerializedAssetPointer};
use crate::{Result, SceneError, yaml::Node};
use std::collections::BTreeMap;

pub(super) fn parse_asset_pointer(node: &Node, context: &str) -> Result<SerializedAssetPointer> {
    let fields = match node {
        Node::Map(fields) => fields,
        _ => {
            return Err(crate::SceneError::new(format!(
                "{context} must be a PPtr mapping"
            )));
        }
    };

    reject_unsupported_pointer_fields(fields, context)?;

    let file_id = required_decimal_field(fields, "fileID", context)?;
    let guid = optional_string_field(fields, "guid", context)?;
    let type_hint = optional_decimal_field(fields, "type", context)?;

    Ok(SerializedAssetPointer {
        file_id,
        guid,
        type_hint,
    })
}

pub(super) fn parse_mesh_field(
    fields: &BTreeMap<String, Node>,
    context: &str,
) -> Result<PointerField> {
    let Some(node) = fields.get("m_Mesh") else {
        return Ok(PointerField::Missing);
    };

    match node {
        Node::Scalar(value) if is_explicit_null(value) => Ok(PointerField::ExplicitNull),
        Node::Map(_) => parse_asset_pointer(node, context).map(PointerField::Pointer),
        _ => Err(expected_pointer_or_null(context)),
    }
}

pub(super) fn parse_materials_field(
    fields: &BTreeMap<String, Node>,
    context: &str,
) -> Result<MaterialField> {
    let Some(node) = fields.get("m_Materials") else {
        return Ok(MaterialField::Missing);
    };

    match node {
        Node::Scalar(value) if is_explicit_null(value) => Ok(MaterialField::ExplicitNull),
        Node::Seq(items) => parse_material_slots(items, context).map(MaterialField::Slots),
        _ => Err(crate::SceneError::new(format!(
            "{context} must be an unquoted null, ~, or a material-slot sequence"
        ))),
    }
}

fn parse_material_slots(items: &[Node], context: &str) -> Result<Vec<SerializedAssetPointer>> {
    let mut slots = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let slot_context = format!("{context}[{index}]");
        let pointer = match item {
            Node::Map(_) => parse_asset_pointer(item, &slot_context)?,
            _ => {
                return Err(crate::SceneError::new(format!(
                    "{slot_context} must be a PPtr mapping"
                )));
            }
        };

        slots.push(pointer);
    }

    Ok(slots)
}

fn reject_unsupported_pointer_fields(fields: &BTreeMap<String, Node>, context: &str) -> Result<()> {
    for field_name in fields.keys() {
        if matches!(field_name.as_str(), "fileID" | "guid" | "type") {
            continue;
        }

        return Err(crate::SceneError::new(format!(
            "{context} has unsupported PPtr field {field_name}"
        )));
    }

    Ok(())
}

fn required_decimal_field(
    fields: &BTreeMap<String, Node>,
    name: &str,
    context: &str,
) -> Result<i64> {
    let field_context = format!("{context}.{name}");
    let node = fields
        .get(name)
        .ok_or_else(|| crate::SceneError::new(format!("{field_context} is missing")))?;

    parse_decimal_scalar(node, &field_context)
}

fn optional_decimal_field(
    fields: &BTreeMap<String, Node>,
    name: &str,
    context: &str,
) -> Result<Option<i64>> {
    let Some(node) = fields.get(name) else {
        return Ok(None);
    };

    let field_context = format!("{context}.{name}");
    parse_decimal_scalar(node, &field_context).map(Some)
}

fn parse_decimal_scalar(node: &Node, context: &str) -> Result<i64> {
    let value = match node {
        Node::Scalar(value) => value,
        Node::Quoted(_) => {
            return Err(crate::SceneError::new(format!(
                "{context} must be an unquoted decimal integer"
            )));
        }
        _ => {
            return Err(crate::SceneError::new(format!(
                "{context} must be an unquoted decimal scalar"
            )));
        }
    };

    if !has_decimal_integer_syntax(value) {
        return Err(crate::SceneError::new(format!(
            "{context} must match decimal integer syntax"
        )));
    }

    value
        .parse::<i64>()
        .map_err(|_| crate::SceneError::new(format!("{context} is outside the i64 range")))
}

fn optional_string_field(
    fields: &BTreeMap<String, Node>,
    name: &str,
    context: &str,
) -> Result<Option<String>> {
    let Some(node) = fields.get(name) else {
        return Ok(None);
    };

    let value = match node {
        Node::Scalar(value) | Node::Quoted(value) => value.clone(),
        _ => {
            return Err(crate::SceneError::new(format!(
                "{context}.{name} must be a scalar string"
            )));
        }
    };

    Ok(Some(value))
}

fn expected_pointer_or_null(context: &str) -> SceneError {
    SceneError::new(format!(
        "{context} must be an unquoted null, ~, or a PPtr mapping"
    ))
}

fn is_explicit_null(value: &str) -> bool {
    matches!(value, "null" | "~")
}

fn has_decimal_integer_syntax(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}
