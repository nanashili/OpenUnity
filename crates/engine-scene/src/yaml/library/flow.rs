use crate::{Result, SceneError};
use yaml_rust2::scanner::Marker;

use super::byte_at;
use crate::yaml::source_line;

pub(super) fn validate_mapping_end(
    source: &str,
    start: Marker,
    end: Marker,
    parent_is_flow: bool,
) -> Result<()> {
    validate_mapping_extent(source, start, end)?;
    reject_trailing_mapping_separator(source, end)?;
    validate_flow_suffix(source, end.index() + 1, parent_is_flow, "mapping")
}

fn validate_mapping_extent(source: &str, start: Marker, end: Marker) -> Result<()> {
    if start.line() != end.line() || byte_at(source, start) != Some(b'{') {
        return Err(SceneError::new("unsupported flow mapping extent"));
    }
    Ok(())
}

fn reject_trailing_mapping_separator(source: &str, end: Marker) -> Result<()> {
    let before_closer = source
        .get(..end.index())
        .ok_or_else(|| SceneError::new("flow mapping boundary"))?;
    if before_closer.trim_end_matches(' ').ends_with(',') {
        return Err(SceneError::new("trailing flow mapping separator"));
    }
    Ok(())
}

pub(super) fn validate_sequence_end(source: &str, end: Marker, parent_is_flow: bool) -> Result<()> {
    validate_flow_suffix(source, end.index() + 1, parent_is_flow, "sequence")
}

fn validate_flow_suffix(
    source: &str,
    index: usize,
    parent_is_flow: bool,
    kind: &str,
) -> Result<()> {
    let (_, line, offset) = source_line(source, index)?;
    let suffix = line
        .get(offset..)
        .ok_or_else(|| SceneError::new(format!("flow {kind} boundary")))?;

    if parent_is_flow {
        let delimiter = suffix.find([',', '}']).unwrap_or(suffix.len());
        if !suffix[..delimiter].bytes().all(|byte| byte == b' ') {
            return Err(SceneError::new(format!("unsupported flow {kind} suffix")));
        }
    } else if !suffix.is_empty() {
        return Err(SceneError::new(format!("unsupported flow {kind} suffix")));
    }

    Ok(())
}
