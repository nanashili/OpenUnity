use crate::{
    FieldPlan, FieldValue, GameObjectPlan, MAX_DEPTH, MAX_FIELDS, MAX_INPUT_BYTES, MAX_OBJECTS,
    Result, SceneError, ScenePlan, ScriptPlan, TransformPlan, json, text,
    yaml::{self, Node},
};
use openunity_assets::{Guid, MetadataIndex};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    schema_version: u32,
    scripts: Vec<Binding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    guid: String,
    assembly: String,
    #[serde(rename = "type")]
    type_name: String,
    execution_order: i32,
    fields: Vec<FieldBinding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldBinding {
    name: String,
    kind: String,
}

fn confined(root: &Path, path: &Path) -> Result<PathBuf> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) || path.as_os_str().is_empty() {
        return Err(SceneError::new("invalid project path"));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    };
    if !absolute.starts_with(root) {
        return Err(SceneError::new("path outside project root"));
    }
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        checked.push(component.as_os_str());
        if fs::symlink_metadata(&checked)
            .map_err(SceneError::new)?
            .file_type()
            .is_symlink()
        {
            return Err(SceneError::new("symbolic project path"));
        }
    }
    Ok(absolute)
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let prior = fs::symlink_metadata(path).map_err(SceneError::new)?;
    if !prior.is_file() || prior.len() > MAX_INPUT_BYTES as u64 {
        return Err(SceneError::new("nonregular or oversized input"));
    }
    let file = File::open(path).map_err(SceneError::new)?;
    let m = file.metadata().map_err(SceneError::new)?;
    if !m.is_file() || m.len() > MAX_INPUT_BYTES as u64 {
        return Err(SceneError::new("nonregular or oversized input"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(SceneError::new)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(SceneError::new("input grew beyond byte limit"));
    }
    Ok(bytes)
}
fn index(
    root: &Path,
    dir: &Path,
    depth: usize,
    count: &mut usize,
    index: &mut MetadataIndex,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(SceneError::new("project directory depth limit"));
    }
    for entry in fs::read_dir(dir).map_err(SceneError::new)? {
        let entry = entry.map_err(SceneError::new)?;
        *count += 1;
        if *count > MAX_OBJECTS {
            return Err(SceneError::new("project entry count limit"));
        }
        let path = confined(root, &entry.path())?;
        let kind = entry.file_type().map_err(SceneError::new)?;
        if kind.is_dir() {
            index_recurse(root, &path, depth + 1, count, index)?;
        } else if !kind.is_file() {
            return Err(SceneError::new("unsupported project file type"));
        } else if path.extension().is_some_and(|e| e == "meta") {
            let meta = openunity_assets::read(&path).map_err(SceneError::new)?;
            let asset = path.with_extension("");
            confined(root, &asset)?;
            index.insert(asset, meta).map_err(SceneError::new)?;
        }
    }
    Ok(())
}
// Separate name avoids shadowing the recursive function with the index value.
fn index_recurse(
    root: &Path,
    dir: &Path,
    depth: usize,
    count: &mut usize,
    entries: &mut MetadataIndex,
) -> Result<()> {
    index(root, dir, depth, count, entries)
}
fn get<'a>(map: &'a BTreeMap<String, Node>, key: &str) -> Result<&'a Node> {
    map.get(key)
        .ok_or_else(|| SceneError::new(format!("missing key {key}")))
}
fn keys(map: &BTreeMap<String, Node>, expected: &[&str]) -> Result<()> {
    if map.len() != expected.len() || expected.iter().any(|k| !map.contains_key(*k)) {
        return Err(SceneError::new("unsupported or missing document keys"));
    }
    Ok(())
}
fn integer(node: &Node) -> Result<i64> {
    if matches!(node, Node::Quoted(_)) {
        return Err(SceneError::new("quoted numeric scalar unsupported"));
    }
    let s = node.scalar()?;
    if s.is_empty()
        || !s
            .strip_prefix('-')
            .unwrap_or(s)
            .bytes()
            .all(|b| b.is_ascii_digit())
    {
        return Err(SceneError::new("unsupported integer scalar"));
    }
    s.parse().map_err(SceneError::new)
}
fn float(node: &Node) -> Result<f32> {
    if matches!(node, Node::Quoted(_)) {
        return Err(SceneError::new("quoted numeric scalar unsupported"));
    }
    let s = node.scalar()?;
    let v = s.parse::<f32>().map_err(SceneError::new)?;
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b'.')
        || !v.is_finite()
    {
        return Err(SceneError::new("unsupported finite float32 scalar"));
    }
    Ok(v)
}
fn flag(node: &Node) -> Result<bool> {
    match integer(node)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::new("boolean must be 0 or 1")),
    }
}
fn constant(map: &BTreeMap<String, Node>, key: &str, value: i64) -> Result<()> {
    if integer(get(map, key)?)? != value {
        return Err(SceneError::new(format!("unsupported {key}")));
    }
    Ok(())
}
fn reference(node: &Node) -> Result<i64> {
    let m = node.map()?;
    keys(m, &["fileID"])?;
    let id = integer(get(m, "fileID")?)?;
    if id < 0 {
        return Err(SceneError::new("negative reference"));
    }
    Ok(id)
}
fn references(node: &Node) -> Result<Vec<i64>> {
    node.seq()?.iter().map(reference).collect()
}
fn vector<const N: usize>(node: &Node) -> Result<[f32; N]> {
    let map = node.map()?;
    let names = &["x", "y", "z", "w"][..N];
    keys(map, names)?;
    let mut out = [0.0; N];
    for (i, key) in names.iter().enumerate() {
        out[i] = float(get(map, key)?)?;
    }
    Ok(out)
}
fn common(map: &BTreeMap<String, Node>) -> Result<()> {
    constant(map, "m_ObjectHideFlags", 0)?;
    for key in [
        "m_CorrespondingSourceObject",
        "m_PrefabInstance",
        "m_PrefabAsset",
    ] {
        if reference(get(map, key)?)? != 0 {
            return Err(SceneError::new("prefab/external reference unsupported"));
        }
    }
    Ok(())
}
fn settings_equal(node: &Node, expected: &Value) -> Result<bool> {
    Ok(match expected {
        Value::Object(m) => {
            let actual = node.map()?;
            actual.len() == m.len()
                && m.iter().all(|(k, v)| {
                    actual
                        .get(k)
                        .is_some_and(|a| settings_equal(a, v).unwrap_or(false))
                })
        }
        Value::String(v) => node.scalar()? == v,
        Value::Number(v) => {
            if matches!(node, Node::Quoted(_)) {
                return Ok(false);
            }
            let raw = node.scalar()?;
            if raw.is_empty()
                || !raw
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b == b'-' || b == b'.')
            {
                return Ok(false);
            }
            let n = raw.parse::<f64>().map_err(SceneError::new)?;
            n.is_finite() && v.as_f64() == Some(n)
        }
        _ => return Err(SceneError::new("unsupported settings schema")),
    })
}
fn value(node: &Node, kind: &str) -> Result<FieldValue> {
    Ok(match kind {
        "i32" => FieldValue::I32(integer(node)?.try_into().map_err(SceneError::new)?),
        "f32" => FieldValue::F32(float(node)?),
        "bool" => FieldValue::Bool(flag(node)?),
        "string" => FieldValue::String(node.scalar()?.to_owned()),
        "vec3" => FieldValue::Vec3(vector(node)?),
        "quat" => FieldValue::Quat(vector(node)?),
        "reference" => FieldValue::Reference(reference(node)?),
        _ => return Err(SceneError::new("unsupported binding field kind")),
    })
}

/// Read actual scene/meta files and an explicit type/field binding manifest.
/// Paths must be within an absolute nonsymlink project root. No file is modified.
/// Only the reviewed authored grammar and exact skipped settings are supported.
pub fn import_project(root: &Path, scene: &Path, bindings: &Path) -> Result<ScenePlan> {
    if !root.is_absolute()
        || root
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || !root.is_dir()
    {
        return Err(SceneError::new("expected absolute project directory"));
    }
    let root = confined(root, root)?;
    let scene = confined(&root, scene)?;
    let binding_path = confined(&root, bindings)?;
    let mut metadata = MetadataIndex::new();
    index(&root, &root, 0, &mut 0, &mut metadata)?;
    let scene_meta = PathBuf::from(format!(
        "{}.meta",
        scene
            .to_str()
            .ok_or_else(|| SceneError::new("nonUnicode scene path"))?
    ));
    openunity_assets::read(&scene_meta).map_err(SceneError::new)?;
    let bindings: Bindings =
        serde_json::from_value(json::parse(&read(&binding_path)?)?).map_err(SceneError::new)?;
    if bindings.schema_version != 1
        || bindings.scripts.is_empty()
        || bindings.scripts.len() > MAX_OBJECTS
    {
        return Err(SceneError::new("unsupported binding manifest"));
    }
    let mut scripts = BTreeMap::new();
    let mut types = BTreeSet::new();
    for binding in bindings.scripts {
        text(&binding.assembly)?;
        text(&binding.type_name)?;
        let guid = binding.guid.parse::<Guid>().map_err(SceneError::new)?;
        let script_asset = metadata
            .asset_path(guid)
            .ok_or_else(|| SceneError::new("unresolved script GUID"))?;
        if !script_asset.is_file()
            || script_asset.extension().is_none_or(|e| e != "cs")
            || binding.guid != guid.to_string()
            || binding.assembly != "Assembly-CSharp"
            || binding.type_name.is_empty()
            || binding.fields.len() > MAX_FIELDS
            || !types.insert((binding.assembly.clone(), binding.type_name.clone()))
        {
            return Err(SceneError::new("unresolved or ambiguous script binding"));
        }
        let mut fields = BTreeSet::new();
        for field in &binding.fields {
            text(&field.name)?;
            if field.name.is_empty()
                || !fields.insert(&field.name)
                || !["i32", "f32", "bool", "string", "vec3", "quat", "reference"]
                    .contains(&field.kind.as_str())
            {
                return Err(SceneError::new("invalid binding field"));
            }
        }
        if scripts.insert(guid, binding).is_some() {
            return Err(SceneError::new("duplicate binding GUID"));
        }
    }
    let settings: Value = json::parse(include_bytes!("settings-defaults.json"))?;
    let mut ids = BTreeSet::new();
    let mut settings_seen = BTreeSet::new();
    let mut root_seen = false;
    let mut plan = ScenePlan {
        schema_version: 1,
        scene_name: scene
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SceneError::new("nonUnicode scene name"))?
            .to_owned(),
        roots: Vec::new(),
        game_objects: Vec::new(),
        transforms: Vec::new(),
        scripts: Vec::new(),
    };
    for doc in yaml::parse(&read(&scene)?)? {
        if !ids.insert(doc.file_id) {
            return Err(SceneError::new("duplicate document fileID"));
        }
        let top = doc.node.map()?;
        if let Some(expected) = settings.get(doc.class_id.to_string()) {
            if !settings_seen.insert(doc.class_id) || !settings_equal(&doc.node, expected)? {
                return Err(SceneError::new("unsupported or duplicated settings record"));
            }
            continue;
        }
        let kind = match doc.class_id {
            1 => "GameObject",
            4 => "Transform",
            114 => "MonoBehaviour",
            1660057539 => "SceneRoots",
            _ => return Err(SceneError::new("unsupported scene classID")),
        };
        keys(top, &[kind])?;
        let m = get(top, kind)?.map()?;
        match doc.class_id {
            1 => {
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
                if get(m, "m_TagString")?.scalar()? != "Untagged"
                    || reference(get(m, "m_Icon")?)? != 0
                {
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
                plan.game_objects.push(GameObjectPlan {
                    file_id: doc.file_id,
                    name: get(m, "m_Name")?.scalar()?.to_owned(),
                    active: flag(get(m, "m_IsActive")?)?,
                    components,
                });
            }
            4 => {
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
                plan.transforms.push(TransformPlan {
                    file_id: doc.file_id,
                    owner: reference(get(m, "m_GameObject")?)?,
                    parent: reference(get(m, "m_Father")?)?,
                    children: references(get(m, "m_Children")?)?,
                    local_position: vector(get(m, "m_LocalPosition")?)?,
                    local_rotation: vector(get(m, "m_LocalRotation")?)?,
                    local_scale: vector(get(m, "m_LocalScale")?)?,
                });
            }
            114 => {
                common(m)?;
                constant(m, "m_EditorHideFlags", 0)?;
                if !get(m, "m_Name")?.scalar()?.is_empty() {
                    return Err(SceneError::new("unsupported component name"));
                }
                let script = get(m, "m_Script")?.map()?;
                keys(script, &["fileID", "guid", "type"])?;
                constant(script, "fileID", 11500000)?;
                constant(script, "type", 3)?;
                let raw_guid = get(script, "guid")?.scalar()?;
                let guid = raw_guid.parse::<Guid>().map_err(SceneError::new)?;
                if raw_guid != guid.to_string() {
                    return Err(SceneError::new("noncanonical script GUID"));
                }
                let b = scripts
                    .get(&guid)
                    .ok_or_else(|| SceneError::new("missing script binding"))?;
                let mut expected = vec![
                    "m_ObjectHideFlags",
                    "m_CorrespondingSourceObject",
                    "m_PrefabInstance",
                    "m_PrefabAsset",
                    "m_GameObject",
                    "m_Enabled",
                    "m_EditorHideFlags",
                    "m_Script",
                    "m_Name",
                    "m_EditorClassIdentifier",
                ];
                expected.extend(b.fields.iter().map(|f| f.name.as_str()));
                keys(m, &expected)?;
                if get(m, "m_EditorClassIdentifier")?.scalar()?
                    != format!("{}::{}", b.assembly, b.type_name)
                {
                    return Err(SceneError::new("script identity disagreement"));
                }
                let fields = b
                    .fields
                    .iter()
                    .map(|f| {
                        Ok(FieldPlan {
                            name: f.name.clone(),
                            value: value(get(m, &f.name)?, &f.kind)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                plan.scripts.push(ScriptPlan {
                    file_id: doc.file_id,
                    owner: reference(get(m, "m_GameObject")?)?,
                    guid: guid.to_string(),
                    assembly: b.assembly.clone(),
                    type_name: b.type_name.clone(),
                    enabled: flag(get(m, "m_Enabled")?)?,
                    execution_order: b.execution_order,
                    fields,
                });
            }
            _ => {
                keys(m, &["m_ObjectHideFlags", "m_Roots"])?;
                constant(m, "m_ObjectHideFlags", 0)?;
                if root_seen {
                    return Err(SceneError::new("duplicate SceneRoots"));
                }
                root_seen = true;
                plan.roots = references(get(m, "m_Roots")?)?;
            }
        }
    }
    if !root_seen {
        return Err(SceneError::new("missing SceneRoots"));
    }
    plan.validate()?;
    Ok(plan)
}
