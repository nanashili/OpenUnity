use crate::{Result, SceneError, text};
use yaml_rust2::{
    parser::Tag,
    scanner::{Marker, TScalarStyle},
};

use super::Node;
use crate::yaml::source_line;

pub(super) struct ScalarInput {
    pub(super) value: String,
    pub(super) style: TScalarStyle,
    pub(super) anchor: usize,
    pub(super) tag: Option<Tag>,
    pub(super) marker: Marker,
    pub(super) parent_is_flow: bool,
    pub(super) separator_is_empty: bool,
}

pub(super) fn mapping_key(
    source: &str,
    value: &str,
    style: TScalarStyle,
    anchor: usize,
    tag: Option<Tag>,
    marker: Marker,
) -> Result<(String, bool)> {
    if style != TScalarStyle::Plain || anchor != 0 || tag.is_some() {
        return Err(SceneError::new("unsupported mapping key syntax"));
    }

    let (_, line, offset) = source_line(source, marker.index())?;
    let tail = line
        .get(offset..)
        .ok_or_else(|| SceneError::new("mapping key boundary"))?;
    let colon = tail
        .find(':')
        .ok_or_else(|| SceneError::new("expected key/value separator"))?;
    let key = &tail[..colon];
    validate_key(key)?;

    if key != value {
        return Err(SceneError::new("unsupported mapping key spelling"));
    }

    let after_colon = &tail[colon + 1..];
    let trimmed_value = after_colon.trim_start_matches(' ');
    if !trimmed_value.is_empty()
        && !trimmed_value.starts_with(['}', ','])
        && !after_colon.starts_with(' ')
    {
        return Err(SceneError::new("unsupported mapping key or separator"));
    }

    let value_is_empty = trimmed_value.is_empty() || trimmed_value.starts_with(['}', ',']);
    Ok((key.to_owned(), value_is_empty))
}

pub(super) fn scalar_value(source: &str, input: ScalarInput) -> Result<Node> {
    if input.anchor != 0 || input.tag.is_some() {
        return Err(SceneError::new("unsupported YAML anchor or tag"));
    }

    let lexeme = if input.parent_is_flow {
        flow_scalar_lexeme(source, input.marker.index())?
    } else {
        block_scalar_lexeme(source, input.marker.index())?
    };

    match input.style {
        TScalarStyle::Plain => plain_node(&input.value, lexeme, input.separator_is_empty),
        TScalarStyle::DoubleQuoted => quoted_node(&input.value, lexeme),
        _ => Err(SceneError::new("unsupported text scalar style")),
    }
}

pub(super) fn validate_sequence_item_prefix(source: &str, marker: Marker) -> Result<()> {
    let (_, line, offset) = source_line(source, marker.index())?;
    let prefix = line
        .get(..offset)
        .ok_or_else(|| SceneError::new("sequence item boundary"))?;
    let indentation = prefix.len() - prefix.trim_start_matches(' ').len();
    let marker_prefix = prefix
        .get(indentation..)
        .ok_or_else(|| SceneError::new("sequence item boundary"))?;

    if marker_prefix != "- " {
        return Err(SceneError::new("unsupported sequence item syntax"));
    }

    Ok(())
}

pub(super) fn validate_plain_lexeme(value: &str) -> Result<String> {
    if value.starts_with(['*', '&', '!', '|', '>', '\'', '[', ']', '}', '#', '%'])
        || value.contains(": ")
        || value.contains(" #")
        || value.chars().any(char::is_control)
    {
        return Err(SceneError::new("unsupported text scalar syntax"));
    }

    text(value)?;
    Ok(value.to_owned())
}

fn validate_key(key: &str) -> Result<()> {
    let simple_identifier = !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    let supported_phrase = matches!(
        key,
        "orthographic size" | "field of view" | "near clip plane" | "far clip plane"
    );

    if simple_identifier || supported_phrase {
        Ok(())
    } else {
        Err(SceneError::new("unsupported mapping key or separator"))
    }
}

fn plain_node(value: &str, lexeme: &str, separator_is_empty: bool) -> Result<Node> {
    if value.is_empty() {
        if !separator_is_empty {
            return Err(SceneError::new("unsupported empty scalar syntax"));
        }
        return Ok(Node::Scalar(String::new()));
    }

    let normalized_lexeme = lexeme.trim_end_matches(' ');
    if normalized_lexeme != value {
        return Err(SceneError::new("unsupported plain scalar spelling"));
    }

    Ok(Node::Scalar(validate_plain_lexeme(lexeme)?))
}

fn quoted_node(value: &str, lexeme: &str) -> Result<Node> {
    let parsed = serde_json::from_str::<String>(lexeme).map_err(SceneError::new)?;
    if parsed != value {
        return Err(SceneError::new("unsupported quoted scalar spelling"));
    }

    text(&parsed)?;
    Ok(Node::Quoted(parsed))
}

fn block_scalar_lexeme(source: &str, start: usize) -> Result<&str> {
    let (_, line, offset) = source_line(source, start)?;
    line.get(offset..)
        .ok_or_else(|| SceneError::new("scalar boundary"))
}

fn flow_scalar_lexeme(source: &str, start: usize) -> Result<&str> {
    let (_, line, offset) = source_line(source, start)?;
    let tail = line
        .get(offset..)
        .ok_or_else(|| SceneError::new("scalar boundary"))?;
    let bytes = tail.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut nested_maps = 0usize;

    for (index, byte) in bytes.iter().copied().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }

        match byte {
            b'"' => quoted = true,
            b'{' => nested_maps += 1,
            b'}' if nested_maps > 0 => nested_maps -= 1,
            b'}' | b',' if nested_maps == 0 => {
                return Ok(tail[..index].trim_end_matches(' '));
            }
            _ => {}
        }
    }

    if quoted || nested_maps != 0 {
        return Err(SceneError::new("unbalanced flow mapping"));
    }

    Ok(tail.trim_end_matches(' '))
}
