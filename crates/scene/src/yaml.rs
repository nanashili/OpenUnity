//! Small parser for the explicitly reviewed authored text grammar, not YAML.
use crate::{MAX_DEPTH, MAX_INPUT_BYTES, MAX_OBJECTS, Result, SceneError, text};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Scalar(String),
    Quoted(String),
    Map(BTreeMap<String, Node>),
    Seq(Vec<Node>),
}
impl Node {
    pub(crate) fn map(&self) -> Result<&BTreeMap<String, Node>> {
        if let Self::Map(m) = self {
            Ok(m)
        } else {
            Err(SceneError::new("expected mapping"))
        }
    }
    pub(crate) fn seq(&self) -> Result<&[Node]> {
        if let Self::Seq(s) = self {
            Ok(s)
        } else {
            Err(SceneError::new("expected sequence"))
        }
    }
    pub(crate) fn scalar(&self) -> Result<&str> {
        if let Self::Scalar(s) | Self::Quoted(s) = self {
            Ok(s)
        } else {
            Err(SceneError::new("expected scalar"))
        }
    }
}
pub(crate) struct Document {
    pub class_id: i64,
    pub file_id: i64,
    pub node: Node,
}
struct Line<'a> {
    indent: usize,
    body: &'a str,
}
fn pair(input: &str) -> Result<(&str, &str)> {
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
fn inline(input: &str, depth: usize) -> Result<Node> {
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
fn block(lines: &[Line<'_>], pos: &mut usize, indent: usize, depth: usize) -> Result<Node> {
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
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<Document>> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(SceneError::new("scene byte limit"));
    }
    let source = std::str::from_utf8(bytes).map_err(SceneError::new)?;
    if source.contains(['\0', '\r', '\t']) {
        return Err(SceneError::new("unsupported scene encoding/control"));
    }
    let mut lines = source.lines();
    for expected in ["%UNITY 7.0", "%YAML 1.1", "%TAG !u! tag:unity3d.com,2011:"] {
        if lines.next() != Some(expected) {
            return Err(SceneError::new("unsupported saved scene directives"));
        }
    }
    let mut result = Vec::new();
    let mut header = None;
    let mut body = Vec::new();
    let finish = |header: Option<(i64, i64)>,
                  body: &mut Vec<Line<'_>>,
                  result: &mut Vec<Document>|
     -> Result<()> {
        if let Some((class_id, file_id)) = header {
            let mut pos = 0;
            let node = block(body, &mut pos, 0, 0)?;
            if pos != body.len() {
                return Err(SceneError::new("unconsumed document body"));
            }
            result.push(Document {
                class_id,
                file_id,
                node,
            });
            body.clear();
            if result.len() > MAX_OBJECTS {
                return Err(SceneError::new("document count limit"));
            }
        }
        Ok(())
    };
    for line in lines {
        if let Some(rest) = line.strip_prefix("--- !u!") {
            finish(header.take(), &mut body, &mut result)?;
            let (class, file) = rest
                .split_once(" &")
                .ok_or_else(|| SceneError::new("unsupported document header"))?;
            if !class.bytes().all(|b| b.is_ascii_digit())
                || !file.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(SceneError::new("invalid document header number"));
            }
            let class_id = class.parse().map_err(SceneError::new)?;
            let file_id = file.parse().map_err(SceneError::new)?;
            if file_id <= 0 {
                return Err(SceneError::new("nonpositive document fileID"));
            }
            header = Some((class_id, file_id));
        } else {
            if header.is_none() || line.is_empty() {
                return Err(SceneError::new("unsupported document syntax"));
            }
            let indent = line.len() - line.trim_start_matches(' ').len();
            if indent % 2 != 0 || indent / 2 > MAX_DEPTH {
                return Err(SceneError::new("unsupported indentation"));
            }
            body.push(Line {
                indent,
                body: &line[indent..],
            });
        }
    }
    finish(header, &mut body, &mut result)?;
    if result.is_empty() {
        return Err(SceneError::new("empty scene"));
    }
    Ok(result)
}
