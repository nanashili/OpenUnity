use super::Node;
use crate::{MAX_DEPTH, Result, SceneError, text};
use std::collections::BTreeMap;

pub(super) fn pair(input: &str) -> Result<(&str, &str)> {
    let (key, value) = input
        .split_once(':')
        .ok_or_else(|| SceneError::new("expected key/value separator"))?;
    if key.is_empty()
        || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || (!value.is_empty() && !value.starts_with(' '))
    {
        return Err(SceneError::new("unsupported mapping key or separator"));
    }
    Ok((key, value.trim_start_matches(' ')))
}
pub(super) fn inline(input: &str, depth: usize) -> Result<Node> {
    if depth > MAX_DEPTH {
        return Err(SceneError::new("text nesting limit"));
    }
    if input == "[]" {
        return Ok(Node::Seq(Vec::new()));
    }
    if input.starts_with('{') {
        let content = input
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| SceneError::new("unterminated flow mapping"))?;
        let mut map = BTreeMap::new();
        let mut start = 0;
        let mut quoted = false;
        let mut escaped = false;
        let mut nested = 0usize;
        for (i, b) in content.bytes().chain(Some(b',')).enumerate() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    quoted = false;
                }
                continue;
            }
            match b {
                b'"' => quoted = true,
                b'{' => nested += 1,
                b'}' => {
                    nested = nested
                        .checked_sub(1)
                        .ok_or_else(|| SceneError::new("unbalanced mapping"))?
                }
                b',' if nested == 0 => {
                    let item = content
                        .get(start..i)
                        .ok_or_else(|| SceneError::new("flow boundary"))?
                        .trim();
                    let (key, value) = pair(item)?;
                    if map
                        .insert(key.to_owned(), inline(value, depth + 1)?)
                        .is_some()
                    {
                        return Err(SceneError::new("duplicate flow key"));
                    }
                    start = i + 1;
                }
                _ => (),
            }
        }
        if quoted || nested != 0 {
            return Err(SceneError::new("unbalanced flow mapping"));
        }
        return Ok(Node::Map(map));
    }
    let value = if input.starts_with('"') {
        serde_json::from_str::<String>(input).map_err(SceneError::new)?
    } else {
        if input.starts_with(['*', '&', '!', '|', '>', '\'', '[', ']', '}', '#', '%'])
            || input.contains(": ")
            || input.contains(" #")
            || input.chars().any(char::is_control)
        {
            return Err(SceneError::new("unsupported text scalar syntax"));
        }
        input.to_owned()
    };
    text(&value)?;
    Ok(if input.starts_with('"') {
        Node::Quoted(value)
    } else {
        Node::Scalar(value)
    })
}
