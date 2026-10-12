use super::SceneIoError;

const MAX_JSON_DEPTH: usize = 32;

// Count containers before serde allocates document fields. Escapes apply to
// exactly the following byte; brackets and escaped quotes inside strings do
// not alter nesting. Serde subsequently validates all JSON grammar and UTF-8.
pub(super) fn check_json_depth(bytes: &[u8]) -> Result<(), SceneIoError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_JSON_DEPTH {
                        return Err(SceneIoError::new(
                            "decode scene",
                            "JSON nesting exceeds 32 levels",
                        ));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}
