use std::collections::BTreeMap;

use crate::{Result, SceneError};
use yaml_rust2::{
    parser::{Event, Parser, Tag},
    scanner::Marker,
};

use super::{NodePosition, byte_at, flow, is_flow_collection, next_event, reject_metadata};
use crate::yaml::{Node, scalar};

struct MappingValueContext<'a> {
    source: &'a str,
    parent_is_flow: bool,
    key_marker: Marker,
    value_marker: Marker,
    separator_is_empty: bool,
    event: &'a Event,
    value: &'a Node,
    is_block_sequence_item: bool,
}

pub(super) fn parse_mapping<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
    source: &str,
    anchor: usize,
    tag: Option<Tag>,
    marker: Marker,
    depth: usize,
    position: NodePosition,
) -> Result<Node> {
    reject_metadata(anchor, tag)?;

    let is_flow = byte_at(source, marker) == Some(b'{');
    validate_mapping_position(marker, is_flow, position)?;

    if matches!(position, NodePosition::SequenceItem) && is_flow {
        scalar::validate_sequence_item_prefix(source, marker)?;
    }

    let mut fields = BTreeMap::new();
    let mut entry_count = 0usize;

    loop {
        let (key_event, key_marker) = next_event(parser)?;
        if let Event::MappingEnd = key_event {
            return finish_mapping(source, marker, key_marker, position, is_flow, fields);
        }

        if is_extra_block_sequence_field(position, is_flow, entry_count) {
            return Err(SceneError::new("unsupported sequence mapping shape"));
        }

        let (key, separator_is_empty) = parse_mapping_key(source, key_event, key_marker)?;
        validate_sequence_key(source, key_marker, position, is_flow)?;

        let (value_event, value_marker) = next_event(parser)?;
        let value = super::parse_node(
            parser,
            source,
            value_event.clone(),
            value_marker,
            depth + 1,
            NodePosition::MappingValue {
                parent_is_flow: is_flow,
                key_marker,
                separator_is_empty,
            },
        )?;
        validate_mapping_value(MappingValueContext {
            source,
            parent_is_flow: is_flow,
            key_marker,
            value_marker,
            separator_is_empty,
            event: &value_event,
            value: &value,
            is_block_sequence_item: matches!(position, NodePosition::SequenceItem) && !is_flow,
        })?;

        if fields.insert(key, value).is_some() {
            return Err(SceneError::new("duplicate mapping key"));
        }
        entry_count += 1;
    }
}

fn finish_mapping(
    source: &str,
    start: Marker,
    end: Marker,
    position: NodePosition,
    is_flow: bool,
    fields: BTreeMap<String, Node>,
) -> Result<Node> {
    if fields.is_empty() {
        return Err(SceneError::new("empty flow mapping"));
    }

    if is_flow {
        let parent_is_flow = matches!(
            position,
            NodePosition::MappingValue {
                parent_is_flow: true,
                ..
            }
        );
        flow::validate_mapping_end(source, start, end, parent_is_flow)?;
    }

    Ok(Node::Map(fields))
}

fn is_extra_block_sequence_field(
    position: NodePosition,
    is_flow: bool,
    entry_count: usize,
) -> bool {
    matches!(position, NodePosition::SequenceItem) && !is_flow && entry_count > 0
}

fn validate_sequence_key(
    source: &str,
    marker: Marker,
    position: NodePosition,
    is_flow: bool,
) -> Result<()> {
    if matches!(position, NodePosition::SequenceItem) && !is_flow {
        scalar::validate_sequence_item_prefix(source, marker)?;
    }
    Ok(())
}

fn parse_mapping_key(source: &str, event: Event, marker: Marker) -> Result<(String, bool)> {
    match event {
        Event::Scalar(value, style, anchor, tag) => {
            scalar::mapping_key(source, &value, style, anchor, tag, marker)
        }
        Event::Alias(_) => Err(SceneError::new("unsupported YAML alias")),
        _ => Err(SceneError::new("unsupported mapping key syntax")),
    }
}

fn validate_mapping_position(marker: Marker, is_flow: bool, position: NodePosition) -> Result<()> {
    match position {
        NodePosition::Root if is_flow => Err(SceneError::new("unsupported root flow mapping")),
        NodePosition::Root => Ok(()),
        NodePosition::MappingValue {
            parent_is_flow,
            key_marker,
            separator_is_empty,
        } => {
            let is_same_line = marker.line() == key_marker.line();
            let valid = if parent_is_flow || is_flow {
                is_flow && is_same_line
            } else {
                separator_is_empty && marker.line() > key_marker.line()
            };

            if valid {
                Ok(())
            } else {
                Err(SceneError::new("unsupported nested mapping placement"))
            }
        }
        NodePosition::SequenceItem => Ok(()),
    }
}

fn validate_mapping_value(context: MappingValueContext<'_>) -> Result<()> {
    let same_line = context.key_marker.line() == context.value_marker.line();
    match context.value {
        Node::Map(_) | Node::Seq(_) => {
            let child_is_flow =
                is_flow_collection(context.source, context.event, context.value_marker);
            validate_collection_value(
                child_is_flow,
                same_line,
                context.parent_is_flow,
                context.is_block_sequence_item,
                context.separator_is_empty,
                context.key_marker,
                context.value_marker,
            )
        }
        Node::Scalar(value) if value.is_empty() => {
            if context.separator_is_empty {
                Ok(())
            } else {
                Err(SceneError::new("unsupported empty scalar syntax"))
            }
        }
        Node::Scalar(_) | Node::Quoted(_) if same_line => Ok(()),
        Node::Scalar(_) | Node::Quoted(_) => Err(SceneError::new("unsupported multiline scalar")),
    }
}

fn validate_collection_value(
    child_is_flow: bool,
    same_line: bool,
    parent_is_flow: bool,
    is_block_sequence_item: bool,
    separator_is_empty: bool,
    key_marker: Marker,
    value_marker: Marker,
) -> Result<()> {
    if child_is_flow {
        return if same_line {
            Ok(())
        } else {
            Err(SceneError::new("unsupported inline collection placement"))
        };
    }

    let is_nested_block_value = !parent_is_flow
        && !is_block_sequence_item
        && separator_is_empty
        && value_marker.line() > key_marker.line();
    if is_nested_block_value {
        Ok(())
    } else {
        Err(SceneError::new("unsupported nested collection placement"))
    }
}
