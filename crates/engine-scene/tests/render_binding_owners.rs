//! Public integration coverage for scene-owned render binding validation.

use engine_scene::{
    GameObjectPlan, ScenePlan, ScriptPlan, TransformPlan, validate_render_binding_owners,
};

#[test]
fn accepts_known_owners_and_camera_draw_co_ownership() {
    let plan = valid_plan();

    let result = validate_render_binding_owners(&plan, 10, &[10, 30]);

    assert!(result.is_ok());
}

#[test]
fn accepts_a_valid_camera_without_draws() {
    let plan = valid_plan();

    assert!(validate_render_binding_owners(&plan, 10, &[]).is_ok());
}

#[test]
fn rejects_nonpositive_and_unknown_camera_owners() {
    let plan = valid_plan();

    for owner in [0, -1, 999] {
        let error = validate_render_binding_owners(&plan, owner, &[])
            .expect_err("camera owner must identify a positive GameObject");

        assert!(error.to_string().contains("camera owner"));
    }
}

#[test]
fn rejects_nonpositive_unknown_and_transform_draw_owners() {
    let plan = valid_plan();

    for owner in [0, -1, 999, 20] {
        let error = validate_render_binding_owners(&plan, 10, &[owner])
            .expect_err("draw owner must identify a positive GameObject");

        assert!(error.to_string().contains("draw owner"));
    }
}

#[test]
fn rejects_transform_and_script_ids_as_gameobject_owners() {
    let plan = plan_with_script_component();

    let transform_error = validate_render_binding_owners(&plan, 20, &[])
        .expect_err("Transform ID is not a GameObject ID");
    let script_error = validate_render_binding_owners(&plan, 10, &[50])
        .expect_err("script ID is not a draw GameObject ID");

    assert!(transform_error.to_string().contains("not a GameObject"));
    assert!(script_error.to_string().contains("not a GameObject"));
}

#[test]
fn rejects_invalid_scene_plan_before_checking_render_owners() {
    let mut plan = valid_plan();
    plan.schema_version = 2;

    let error = validate_render_binding_owners(&plan, 0, &[])
        .expect_err("invalid ScenePlan must fail before owner validation");

    assert!(error.to_string().contains("unsupported plan schema"));
}

#[test]
fn rejects_duplicate_draw_owners() {
    let plan = valid_plan();

    let error = validate_render_binding_owners(&plan, 10, &[30, 30])
        .expect_err("one GameObject cannot own multiple render draws");

    assert!(error.to_string().contains("multiple render draws"));
}

fn valid_plan() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: "render-binding-test".to_owned(),
        roots: vec![20, 40],
        game_objects: vec![game_object(10, vec![20]), game_object(30, vec![40])],
        transforms: vec![transform(20, 10), transform(40, 30)],
        scripts: Vec::new(),
    }
}

fn plan_with_script_component() -> ScenePlan {
    let mut plan = valid_plan();
    plan.game_objects[0].components.push(50);
    plan.scripts.push(ScriptPlan {
        file_id: 50,
        owner: 10,
        guid: "0123456789abcdef0123456789abcdef".to_owned(),
        assembly: "Assembly-CSharp".to_owned(),
        type_name: "Fixture.Driver".to_owned(),
        enabled: true,
        execution_order: 0,
        fields: Vec::new(),
    });
    plan
}

fn game_object(file_id: i64, components: Vec<i64>) -> GameObjectPlan {
    GameObjectPlan {
        file_id,
        name: format!("object-{file_id}"),
        active: true,
        components,
    }
}

fn transform(file_id: i64, owner: i64) -> TransformPlan {
    TransformPlan {
        file_id,
        owner,
        parent: 0,
        children: Vec::new(),
        local_position: [0.0; 3],
        local_rotation: [0.0, 0.0, 0.0, 1.0],
        local_scale: [1.0; 3],
    }
}
