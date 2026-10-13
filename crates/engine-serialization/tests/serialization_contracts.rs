//! Public contracts for versioned scene plan serialization.

use engine_serialization::{
    FieldPlan, FieldValue, GameObjectPlan, MAX_FIELDS, MAX_INPUT_BYTES, MAX_STRING_BYTES,
    ScenePlan, ScriptPlan, TransformPlan, parse_plan, serialize_plan,
};

fn valid_plan(scene_name: &str) -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: scene_name.to_owned(),
        roots: vec![10],
        game_objects: vec![GameObjectPlan {
            file_id: 1,
            name: "Root".to_owned(),
            active: true,
            components: vec![10, 20],
        }],
        transforms: vec![TransformPlan {
            file_id: 10,
            owner: 1,
            parent: 0,
            children: Vec::new(),
            local_position: [-0.0, 1.5, 2.0],
            local_rotation: [0.0, 0.0, 0.0, 1.0],
            local_scale: [1.0, 1.0, 1.0],
        }],
        scripts: vec![ScriptPlan {
            file_id: 20,
            owner: 1,
            guid: "0123456789abcdef0123456789abcdef".to_owned(),
            assembly: "Example.Game".to_owned(),
            type_name: "Example.Behaviour".to_owned(),
            enabled: true,
            execution_order: -4,
            fields: vec![FieldPlan {
                name: "target".to_owned(),
                value: FieldValue::Reference(10),
            }],
        }],
    }
}

const BOUNDARY_FIELD_COUNT: usize = 16;
const EMPTY_PLAN_OUTPUT: &str = concat!(
    r#"{"schema_version":1,"scene_name":"","roots":[10],"game_objects":[{"file_id":1,"name":"Root","active":true,"components":[10,20]}],"transforms":["#,
    r#"{"file_id":10,"owner":1,"parent":0,"children":[],"local_position":[-0.0,1.5,2.0],"local_rotation":[0.0,0.0,0.0,1.0],"local_scale":[1.0,1.0,1.0]}],"scripts":["#,
    r#"{"file_id":20,"owner":1,"guid":"0123456789abcdef0123456789abcdef","assembly":"Example.Game","type_name":"Example.Behaviour","enabled":true,"execution_order":-4,"fields":[]}]}"#
);
const EMPTY_STRING_FIELD: &str = r#"{"name":"field-00","value":{"kind":"string","value":""}}"#;

fn exact_output_string_bytes() -> usize {
    let commas = BOUNDARY_FIELD_COUNT - 1;
    let empty_array_bytes = "[]".len();
    let plan_without_empty_field_array = EMPTY_PLAN_OUTPUT.len() - empty_array_bytes;
    let populated_field_array_bytes =
        empty_array_bytes + (BOUNDARY_FIELD_COUNT * EMPTY_STRING_FIELD.len()) + commas;
    let fixed_output_bytes = plan_without_empty_field_array + populated_field_array_bytes;
    let total_string_bytes = MAX_INPUT_BYTES - fixed_output_bytes;
    let full_string_bytes = (BOUNDARY_FIELD_COUNT - 1) * MAX_STRING_BYTES;
    let final_string_bytes = total_string_bytes - full_string_bytes;

    assert!(final_string_bytes <= MAX_STRING_BYTES);

    final_string_bytes
}

fn exact_output_plan() -> ScenePlan {
    let final_string_bytes = exact_output_string_bytes();
    let mut plan = valid_plan("");

    plan.scripts[0].fields = (0..BOUNDARY_FIELD_COUNT - 1)
        .map(|index| FieldPlan {
            name: format!("field-{index:02}"),
            value: FieldValue::String("x".repeat(MAX_STRING_BYTES)),
        })
        .collect();
    plan.scripts[0].fields.push(FieldPlan {
        name: format!("field-{:02}", BOUNDARY_FIELD_COUNT - 1),
        value: FieldValue::String("x".repeat(final_string_bytes)),
    });

    plan
}

#[test]
fn serialize_plan_emits_canonical_compact_bytes_and_preserves_valid_component_order() {
    let plan = valid_plan("Root \"quoted\"\\\né");
    let expected = concat!(
        r#"{"schema_version":1,"scene_name":"Root \"quoted\"\\\né","roots":[10],"game_objects":[{"file_id":1,"name":"Root","active":true,"components":[10,20]}],"transforms":["#,
        r#"{"file_id":10,"owner":1,"parent":0,"children":[],"local_position":[-0.0,1.5,2.0],"local_rotation":[0.0,0.0,0.0,1.0],"local_scale":[1.0,1.0,1.0]}],"scripts":["#,
        r#"{"file_id":20,"owner":1,"guid":"0123456789abcdef0123456789abcdef","assembly":"Example.Game","type_name":"Example.Behaviour","enabled":true,"execution_order":-4,"fields":[{"name":"target","value":{"kind":"reference","value":10}}]}]}"#
    );
    assert_eq!(serialize_plan(&plan).unwrap(), expected.as_bytes());
    assert_eq!(
        serialize_plan(&plan).unwrap(),
        serialize_plan(&plan).unwrap()
    );
}

#[test]
fn serialize_and_parse_preserve_order_after_mandatory_transform() {
    let mut plan = valid_plan("Component order");
    plan.scripts.push(ScriptPlan {
        file_id: 30,
        owner: 1,
        guid: "11111111111111111111111111111111".to_owned(),
        assembly: "Example.Game".to_owned(),
        type_name: "Example.SecondBehaviour".to_owned(),
        enabled: true,
        execution_order: -3,
        fields: Vec::new(),
    });
    plan.game_objects[0].components = vec![10, 30, 20];

    let bytes = serialize_plan(&plan).unwrap();
    let parsed = parse_plan(&bytes).unwrap();

    assert_eq!(&parsed.game_objects[0].components, &[10, 30, 20]);
}

#[test]
fn serialize_and_parse_reject_transform_after_another_component() {
    let mut reversed = valid_plan("Transform order");
    reversed.game_objects[0].components = vec![20, 10];

    assert!(
        serialize_plan(&reversed)
            .unwrap_err()
            .to_string()
            .contains("mandatory Transform must be first component")
    );

    let valid_bytes = serialize_plan(&valid_plan("Transform order")).unwrap();
    let valid_json = std::str::from_utf8(&valid_bytes).unwrap();
    let reversed_json = valid_json.replace(r#""components":[10,20]"#, r#""components":[20,10]"#);
    assert_ne!(reversed_json, valid_json);

    assert!(
        parse_plan(reversed_json.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("mandatory Transform must be first component")
    );
}

#[test]
fn serialize_plan_round_trips_through_strict_parse() {
    let plan = valid_plan("Round trip");
    let bytes = serialize_plan(&plan).unwrap();
    let parsed = parse_plan(&bytes).unwrap();

    assert_eq!(parsed.schema_version, plan.schema_version);
    assert_eq!(parsed.scene_name, plan.scene_name);
    assert_eq!(parsed.roots, plan.roots);
    assert_eq!(parsed.game_objects[0].file_id, plan.game_objects[0].file_id);
    assert_eq!(parsed.game_objects[0].name, plan.game_objects[0].name);
    assert_eq!(parsed.game_objects[0].active, plan.game_objects[0].active);
    assert_eq!(
        parsed.game_objects[0].components,
        plan.game_objects[0].components
    );
    assert_eq!(parsed.transforms[0].file_id, plan.transforms[0].file_id);
    assert_eq!(parsed.transforms[0].owner, plan.transforms[0].owner);
    assert_eq!(parsed.transforms[0].parent, plan.transforms[0].parent);
    assert_eq!(parsed.transforms[0].children, plan.transforms[0].children);
    assert_eq!(
        parsed.transforms[0].local_position,
        plan.transforms[0].local_position
    );
    assert_eq!(
        parsed.transforms[0].local_rotation,
        plan.transforms[0].local_rotation
    );
    assert_eq!(
        parsed.transforms[0].local_scale,
        plan.transforms[0].local_scale
    );
    assert_eq!(parsed.scripts[0].guid, plan.scripts[0].guid);
    assert_eq!(parsed.scripts[0].assembly, plan.scripts[0].assembly);
    assert_eq!(parsed.scripts[0].type_name, plan.scripts[0].type_name);
    assert_eq!(parsed.scripts[0].enabled, plan.scripts[0].enabled);
    assert_eq!(
        parsed.scripts[0].execution_order,
        plan.scripts[0].execution_order
    );
    assert_eq!(
        parsed.scripts[0].fields[0].name,
        plan.scripts[0].fields[0].name
    );
    match (
        &parsed.scripts[0].fields[0].value,
        &plan.scripts[0].fields[0].value,
    ) {
        (FieldValue::Reference(parsed), FieldValue::Reference(expected)) => {
            assert_eq!(parsed, expected);
        }
        _ => panic!("round trip changed the typed field kind"),
    }
}

#[test]
fn serialize_plan_preserves_finite_values_and_negative_zero() {
    let mut plan = valid_plan("Numbers");
    plan.transforms[0].local_position[2] = -2.75;
    let parsed = parse_plan(&serialize_plan(&plan).unwrap()).unwrap();
    assert_eq!(
        parsed.transforms[0].local_position[0].to_bits(),
        (-0.0f32).to_bits()
    );
    assert_eq!(parsed.transforms[0].local_position[1], 1.5);
    assert_eq!(
        parsed.transforms[0].local_position[2].to_bits(),
        (-2.75f32).to_bits()
    );
    assert!(
        parsed.transforms[0]
            .local_position
            .iter()
            .all(|value| value.is_finite())
    );
}

#[test]
fn serialize_plan_validates_before_returning_output() {
    let mut invalid = valid_plan("Invalid");
    invalid.transforms[0].local_position[0] = f32::NAN;
    assert!(serialize_plan(&invalid).is_err());
    assert!(invalid.transforms[0].local_position[0].is_nan());
}

#[test]
fn serialize_plan_rejects_unresolved_references_and_duplicate_identity() {
    let mut unresolved_reference = valid_plan("Unresolved reference");
    unresolved_reference.scripts[0].fields[0].value = FieldValue::Reference(999);
    assert!(
        serialize_plan(&unresolved_reference)
            .unwrap_err()
            .to_string()
            .contains("unresolved field reference")
    );

    let mut duplicate_identity = valid_plan("Duplicate identity");
    duplicate_identity.transforms[0].file_id = duplicate_identity.game_objects[0].file_id;
    assert!(
        serialize_plan(&duplicate_identity)
            .unwrap_err()
            .to_string()
            .contains("invalid or duplicate fileID")
    );
}

#[test]
fn serialize_plan_rejects_schema_graph_guid_string_and_field_limit_violations() {
    let mut unsupported_schema = valid_plan("Unsupported schema");
    unsupported_schema.schema_version = 2;
    assert!(
        serialize_plan(&unsupported_schema)
            .unwrap_err()
            .to_string()
            .contains("unsupported plan schema")
    );

    let mut unresolved_parent = valid_plan("Unresolved parent");
    unresolved_parent.transforms[0].parent = 999;
    assert!(
        serialize_plan(&unresolved_parent)
            .unwrap_err()
            .to_string()
            .contains("unresolved parent")
    );

    let mut invalid_guid = valid_plan("Invalid GUID");
    invalid_guid.scripts[0].guid = "not-a-guid".to_owned();
    assert!(serialize_plan(&invalid_guid).is_err());

    let mut oversized_string = valid_plan("Oversized string");
    oversized_string.scene_name = "x".repeat(MAX_STRING_BYTES + 1);
    assert!(serialize_plan(&oversized_string).is_err());

    let mut excessive_fields = valid_plan("Too many fields");
    excessive_fields.scripts[0].fields = (0..=MAX_FIELDS)
        .map(|index| FieldPlan {
            name: format!("field-{index}"),
            value: FieldValue::Reference(0),
        })
        .collect();
    assert!(serialize_plan(&excessive_fields).is_err());
}

#[test]
fn serialize_plan_accepts_exact_limit_and_rejects_over_limit_without_partial_output() {
    let mut empty_plan = valid_plan("");
    empty_plan.scripts[0].fields.clear();
    assert_eq!(
        serialize_plan(&empty_plan).unwrap(),
        EMPTY_PLAN_OUTPUT.as_bytes()
    );

    let exact = exact_output_plan();
    let exact_bytes = serialize_plan(&exact).unwrap();
    assert_eq!(exact_bytes.len(), MAX_INPUT_BYTES);

    let mut over_limit = exact.clone();
    let last_field = over_limit.scripts[0].fields.last_mut().unwrap();
    if let FieldValue::String(value) = &mut last_field.value {
        value.push('x');
    } else {
        panic!("boundary fixture must use a string field");
    }

    assert!(
        serialize_plan(&over_limit)
            .unwrap_err()
            .to_string()
            .contains("output byte limit")
    );
    assert_eq!(exact.scripts[0].fields.len(), BOUNDARY_FIELD_COUNT);
    match &over_limit.scripts[0].fields[BOUNDARY_FIELD_COUNT - 1].value {
        FieldValue::String(value) => {
            assert_eq!(value.len(), exact_output_string_bytes() + 1)
        }
        _ => panic!("boundary fixture must use a string field"),
    }
}
