use super::super::RenderManifestError;
use serde_json::{Map, Value};

pub(super) fn object(
    value: Value,
    field: &'static str,
) -> Result<Map<String, Value>, RenderManifestError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(RenderManifestError::InvalidField(field)),
    }
}

pub(super) fn array(value: Value, field: &'static str) -> Result<Vec<Value>, RenderManifestError> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(RenderManifestError::InvalidField(field)),
    }
}

pub(super) fn take(
    object: &mut Map<String, Value>,
    field: &'static str,
) -> Result<Value, RenderManifestError> {
    object
        .remove(field)
        .ok_or(RenderManifestError::InvalidField(field))
}

pub(super) fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    field: &'static str,
) -> Result<(), RenderManifestError> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(RenderManifestError::UnknownField(field));
    }
    Ok(())
}

pub(super) fn parse_u32(value: Value, field: &'static str) -> Result<u32, RenderManifestError> {
    value
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(RenderManifestError::InvalidField(field))
}

pub(super) fn parse_owner(value: Value, field: &'static str) -> Result<i64, RenderManifestError> {
    let owner = value
        .as_i64()
        .ok_or(RenderManifestError::InvalidField(field))?;
    if owner <= 0 {
        return Err(RenderManifestError::InvalidField(field));
    }
    Ok(owner)
}

pub(super) fn parse_f32(value: Value, field: &'static str) -> Result<f32, RenderManifestError> {
    let number = value
        .as_f64()
        .ok_or(RenderManifestError::InvalidField(field))?;
    let value = number as f32;
    value
        .is_finite()
        .then_some(value)
        .ok_or(RenderManifestError::InvalidField(field))
}

pub(super) fn parse_rgba(
    value: Value,
    field: &'static str,
) -> Result<[u8; 4], RenderManifestError> {
    let values = array(value, field)?;
    if values.len() != 4 {
        return Err(RenderManifestError::InvalidField(field));
    }
    Ok([
        parse_byte(values[0].clone(), field)?,
        parse_byte(values[1].clone(), field)?,
        parse_byte(values[2].clone(), field)?,
        parse_byte(values[3].clone(), field)?,
    ])
}

pub(super) fn parse_byte(value: Value, field: &'static str) -> Result<u8, RenderManifestError> {
    value
        .as_u64()
        .and_then(|value| u8::try_from(value).ok())
        .ok_or(RenderManifestError::InvalidField(field))
}
