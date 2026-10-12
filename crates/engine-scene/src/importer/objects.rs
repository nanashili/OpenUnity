use super::document_values::{common, constant, flag, get, keys, reference, references, vector};
use crate::{GameObjectPlan, Result, SceneError, TransformPlan, yaml::Node};
use std::collections::BTreeMap;

pub(super) fn game_object(file_id: i64, m: &BTreeMap<String, Node>) -> Result<GameObjectPlan> {
    keys(
        m,
        &[
            "m_ObjectHideFlags",
            "m_CorrespondingSourceObject",
            "m_PrefabInstance",
            "m_PrefabAsset",
            "serializedVersion",
            "m_Component",
            "m_Layer",
            "m_Name",
            "m_TagString",
            "m_Icon",
            "m_NavMeshLayer",
            "m_StaticEditorFlags",
            "m_IsActive",
        ],
    )?;
    common(m)?;
    constant(m, "serializedVersion", 6)?;
    for k in ["m_Layer", "m_NavMeshLayer", "m_StaticEditorFlags"] {
        constant(m, k, 0)?;
    }
    if get(m, "m_TagString")?.scalar()? != "Untagged" || reference(get(m, "m_Icon")?)? != 0 {
        return Err(SceneError::new("unsupported GameObject metadata"));
    }
    let components = get(m, "m_Component")?
        .seq()?
        .iter()
        .map(|n| {
            let map = n.map()?;
            keys(map, &["component"])?;
            reference(get(map, "component")?)
        })
        .collect::<Result<_>>()?;
    Ok(GameObjectPlan {
        file_id,
        name: get(m, "m_Name")?.scalar()?.to_owned(),
        active: flag(get(m, "m_IsActive")?)?,
        components,
    })
}

pub(super) fn transform(file_id: i64, m: &BTreeMap<String, Node>) -> Result<TransformPlan> {
    keys(
        m,
        &[
            "m_ObjectHideFlags",
            "m_CorrespondingSourceObject",
            "m_PrefabInstance",
            "m_PrefabAsset",
            "m_GameObject",
            "serializedVersion",
            "m_LocalRotation",
            "m_LocalPosition",
            "m_LocalScale",
            "m_ConstrainProportionsScale",
            "m_Children",
            "m_Father",
            "m_LocalEulerAnglesHint",
        ],
    )?;
    common(m)?;
    constant(m, "serializedVersion", 2)?;
    constant(m, "m_ConstrainProportionsScale", 0)?;
    if vector::<3>(get(m, "m_LocalEulerAnglesHint")?)? != [0.; 3] {
        return Err(SceneError::new("unsupported Euler hint"));
    }
    Ok(TransformPlan {
        file_id,
        owner: reference(get(m, "m_GameObject")?)?,
        parent: reference(get(m, "m_Father")?)?,
        children: references(get(m, "m_Children")?)?,
        local_position: vector(get(m, "m_LocalPosition")?)?,
        local_rotation: vector(get(m, "m_LocalRotation")?)?,
        local_scale: vector(get(m, "m_LocalScale")?)?,
    })
}
