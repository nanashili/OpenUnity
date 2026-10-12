//! Public contracts for bounded JSON parsing.

use engine_serialization::{
    MAX_DEPTH, MAX_INPUT_BYTES, MAX_OBJECTS, MAX_STRING_BYTES, parse_json, parse_plan,
};

#[test]
fn parse_json_accepts_generic_values_and_unknown_members() {
    let value =
        parse_json(br#"{"unknown":{"nested":[null,true,"brace } and ]"]},"count":2}"#).unwrap();
    assert_eq!(value["unknown"]["nested"][2], "brace } and ]");
    assert_eq!(value["count"], 2);
}

#[test]
fn parse_json_enforces_object_member_bounds() {
    let object = (0..MAX_OBJECTS)
        .map(|index| format!("\"k{index}\":null"))
        .collect::<Vec<_>>()
        .join(",");
    let object_at_limit = format!("{{{object}}}");
    assert_eq!(
        parse_json(object_at_limit.as_bytes())
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        MAX_OBJECTS
    );

    let too_many_members = format!("{{{object},\"overflow\":null}}");
    assert!(
        parse_json(too_many_members.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("JSON object member limit")
    );
}

#[test]
fn parse_json_enforces_array_item_bounds() {
    let array = format!(
        "[{}]",
        (0..MAX_OBJECTS)
            .map(|_| "null")
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(
        parse_json(array.as_bytes())
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        MAX_OBJECTS
    );

    let too_many_items = format!(
        "[{},null]",
        (0..MAX_OBJECTS)
            .map(|_| "null")
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(
        parse_json(too_many_items.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("JSON array item limit")
    );
}

#[test]
fn parse_json_enforces_decoded_string_byte_bounds() {
    let exact_string = format!("\"{}\"", "x".repeat(MAX_STRING_BYTES));
    assert_eq!(
        parse_json(exact_string.as_bytes())
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        MAX_STRING_BYTES
    );

    let oversized_string = format!("\"{}\"", "x".repeat(MAX_STRING_BYTES + 1));
    assert!(
        parse_json(oversized_string.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("oversized string")
    );

    let unicode_at_limit = format!("\"{}\"", "é".repeat(MAX_STRING_BYTES / 2));
    assert_eq!(
        parse_json(unicode_at_limit.as_bytes())
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        MAX_STRING_BYTES
    );
    let unicode_over_limit = format!("\"{}x\"", "é".repeat(MAX_STRING_BYTES / 2));
    assert_eq!(
        parse_json(unicode_over_limit.as_bytes())
            .unwrap_err()
            .to_string(),
        "oversized string or embedded NUL"
    );
}

#[test]
fn parse_json_enforces_decoded_key_byte_bounds() {
    let key = "é".repeat(MAX_STRING_BYTES / 2);
    let at_limit = format!("{{\"{key}\":null}}");
    assert!(parse_json(at_limit.as_bytes()).unwrap().get(&key).is_some());

    let over_limit = format!("{{\"{key}x\":null}}");
    assert_eq!(
        parse_json(over_limit.as_bytes()).unwrap_err().to_string(),
        "oversized string or embedded NUL"
    );
}

#[test]
fn parse_json_enforces_input_byte_bounds() {
    let exact_input = format!("{{}}{}", " ".repeat(MAX_INPUT_BYTES - 2));
    assert!(parse_json(exact_input.as_bytes()).is_ok());

    let over_input = format!("{{}}{}", " ".repeat(MAX_INPUT_BYTES - 1));
    for invalid in [b"".as_slice(), over_input.as_bytes()] {
        assert_eq!(
            parse_json(invalid).unwrap_err().to_string(),
            "JSON input byte limit"
        );
    }
}

#[test]
fn full_array_rejects_another_element_before_decoding_its_contents() {
    let admitted = vec!["null"; MAX_OBJECTS].join(",");
    let input = format!("[{admitted},{{\"key\":1,\"key\":2}}]");

    assert_eq!(
        parse_json(input.as_bytes()).unwrap_err().to_string(),
        "JSON array item limit"
    );
}

#[test]
fn parse_json_enforces_depth_and_malformed_delimiters() {
    let exact_depth = format!("{}0{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert!(parse_json(exact_depth.as_bytes()).unwrap().is_array());

    let excessive_depth = format!(
        "{}0{}",
        "[".repeat(MAX_DEPTH + 1),
        "]".repeat(MAX_DEPTH + 1)
    );
    assert!(
        parse_json(excessive_depth.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("JSON depth limit")
    );

    for malformed in [
        b"[{]".as_slice(),
        br#"{"x":[1}}"#.as_slice(),
        b"[1".as_slice(),
        br#"{"x":"unterminated"#.as_slice(),
        b"{} {}".as_slice(),
    ] {
        assert_eq!(
            parse_json(malformed).unwrap_err().to_string(),
            "malformed JSON",
            "incorrect category for malformed JSON: {malformed:?}"
        );
    }
}

#[test]
fn parse_json_rejects_duplicate_keys_at_each_object_boundary() {
    for duplicate in [
        br#"{"a":1,"a":2}"#.as_slice(),
        br#"{"outer":{"a":1,"a":1}}"#.as_slice(),
        br#"{"a":1,"\u0061":2}"#.as_slice(),
    ] {
        assert!(
            parse_json(duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicate JSON key")
        );
    }
}

#[test]
fn parse_plan_rejects_unknown_fields_after_json_succeeds() {
    let valid = br#"{"schema_version":1,"scene_name":"Root","roots":[10],"game_objects":[{"file_id":1,"name":"Root","active":true,"components":[10]}],"transforms":[{"file_id":10,"owner":1,"parent":0,"children":[],"local_position":[0.0,0.0,0.0],"local_rotation":[0.0,0.0,0.0,1.0],"local_scale":[1.0,1.0,1.0]}],"scripts":[]}"#;
    assert!(parse_plan(valid).is_ok());

    let with_unknown = br#"{"schema_version":1,"scene_name":"Root","roots":[10],"game_objects":[{"file_id":1,"name":"Root","active":true,"components":[10],"future":true}],"transforms":[{"file_id":10,"owner":1,"parent":0,"children":[],"local_position":[0.0,0.0,0.0],"local_rotation":[0.0,0.0,0.0,1.0],"local_scale":[1.0,1.0,1.0]}],"scripts":[]}"#;
    assert!(parse_json(with_unknown).is_ok());
    assert!(parse_plan(with_unknown).is_err());
}

#[test]
fn parse_json_rejects_invalid_utf8_and_embedded_nul() {
    assert_eq!(
        parse_json(b"\"\xff\"").unwrap_err().to_string(),
        "malformed JSON"
    );
    assert_eq!(
        parse_json(br#""nul\u0000inside""#).unwrap_err().to_string(),
        "oversized string or embedded NUL"
    );
}
