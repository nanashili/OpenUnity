use crate::{Result, SceneError};
use yaml_rust2::{
    parser::{Event, Parser, Tag},
    scanner::Marker,
};

use super::{NodePosition, byte_at, flow, next_event, reject_metadata};
use crate::yaml::Node;

pub(super) fn parse_sequence<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
    source: &str,
    anchor: usize,
    tag: Option<Tag>,
    marker: Marker,
    depth: usize,
    position: NodePosition,
) -> Result<Node> {
    reject_metadata(anchor, tag)?;

    let is_flow = byte_at(source, marker) == Some(b'[');
    validate_sequence_position(marker, is_flow, position)?;

    if is_flow {
        return parse_empty_flow_sequence(parser, source, marker, position);
    }

    parse_block_sequence(parser, source, depth)
}

fn parse_empty_flow_sequence<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
    source: &str,
    marker: Marker,
    position: NodePosition,
) -> Result<Node> {
    let (end_event, end_marker) = next_event(parser)?;
    let is_empty_sequence = end_event == Event::SequenceEnd
        && source
            .get(marker.index()..=end_marker.index())
            .is_some_and(|lexeme| lexeme == "[]");
    if !is_empty_sequence {
        return Err(SceneError::new("unsupported flow sequence"));
    }

    let parent_is_flow = matches!(
        position,
        NodePosition::MappingValue {
            parent_is_flow: true,
            ..
        }
    );
    flow::validate_sequence_end(source, end_marker, parent_is_flow)?;

    Ok(Node::Seq(Vec::new()))
}

fn parse_block_sequence<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
    source: &str,
    depth: usize,
) -> Result<Node> {
    let mut items = Vec::new();

    loop {
        let (event, item_marker) = next_event(parser)?;
        if event == Event::SequenceEnd {
            return Ok(Node::Seq(items));
        }

        let item = super::parse_node(
            parser,
            source,
            event,
            item_marker,
            depth,
            NodePosition::SequenceItem,
        )?;
        if !matches!(item, Node::Map(_)) {
            return Err(SceneError::new("unsupported sequence item shape"));
        }

        items.push(item);
    }
}

fn validate_sequence_position(marker: Marker, is_flow: bool, position: NodePosition) -> Result<()> {
    match position {
        NodePosition::Root if is_flow => Err(SceneError::new("unsupported root flow sequence")),
        NodePosition::Root => Ok(()),
        NodePosition::MappingValue {
            parent_is_flow,
            key_marker,
            separator_is_empty,
        } => {
            let is_same_line = marker.line() == key_marker.line();
            let valid = if is_flow {
                is_same_line
            } else {
                !parent_is_flow && separator_is_empty && marker.line() > key_marker.line()
            };

            if valid {
                Ok(())
            } else {
                Err(SceneError::new("unsupported nested sequence placement"))
            }
        }
        NodePosition::SequenceItem => Err(SceneError::new("unsupported nested sequence item")),
    }
}
