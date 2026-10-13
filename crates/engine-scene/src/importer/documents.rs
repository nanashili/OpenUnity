use super::{
    bindings::Binding,
    document_values::{constant, get, keys, references, settings_equal},
    objects, scripts,
};
use crate::{Result, SceneError, ScenePlan, yaml::Document};
use engine_assets::Guid;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(super) struct DocumentConversion<'a> {
    bindings: &'a BTreeMap<Guid, Binding>,
    render_documents: RenderDocumentHandling,
    settings: Value,
    ids: BTreeSet<i64>,
    render_component_ids: BTreeSet<i64>,
    settings_seen: BTreeSet<i64>,
    root_seen: bool,
    plan: ScenePlan,
}

#[derive(Clone, Copy)]
pub(super) enum RenderDocumentHandling {
    Reject,
    Extracted,
}

impl<'a> DocumentConversion<'a> {
    pub(super) fn new(
        scene: &Path,
        bindings: &'a BTreeMap<Guid, Binding>,
        settings: Value,
        render_documents: RenderDocumentHandling,
    ) -> Result<Self> {
        Ok(Self {
            bindings,
            render_documents,
            settings,
            ids: BTreeSet::new(),
            render_component_ids: BTreeSet::new(),
            settings_seen: BTreeSet::new(),
            root_seen: false,
            plan: ScenePlan {
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
            },
        })
    }

    pub(super) fn push(&mut self, doc: Document) -> Result<()> {
        if !self.ids.insert(doc.file_id) {
            return Err(SceneError::new("duplicate document fileID"));
        }
        let top = doc.node.map()?;
        if let Some(expected) = self.settings.get(doc.class_id.to_string()) {
            if !self.settings_seen.insert(doc.class_id) || !settings_equal(&doc.node, expected)? {
                return Err(SceneError::new("unsupported or duplicated settings record"));
            }
            return Ok(());
        }
        let kind = match doc.class_id {
            1 => "GameObject",
            4 => "Transform",
            114 => "MonoBehaviour",
            23 if matches!(self.render_documents, RenderDocumentHandling::Extracted) => {
                "MeshRenderer"
            }
            33 if matches!(self.render_documents, RenderDocumentHandling::Extracted) => {
                "MeshFilter"
            }
            1660057539 => "SceneRoots",
            _ => return Err(SceneError::new("unsupported scene classID")),
        };
        keys(top, &[kind])?;

        if matches!(doc.class_id, 23 | 33) {
            self.render_component_ids.insert(doc.file_id);
            return Ok(());
        }

        let m = get(top, kind)?.map()?;
        match doc.class_id {
            1 => self
                .plan
                .game_objects
                .push(objects::game_object(doc.file_id, m)?),
            4 => self
                .plan
                .transforms
                .push(objects::transform(doc.file_id, m)?),
            114 => self
                .plan
                .scripts
                .push(scripts::convert(doc.file_id, m, self.bindings)?),
            _ => {
                keys(m, &["m_ObjectHideFlags", "m_Roots"])?;
                constant(m, "m_ObjectHideFlags", 0)?;
                if self.root_seen {
                    return Err(SceneError::new("duplicate SceneRoots"));
                }
                self.root_seen = true;
                self.plan.roots = references(get(m, "m_Roots")?)?;
            }
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<ScenePlan> {
        if !self.root_seen {
            return Err(SceneError::new("missing SceneRoots"));
        }

        if matches!(self.render_documents, RenderDocumentHandling::Extracted) {
            self.remove_extracted_render_components();
        }

        self.plan.validate()?;
        Ok(self.plan)
    }

    fn remove_extracted_render_components(&mut self) {
        for game_object in &mut self.plan.game_objects {
            game_object
                .components
                .retain(|component_id| !self.render_component_ids.contains(component_id));
        }
    }
}
