use super::super::RenderManifestError;
use super::manifest_wire::validate_document_bounds;
use super::unique_json_value::{ARRAY_LIMIT, MEMBER_LIMIT, NODE_LIMIT, ParseBudget, UniqueSeed};
use serde::de::DeserializeSeed;
use serde_json::Value;

pub(super) fn parse_unique_json(bytes: &[u8]) -> Result<Value, RenderManifestError> {
    parse_unique_json_with_seed(bytes, UniqueSeed::new(ParseBudget::new()), None)
}

pub(super) fn parse_unique_array(
    bytes: &[u8],
    maximum_items: usize,
    limit_name: &'static str,
) -> Result<Vec<Value>, RenderManifestError> {
    let value = parse_unique_json_with_seed(
        bytes,
        UniqueSeed::bounded_root_array(ParseBudget::new(), maximum_items),
        Some(limit_name),
    )?;

    match value {
        Value::Array(values) => Ok(values),
        _ => Err(RenderManifestError::InvalidField("JSON array")),
    }
}

fn parse_unique_json_with_seed(
    bytes: &[u8],
    seed: UniqueSeed,
    array_limit_name: Option<&'static str>,
) -> Result<Value, RenderManifestError> {
    validate_document_bounds(bytes)?;

    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let parsed = seed.deserialize(&mut deserializer);
    let parsed = parsed.and_then(|unique| {
        deserializer.end()?;
        Ok(unique.0)
    });
    parsed.map_err(|error| {
        if error.to_string().contains("duplicate JSON key") {
            RenderManifestError::DuplicateKey
        } else if error.to_string().contains(ARRAY_LIMIT) {
            RenderManifestError::Limit(array_limit_name.unwrap_or("JSON array items"))
        } else if error.to_string().contains(NODE_LIMIT) {
            RenderManifestError::Limit("JSON nodes")
        } else if error.to_string().contains(MEMBER_LIMIT) {
            RenderManifestError::Limit("JSON object members")
        } else {
            RenderManifestError::Parse(error.to_string())
        }
    })
}
