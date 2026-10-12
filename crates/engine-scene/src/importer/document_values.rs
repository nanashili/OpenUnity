use crate::{FieldValue, Result, SceneError, yaml::Node};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn get<'a>(map: &'a BTreeMap<String, Node>, key: &str) -> Result<&'a Node> {
    map.get(key)
        .ok_or_else(|| SceneError::new(format!("missing key {key}")))
}
pub(super) fn keys(map: &BTreeMap<String, Node>, expected: &[&str]) -> Result<()> {
    if map.len() != expected.len() || expected.iter().any(|k| !map.contains_key(*k)) {
        return Err(SceneError::new("unsupported or missing document keys"));
    }
    Ok(())
}
pub(super) fn integer(node: &Node) -> Result<i64> {
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
pub(super) fn float(node: &Node) -> Result<f32> {
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
pub(super) fn flag(node: &Node) -> Result<bool> {
    match integer(node)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::new("boolean must be 0 or 1")),
    }
}
pub(super) fn constant(map: &BTreeMap<String, Node>, key: &str, value: i64) -> Result<()> {
    if integer(get(map, key)?)? != value {
        return Err(SceneError::new(format!("unsupported {key}")));
    }
    Ok(())
}
pub(super) fn reference(node: &Node) -> Result<i64> {
    let m = node.map()?;
    keys(m, &["fileID"])?;
    let id = integer(get(m, "fileID")?)?;
    if id < 0 {
        return Err(SceneError::new("negative reference"));
    }
    Ok(id)
}
pub(super) fn references(node: &Node) -> Result<Vec<i64>> {
    node.seq()?.iter().map(reference).collect()
}
pub(super) fn vector<const N: usize>(node: &Node) -> Result<[f32; N]> {
    let map = node.map()?;
    let names = &["x", "y", "z", "w"][..N];
    keys(map, names)?;
    let mut out = [0.0; N];
    for (i, key) in names.iter().enumerate() {
        out[i] = float(get(map, key)?)?;
    }
    Ok(out)
}
pub(super) fn common(map: &BTreeMap<String, Node>) -> Result<()> {
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
pub(super) fn settings_equal(node: &Node, expected: &Value) -> Result<bool> {
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
pub(super) fn value(node: &Node, kind: &str) -> Result<FieldValue> {
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
