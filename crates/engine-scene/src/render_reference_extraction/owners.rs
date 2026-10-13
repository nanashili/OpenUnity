use crate::{Result, SceneError, yaml::Document, yaml::Node};
use std::collections::BTreeMap;

const GAME_OBJECT_CLASS_ID: i64 = 1;

pub(super) fn owner_by_component(documents: &[Document]) -> Result<BTreeMap<i64, i64>> {
    let document_classes = unique_document_classes(documents)?;
    let mut owners = BTreeMap::new();

    for document in documents {
        if document.class_id == GAME_OBJECT_CLASS_ID {
            collect_game_object_memberships(document, &mut owners, &document_classes)?;
        }
    }

    validate_membership_targets(&owners, &document_classes)?;

    Ok(owners)
}

pub(super) fn parse_local_owner(node: &Node, context: &str) -> Result<i64> {
    let pointer = match node {
        Node::Map(pointer) => pointer,
        _ => {
            return Err(SceneError::new(format!(
                "render-reference extraction: {context} must be a one-key local pointer map"
            )));
        }
    };

    if pointer.len() != 1 || !pointer.contains_key("fileID") {
        return Err(SceneError::new(format!(
            "render-reference extraction: {context} must contain only fileID"
        )));
    }

    let file_id = pointer.get("fileID").ok_or_else(|| {
        SceneError::new(format!(
            "render-reference extraction: {context} is missing fileID"
        ))
    })?;

    positive_file_id(file_id, context)
}

fn unique_document_classes(documents: &[Document]) -> Result<BTreeMap<i64, i64>> {
    let mut classes = BTreeMap::new();

    for document in documents {
        if document.file_id <= 0 {
            return Err(SceneError::new(format!(
                "render-reference extraction: document fileID {} must be positive",
                document.file_id
            )));
        }

        if classes
            .insert(document.file_id, document.class_id)
            .is_some()
        {
            return Err(SceneError::new(format!(
                "render-reference extraction: duplicate document fileID {}",
                document.file_id
            )));
        }
    }

    Ok(classes)
}

fn collect_game_object_memberships(
    document: &Document,
    owners: &mut BTreeMap<i64, i64>,
    document_classes: &BTreeMap<i64, i64>,
) -> Result<()> {
    let context = format!("GameObject document {}", document.file_id);
    let game_object = top_mapping(&document.node, "GameObject", &context)?;
    let entries = required_sequence(game_object, "m_Component", &context)?;

    for (index, entry) in entries.iter().enumerate() {
        let component_id = component_entry_file_id(entry, document.file_id, index)?;

        if let Some(previous_owner_id) = owners.get(&component_id) {
            let class_context = document_classes
                .get(&component_id)
                .map(|class_id| format!("classID {class_id} "))
                .unwrap_or_default();

            if *previous_owner_id == document.file_id {
                return Err(SceneError::new(format!(
                    "render-reference extraction: GameObject document {} lists {class_context}component document {component_id} more than once",
                    document.file_id
                )));
            }

            return Err(SceneError::new(format!(
                "render-reference extraction: {class_context}component document {component_id} is listed by GameObject documents {previous_owner_id} and {}",
                document.file_id
            )));
        }

        owners.insert(component_id, document.file_id);
    }

    Ok(())
}

fn component_entry_file_id(entry: &Node, owner_id: i64, index: usize) -> Result<i64> {
    let entry_map = match entry {
        Node::Map(entry_map) => entry_map,
        _ => {
            return Err(SceneError::new(format!(
                "render-reference extraction: GameObject document {owner_id} m_Component entry {index} must be a component mapping"
            )));
        }
    };

    if entry_map.len() != 1 || !entry_map.contains_key("component") {
        return Err(SceneError::new(format!(
            "render-reference extraction: GameObject document {owner_id} m_Component entry {index} must contain only component"
        )));
    }

    let pointer = entry_map.get("component").ok_or_else(|| {
        SceneError::new(format!(
            "render-reference extraction: GameObject document {owner_id} m_Component entry {index} is missing component"
        ))
    })?;
    let context = format!("GameObject document {owner_id} m_Component entry {index} component");

    parse_local_owner(pointer, &context)
}

fn positive_file_id(node: &Node, context: &str) -> Result<i64> {
    let raw = match node {
        Node::Scalar(value) => value,
        Node::Quoted(_) => {
            return Err(SceneError::new(format!(
                "render-reference extraction: {context}.fileID must be an unquoted decimal scalar"
            )));
        }
        _ => {
            return Err(SceneError::new(format!(
                "render-reference extraction: {context}.fileID must be a scalar"
            )));
        }
    };

    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(SceneError::new(format!(
            "render-reference extraction: {context}.fileID must be a decimal integer"
        )));
    }

    let value = raw.parse::<i64>().map_err(|_| {
        SceneError::new(format!(
            "render-reference extraction: {context}.fileID is outside the i64 range"
        ))
    })?;

    if value <= 0 {
        return Err(SceneError::new(format!(
            "render-reference extraction: {context}.fileID must be positive"
        )));
    }

    Ok(value)
}

fn top_mapping<'a>(node: &'a Node, key: &str, context: &str) -> Result<&'a BTreeMap<String, Node>> {
    let top = match node {
        Node::Map(top) => top,
        _ => {
            return Err(SceneError::new(format!(
                "render-reference extraction: {context} requires a top-level {key} mapping"
            )));
        }
    };

    if top.len() != 1 || !top.contains_key(key) {
        return Err(SceneError::new(format!(
            "render-reference extraction: {context} requires only a top-level {key} mapping"
        )));
    }

    match top.get(key) {
        Some(Node::Map(fields)) => Ok(fields),
        _ => Err(SceneError::new(format!(
            "render-reference extraction: {context} {key} value must be a mapping"
        ))),
    }
}

fn required_sequence<'a>(
    fields: &'a BTreeMap<String, Node>,
    key: &str,
    context: &str,
) -> Result<&'a [Node]> {
    match fields.get(key) {
        None => Err(SceneError::new(format!(
            "render-reference extraction: {context} is missing {key}"
        ))),
        Some(Node::Seq(entries)) => Ok(entries),
        Some(_) => Err(SceneError::new(format!(
            "render-reference extraction: {context} {key} must be a sequence"
        ))),
    }
}

fn validate_membership_targets(
    owners: &BTreeMap<i64, i64>,
    document_classes: &BTreeMap<i64, i64>,
) -> Result<()> {
    for (component_id, owner_id) in owners {
        if !document_classes.contains_key(component_id) {
            return Err(SceneError::new(format!(
                "render-reference extraction: GameObject document {owner_id} lists missing component document {component_id}"
            )));
        }
    }

    Ok(())
}
