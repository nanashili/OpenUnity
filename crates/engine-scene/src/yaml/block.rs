use super::{
    Node,
    inline::{inline, pair},
};
use crate::{MAX_DEPTH, Result, SceneError};
use std::collections::BTreeMap;

pub(super) struct Line<'a> {
    pub(super) indent: usize,
    pub(super) body: &'a str,
}
pub(super) fn block(
    lines: &[Line<'_>],
    pos: &mut usize,
    indent: usize,
    depth: usize,
) -> Result<Node> {
    if depth > MAX_DEPTH || lines.get(*pos).is_none_or(|l| l.indent != indent) {
        return Err(SceneError::new("invalid text indentation/depth"));
    }
    if lines[*pos].body.starts_with("- ") {
        let mut seq = Vec::new();
        while let Some(line) = lines.get(*pos) {
            if line.indent != indent || !line.body.starts_with("- ") {
                break;
            }
            let value = &line.body[2..];
            seq.push(if value.starts_with('{') {
                inline(value, depth + 1)?
            } else {
                let (key, value) = pair(value)?;
                Node::Map(BTreeMap::from([(
                    key.to_owned(),
                    inline(value, depth + 1)?,
                )]))
            });
            *pos += 1;
        }
        return Ok(Node::Seq(seq));
    }
    let mut map = BTreeMap::new();
    while let Some(line) = lines.get(*pos) {
        if line.indent != indent || line.body.starts_with("- ") {
            break;
        }
        let (key, value) = pair(line.body)?;
        *pos += 1;
        let value = if value.is_empty()
            && lines.get(*pos).is_some_and(|next| {
                next.indent > indent || (next.indent == indent && next.body.starts_with("- "))
            }) {
            block(lines, pos, lines[*pos].indent, depth + 1)?
        } else {
            inline(value, depth + 1)?
        };
        if map.insert(key.to_owned(), value).is_some() {
            return Err(SceneError::new("duplicate mapping key"));
        }
    }
    Ok(Node::Map(map))
}
