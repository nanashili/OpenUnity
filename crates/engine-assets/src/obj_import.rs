use crate::mesh_asset::{MAX_MESH_INDICES, MAX_MESH_POSITIONS};
use crate::{MeshAsset, obj_import_error::ObjImportError};

const MAX_OBJ_BYTES: usize = 16 * 1024 * 1024;

/// Parses the deliberately bounded, triangle-only OBJ subset without I/O.
pub fn parse_obj_mesh(id: u32, bytes: &[u8]) -> Result<MeshAsset, ObjImportError> {
    if bytes.len() > MAX_OBJ_BYTES {
        return Err(ObjImportError::InputTooLarge);
    }
    validate_text(bytes)?;

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for (line_index, raw_line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        let mut tokens = Tokens::new(line);
        let Some(statement) = tokens.next() else {
            continue;
        };
        if statement.first() == Some(&b'#') {
            continue;
        }
        let line_number = line_index + 1;
        match statement {
            b"v" => parse_vertex(&mut tokens, line_number, &mut positions)?,
            b"f" => parse_face(&mut tokens, line_number, positions.len(), &mut indices)?,
            _ => return Err(ObjImportError::UnsupportedStatement { line: line_number }),
        }
    }
    MeshAsset::try_new(id, positions, indices).map_err(Into::into)
}

fn validate_text(bytes: &[u8]) -> Result<(), ObjImportError> {
    for (offset, byte) in bytes.iter().copied().enumerate() {
        if byte >= 0x80 {
            return Err(ObjImportError::NonAscii {
                byte_offset: offset,
            });
        }
        if byte == b'\r' && bytes.get(offset + 1) != Some(&b'\n') {
            return Err(ObjImportError::InvalidTextByte {
                byte_offset: offset,
            });
        }
        if byte == 0x7f || (byte < b' ' && !matches!(byte, b'\t' | b'\n' | b'\r')) {
            return Err(ObjImportError::InvalidTextByte {
                byte_offset: offset,
            });
        }
    }
    Ok(())
}

struct Tokens<'a> {
    line: &'a [u8],
    offset: usize,
}

impl<'a> Tokens<'a> {
    fn new(line: &'a [u8]) -> Self {
        Self { line, offset: 0 }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        while self
            .line
            .get(self.offset)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
        {
            self.offset += 1;
        }
        let start = self.offset;
        while self
            .line
            .get(self.offset)
            .is_some_and(|byte| !matches!(byte, b' ' | b'\t'))
        {
            self.offset += 1;
        }
        (self.offset > start).then(|| &self.line[start..self.offset])
    }
}

fn parse_vertex(
    tokens: &mut Tokens<'_>,
    line: usize,
    positions: &mut Vec<[f32; 3]>,
) -> Result<(), ObjImportError> {
    let Some(x) = tokens.next() else {
        return Err(ObjImportError::MalformedVertex { line });
    };
    let Some(y) = tokens.next() else {
        return Err(ObjImportError::MalformedVertex { line });
    };
    let Some(z) = tokens.next() else {
        return Err(ObjImportError::MalformedVertex { line });
    };
    if tokens.next().is_some() {
        return Err(ObjImportError::MalformedVertex { line });
    }
    if positions.len() >= MAX_MESH_POSITIONS {
        return Err(ObjImportError::PositionLimitExceeded { line });
    }
    let mut coordinates = [0.0; 3];
    for (coordinate, token) in coordinates.iter_mut().zip([x, y, z]) {
        *coordinate = parse_coordinate(token).ok_or(ObjImportError::InvalidCoordinate { line })?;
    }
    positions.push(coordinates);
    Ok(())
}

fn parse_face(
    tokens: &mut Tokens<'_>,
    line: usize,
    position_count: usize,
    indices: &mut Vec<u32>,
) -> Result<(), ObjImportError> {
    let Some(first) = tokens.next() else {
        return Err(ObjImportError::UnsupportedFaceArity { line });
    };
    let Some(second) = tokens.next() else {
        return Err(ObjImportError::UnsupportedFaceArity { line });
    };
    let Some(third) = tokens.next() else {
        return Err(ObjImportError::UnsupportedFaceArity { line });
    };
    if tokens.next().is_some() {
        return Err(ObjImportError::UnsupportedFaceArity { line });
    }
    if indices.len() > MAX_MESH_INDICES - 3 {
        return Err(ObjImportError::IndexLimitExceeded { line });
    }
    let mut face = [0_u32; 3];
    for (index, token) in face.iter_mut().zip([first, second, third]) {
        *index = parse_face_index(token, line, position_count)?;
    }
    indices.extend(face);
    Ok(())
}

fn parse_coordinate(token: &[u8]) -> Option<f32> {
    if !decimal_grammar(token) {
        return None;
    }
    let text = std::str::from_utf8(token).ok()?;
    let value = text.parse::<f32>().ok()?;
    value.is_finite().then_some(value)
}

fn decimal_grammar(token: &[u8]) -> bool {
    let mut cursor = usize::from(matches!(token.first(), Some(b'+' | b'-')));
    let before = consume_digits(token, &mut cursor);
    let after = if token.get(cursor) == Some(&b'.') {
        cursor += 1;
        consume_digits(token, &mut cursor)
    } else {
        false
    };
    if !before && !after {
        return false;
    }
    if matches!(token.get(cursor), Some(b'e' | b'E')) {
        cursor += 1;
        cursor += usize::from(matches!(token.get(cursor), Some(b'+' | b'-')));
        if !consume_digits(token, &mut cursor) {
            return false;
        }
    }
    cursor == token.len()
}

fn consume_digits(token: &[u8], cursor: &mut usize) -> bool {
    let start = *cursor;
    while token.get(*cursor).is_some_and(u8::is_ascii_digit) {
        *cursor += 1;
    }
    *cursor != start
}

fn parse_face_index(
    token: &[u8],
    line: usize,
    position_count: usize,
) -> Result<u32, ObjImportError> {
    if token.contains(&b'/') || token.first() == Some(&b'-') {
        return Err(ObjImportError::UnsupportedFaceReference { line });
    }
    if token.is_empty() || !token.iter().all(u8::is_ascii_digit) {
        return Err(ObjImportError::InvalidFaceReference { line });
    }
    let parsed = std::str::from_utf8(token)
        .ok()
        .and_then(|text| text.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .ok_or(ObjImportError::InvalidFaceReference { line })?;
    let zero_based = parsed - 1;
    if usize::try_from(zero_based)
        .ok()
        .is_none_or(|index| index >= position_count)
    {
        return Err(ObjImportError::InvalidFaceReference { line });
    }
    Ok(zero_based)
}
