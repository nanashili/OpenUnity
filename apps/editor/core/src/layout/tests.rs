use super::*;

#[test]
fn round_trip_preserves_visibility_and_boundary_sizes() {
    let layout = Layout {
        project: false,
        hierarchy: true,
        inspector: false,
        console: true,
        left_width: MIN_PANEL_SIZE,
        right_width: MAX_PANEL_SIZE,
        console_height: 321.5,
    };
    let encoded = layout.encode().unwrap();
    assert_eq!(Layout::decode(&encoded).unwrap(), layout);
    assert!(
        String::from_utf8(encoded)
            .unwrap()
            .contains("openunity.layout")
    );
}

#[test]
fn reset_restores_every_field_even_from_an_invalid_layout() {
    let mut layout = Layout {
        project: false,
        hierarchy: false,
        inspector: false,
        console: false,
        left_width: f32::NAN,
        right_width: 1.0,
        console_height: 900.0,
    };
    layout.reset();
    assert_eq!(layout, Layout::default());
    assert!(layout.validate().is_ok());
}

#[test]
fn all_dimensions_reject_nonfinite_and_out_of_bounds_values() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 99.0, 801.0] {
        for field in ["left_width", "right_width", "console_height"] {
            let mut layout = Layout::default();
            match field {
                "left_width" => layout.left_width = invalid,
                "right_width" => layout.right_width = invalid,
                _ => layout.console_height = invalid,
            }
            assert_eq!(layout.validate(), Err(LayoutError::InvalidSize(field)));
            assert_eq!(layout.encode(), Err(LayoutError::InvalidSize(field)));
        }
    }
}

#[test]
fn decode_rejects_invalid_sizes_and_unsupported_identity() {
    let json = String::from_utf8(Layout::default().encode().unwrap()).unwrap();
    assert_eq!(
        Layout::decode(json.replace("240.0", "99.0").as_bytes()),
        Err(LayoutError::InvalidSize("left_width"))
    );
    assert_eq!(
        Layout::decode(json.replace("\"version\": 1", "\"version\": 2").as_bytes()),
        Err(LayoutError::UnsupportedVersion(2))
    );
    assert_eq!(
        Layout::decode(
            json.replace("openunity.layout", "unrelated.layout")
                .as_bytes()
        ),
        Err(LayoutError::UnsupportedFormat)
    );
}

#[test]
fn decode_rejects_unknown_duplicate_and_missing_fields() {
    let json = String::from_utf8(Layout::default().encode().unwrap()).unwrap();
    for input in [
        json.replacen('{', "{\"extra\": true,", 1),
        json.replacen('{', "{\"version\": 1,", 1),
        json.replacen('{', "{\"left_width\": 200,", 1),
        json.replace("  \"project\": true,\n", ""),
    ] {
        assert!(matches!(
            Layout::decode(input.as_bytes()),
            Err(LayoutError::Json(_))
        ));
    }
}

#[test]
fn decode_bounds_bytes_before_parsing() {
    let mut bytes = Layout::default().encode().unwrap();
    bytes.resize(MAX_LAYOUT_BYTES, b' ');
    assert_eq!(Layout::decode(&bytes).unwrap(), Layout::default());
    bytes.push(b' ');
    assert_eq!(Layout::decode(&bytes), Err(LayoutError::InputTooLarge));
}
