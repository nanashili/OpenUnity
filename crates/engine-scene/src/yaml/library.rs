mod flow;
mod mapping;
mod sequence;

use crate::{MAX_DEPTH, Result, SceneError};
use yaml_rust2::{
    parser::{Event, Parser, Tag},
    scanner::{Marker, TScalarStyle},
};

use super::{Line, Node, scalar};

#[derive(Clone, Copy)]
pub(super) enum NodePosition {
    Root,
    MappingValue {
        parent_is_flow: bool,
        key_marker: Marker,
        separator_is_empty: bool,
    },
    SequenceItem,
}

pub(super) fn parse_block_document(lines: &[Line<'_>]) -> Result<Node> {
    if lines.is_empty() {
        return Err(SceneError::new("invalid text indentation/depth"));
    }

    let source = document_source(lines);
    let mut parser = Parser::new_from_str(&source);
    expect_event(&mut parser, Event::StreamStart)?;
    expect_event(&mut parser, Event::DocumentStart)?;

    let (event, marker) = next_event(&mut parser)?;
    let root = parse_node(&mut parser, &source, event, marker, 0, NodePosition::Root)?;

    expect_event(&mut parser, Event::DocumentEnd)?;
    expect_event(&mut parser, Event::StreamEnd)?;

    Ok(root)
}

fn parse_node<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
    source: &str,
    event: Event,
    marker: Marker,
    depth: usize,
    position: NodePosition,
) -> Result<Node> {
    if depth > MAX_DEPTH {
        return Err(SceneError::new("text nesting limit"));
    }

    match event {
        Event::Scalar(value, style, anchor, tag) => {
            parse_scalar(source, value, style, anchor, tag, marker, position)
        }
        Event::MappingStart(anchor, tag) => {
            mapping::parse_mapping(parser, source, anchor, tag, marker, depth, position)
        }
        Event::SequenceStart(anchor, tag) => {
            sequence::parse_sequence(parser, source, anchor, tag, marker, depth, position)
        }
        Event::Alias(_) => Err(SceneError::new("unsupported YAML alias")),
        _ => Err(SceneError::new("unsupported YAML node")),
    }
}

fn parse_scalar(
    source: &str,
    value: String,
    style: TScalarStyle,
    anchor: usize,
    tag: Option<Tag>,
    marker: Marker,
    position: NodePosition,
) -> Result<Node> {
    let NodePosition::MappingValue {
        parent_is_flow,
        separator_is_empty,
        ..
    } = position
    else {
        return Err(SceneError::new("unsupported scalar node position"));
    };

    scalar::scalar_value(
        source,
        scalar::ScalarInput {
            value,
            style,
            anchor,
            tag,
            marker,
            parent_is_flow,
            separator_is_empty,
        },
    )
}

fn document_source(lines: &[Line<'_>]) -> String {
    let capacity = lines
        .iter()
        .map(|line| line.indent + line.body.len() + 1)
        .sum();
    let mut source = String::with_capacity(capacity);

    for line in lines {
        for _ in 0..line.indent {
            source.push(' ');
        }
        source.push_str(line.body);
        source.push('\n');
    }

    source
}

fn expect_event<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
    expected: Event,
) -> Result<Marker> {
    let (event, marker) = next_event(parser)?;
    if event == expected {
        Ok(marker)
    } else {
        Err(SceneError::new("unexpected YAML stream event"))
    }
}

pub(super) fn next_event<T: Iterator<Item = char>>(
    parser: &mut Parser<T>,
) -> Result<(Event, Marker)> {
    parser.next_token().map_err(SceneError::new)
}

pub(super) fn byte_at(source: &str, marker: Marker) -> Option<u8> {
    source.as_bytes().get(marker.index()).copied()
}

pub(super) fn is_flow_collection(source: &str, event: &Event, marker: Marker) -> bool {
    match event {
        Event::MappingStart(..) => byte_at(source, marker) == Some(b'{'),
        Event::SequenceStart(..) => byte_at(source, marker) == Some(b'['),
        _ => false,
    }
}

pub(super) fn reject_metadata(anchor: usize, tag: Option<Tag>) -> Result<()> {
    if anchor != 0 || tag.is_some() {
        return Err(SceneError::new("unsupported YAML anchor or tag"));
    }
    Ok(())
}
