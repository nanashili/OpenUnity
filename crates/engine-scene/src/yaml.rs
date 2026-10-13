//! Adapter for the explicitly reviewed authored scene-text grammar.
use crate::{MAX_DEPTH, MAX_INPUT_BYTES, MAX_OBJECTS, Result, SceneError};

mod library;
mod node;
mod scalar;

pub(crate) use node::Node;

pub(super) struct Line<'a> {
    pub(super) indent: usize,
    pub(super) body: &'a str,
}

pub(crate) struct Document {
    pub class_id: i64,
    pub file_id: i64,
    pub node: Node,
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
    for line in lines {
        if let Some(rest) = line.strip_prefix("--- !u!") {
            finish_document(header.take(), &mut body, &mut result)?;
            header = Some(parse_document_header(rest)?);
        } else {
            if header.is_none() || line.is_empty() {
                return Err(SceneError::new("unsupported document syntax"));
            }
            let indent = line.len() - line.trim_start_matches(' ').len();
            if indent % 2 != 0 || indent / 2 > MAX_DEPTH {
                return Err(SceneError::new("unsupported indentation"));
            }
            let body_text = &line[indent..];
            if body_text.is_empty()
                || body_text.starts_with(['#', '%'])
                || body_text.starts_with("...")
            {
                return Err(SceneError::new("unsupported document syntax"));
            }
            body.push(Line {
                indent,
                body: body_text,
            });
        }
    }
    finish_document(header, &mut body, &mut result)?;
    if result.is_empty() {
        return Err(SceneError::new("empty scene"));
    }
    Ok(result)
}

fn parse_document_header(rest: &str) -> Result<(i64, i64)> {
    let (class, file) = rest
        .split_once(" &")
        .ok_or_else(|| SceneError::new("unsupported document header"))?;
    if !class.bytes().all(|byte| byte.is_ascii_digit())
        || !file.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(SceneError::new("invalid document header number"));
    }

    let class_id = class.parse().map_err(SceneError::new)?;
    let file_id = file.parse().map_err(SceneError::new)?;
    if file_id <= 0 {
        return Err(SceneError::new("nonpositive document fileID"));
    }

    Ok((class_id, file_id))
}

fn finish_document(
    header: Option<(i64, i64)>,
    body: &mut Vec<Line<'_>>,
    result: &mut Vec<Document>,
) -> Result<()> {
    let Some((class_id, file_id)) = header else {
        return Ok(());
    };
    if body.first().is_none_or(|line| line.indent != 0) {
        return Err(SceneError::new("invalid text indentation/depth"));
    }

    let node = library::parse_block_document(body)?;
    result.push(Document {
        class_id,
        file_id,
        node,
    });
    body.clear();

    if result.len() > MAX_OBJECTS {
        return Err(SceneError::new("document count limit"));
    }

    Ok(())
}

fn source_line(source: &str, index: usize) -> Result<(usize, &str, usize)> {
    let prefix = source
        .get(..index)
        .ok_or_else(|| SceneError::new("source marker boundary"))?;
    let line_start = prefix.rfind('\n').map_or(0, |newline| newline + 1);
    let line_end = source[index..]
        .find('\n')
        .map_or(source.len(), |newline| index + newline);
    let line = source
        .get(line_start..line_end)
        .ok_or_else(|| SceneError::new("source line boundary"))?;
    let offset = index - line_start;

    Ok((line_start, line, offset))
}
