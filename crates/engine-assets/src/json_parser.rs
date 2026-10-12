use serde::de::DeserializeOwned;

use super::mesh_asset::AssetError;

pub(super) const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_JSON_DEPTH: usize = 64;

pub(super) fn parse_asset_json<T>(bytes: &[u8]) -> Result<T, AssetError>
where
    T: DeserializeOwned,
{
    parse_asset_json_with_depth_limit(bytes, MAX_JSON_DEPTH)
}

pub(super) fn parse_asset_json_with_depth_limit<T>(
    bytes: &[u8],
    max_depth: usize,
) -> Result<T, AssetError>
where
    T: DeserializeOwned,
{
    validate_json_bounds(bytes, max_depth)?;

    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut deserializer).map_err(map_json_error)?;
    deserializer.end().map_err(map_json_error)?;

    Ok(value)
}

pub(super) fn validate_asset_parts_bounds(
    meshes_json: &[u8],
    materials_json: &[u8],
) -> Result<(), AssetError> {
    let fragment_bytes = meshes_json
        .len()
        .checked_add(materials_json.len())
        .ok_or(AssetError::Limit("JSON input"))?;
    if fragment_bytes > MAX_JSON_BYTES {
        return Err(AssetError::Limit("JSON input"));
    }

    let fragment_depth = MAX_JSON_DEPTH - 1;
    validate_json_bounds(meshes_json, fragment_depth)?;
    validate_json_bounds(materials_json, fragment_depth)?;
    Ok(())
}

fn map_json_error(error: serde_json::Error) -> AssetError {
    let detail = error.to_string();
    if detail.contains("duplicate field") {
        return AssetError::DuplicateKey;
    }

    for kind in ["render asset count", "mesh positions", "mesh indices"] {
        let marker = format!("asset-limit: {kind}");
        if detail.contains(&marker) {
            return AssetError::Limit(kind);
        }
    }

    AssetError::Parse(detail)
}

fn validate_json_bounds(bytes: &[u8], max_depth: usize) -> Result<(), AssetError> {
    if bytes.is_empty() || bytes.len() > MAX_JSON_BYTES {
        return Err(AssetError::Limit("JSON input"));
    }

    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for byte in bytes {
        if quoted {
            update_string_state(*byte, &mut quoted, &mut escaped);
            continue;
        }

        match *byte {
            b'"' => quoted = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > max_depth {
                    return Err(AssetError::Limit("JSON depth"));
                }
            }
            b'}' | b']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or(AssetError::Parse("unbalanced JSON".into()))?;
            }
            _ => {}
        }
    }

    if quoted || depth != 0 {
        return Err(AssetError::Parse("unterminated JSON".into()));
    }
    Ok(())
}

fn update_string_state(byte: u8, quoted: &mut bool, escaped: &mut bool) {
    if *escaped {
        *escaped = false;
    } else if byte == b'\\' {
        *escaped = true;
    } else if byte == b'"' {
        *quoted = false;
    }
}
