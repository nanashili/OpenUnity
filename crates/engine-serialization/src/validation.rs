use crate::{
    FieldValue, GameObjectPlan, MAX_DEPTH, MAX_FIELDS, MAX_OBJECTS, Result, SceneError, ScenePlan,
    TransformPlan, input::unique_ids, json, text,
};
use engine_assets::Guid;
use std::collections::{BTreeMap, BTreeSet};

impl ScenePlan {
    /// Validate bounded identities, ownership, ordering, references and numbers.
    /// Native Transform admissibility and managed reflection are separate checks.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(SceneError::new("unsupported plan schema"));
        }
        text(&self.scene_name)?;
        let count = self
            .game_objects
            .len()
            .checked_add(self.transforms.len())
            .and_then(|n| n.checked_add(self.scripts.len()))
            .ok_or_else(|| SceneError::new("object count overflow"))?;
        if count == 0 || count > MAX_OBJECTS || self.transforms.len() != self.game_objects.len() {
            return Err(SceneError::new(
                "invalid graph size or mandatory Transform count",
            ));
        }
        let all = unique_ids(
            self.game_objects
                .iter()
                .map(|o| o.file_id)
                .chain(self.transforms.iter().map(|o| o.file_id))
                .chain(self.scripts.iter().map(|o| o.file_id)),
        )?;
        let gos: BTreeMap<_, _> = self.game_objects.iter().map(|o| (o.file_id, o)).collect();
        let transforms: BTreeMap<_, _> = self.transforms.iter().map(|o| (o.file_id, o)).collect();
        let mut owner = BTreeMap::new();
        self.validate_transforms(&gos, &transforms, &mut owner)?;
        self.validate_roots()?;
        self.validate_scripts(&all, &gos, &mut owner)?;
        self.validate_components(&transforms, &owner)
    }

    fn validate_transforms(
        &self,
        gos: &BTreeMap<i64, &GameObjectPlan>,
        transforms: &BTreeMap<i64, &TransformPlan>,
        owner: &mut BTreeMap<i64, i64>,
    ) -> Result<()> {
        for tr in &self.transforms {
            if !gos.contains_key(&tr.owner) {
                return Err(SceneError::new("unresolved Transform owner"));
            }
            owner.insert(tr.file_id, tr.owner);
            if !tr
                .local_position
                .iter()
                .chain(&tr.local_rotation)
                .chain(&tr.local_scale)
                .all(|f| f.is_finite())
            {
                return Err(SceneError::new("nonfinite Transform value"));
            }
            if tr.parent != 0 && !transforms.contains_key(&tr.parent) {
                return Err(SceneError::new("unresolved parent"));
            }
            unique_ids(tr.children.iter().copied())?;
            for child in &tr.children {
                if transforms.get(child).is_none_or(|c| c.parent != tr.file_id) {
                    return Err(SceneError::new("parent/child disagreement"));
                }
            }
            Self::validate_ancestry(tr.file_id, transforms)?;
        }
        Ok(())
    }

    fn validate_ancestry(start: i64, transforms: &BTreeMap<i64, &TransformPlan>) -> Result<()> {
        let mut ancestor = start;
        let mut visited = BTreeSet::new();

        while ancestor != 0 {
            if !visited.insert(ancestor) || visited.len() > MAX_DEPTH {
                return Err(SceneError::new("hierarchy cycle or depth limit"));
            }
            let node = transforms
                .get(&ancestor)
                .ok_or_else(|| SceneError::new("unresolved ancestor"))?;
            if node.parent != 0 {
                let parent = transforms
                    .get(&node.parent)
                    .ok_or_else(|| SceneError::new("unresolved parent"))?;
                if !parent.children.contains(&ancestor) {
                    return Err(SceneError::new("missing child link"));
                }
            }
            ancestor = node.parent;
        }
        Ok(())
    }

    fn validate_roots(&self) -> Result<()> {
        let roots = unique_ids(self.roots.iter().copied())?;
        let expected: BTreeSet<_> = self
            .transforms
            .iter()
            .filter(|t| t.parent == 0)
            .map(|t| t.file_id)
            .collect();
        if roots != expected {
            return Err(SceneError::new("root order membership mismatch"));
        }
        Ok(())
    }

    fn validate_scripts(
        &self,
        all: &BTreeSet<i64>,
        gos: &BTreeMap<i64, &GameObjectPlan>,
        owner: &mut BTreeMap<i64, i64>,
    ) -> Result<()> {
        for script in &self.scripts {
            let guid = script.guid.parse::<Guid>().map_err(SceneError::new)?;
            if guid.to_string() != script.guid {
                return Err(SceneError::new("script GUID must be canonical lower-case"));
            }
            text(&script.assembly)?;
            text(&script.type_name)?;
            if script.assembly.is_empty()
                || script.type_name.is_empty()
                || !gos.contains_key(&script.owner)
                || script.fields.len() > MAX_FIELDS
            {
                return Err(SceneError::new("invalid script binding/owner/fields"));
            }
            owner.insert(script.file_id, script.owner);
            let mut names = BTreeSet::new();
            for field in &script.fields {
                text(&field.name)?;
                if field.name.is_empty() || !names.insert(&field.name) {
                    return Err(SceneError::new("invalid or duplicate field"));
                }
                match &field.value {
                    FieldValue::F32(v) if !v.is_finite() => {
                        return Err(SceneError::new("nonfinite field"));
                    }
                    FieldValue::Vec3(v) if !v.iter().all(|v| v.is_finite()) => {
                        return Err(SceneError::new("nonfinite vector field"));
                    }
                    FieldValue::Quat(v) if !v.iter().all(|v| v.is_finite()) => {
                        return Err(SceneError::new("nonfinite quaternion field"));
                    }
                    FieldValue::String(v) => text(v)?,
                    FieldValue::Reference(id) if *id != 0 && !all.contains(id) => {
                        return Err(SceneError::new("unresolved field reference"));
                    }
                    _ => (),
                }
            }
        }
        Ok(())
    }

    fn validate_components(
        &self,
        transforms: &BTreeMap<i64, &TransformPlan>,
        owner: &BTreeMap<i64, i64>,
    ) -> Result<()> {
        let mut listed = BTreeSet::new();
        for go in &self.game_objects {
            text(&go.name)?;
            if go.components.is_empty() || go.components.len() > MAX_OBJECTS {
                return Err(SceneError::new("component list limit"));
            }
            let mut mandatory = 0;
            for component in &go.components {
                if owner.get(component) != Some(&go.file_id) || !listed.insert(*component) {
                    return Err(SceneError::new("component owner/list disagreement"));
                }
                mandatory += usize::from(transforms.contains_key(component));
            }
            if mandatory != 1 {
                return Err(SceneError::new("mandatory Transform missing/duplicated"));
            }
        }
        if listed.len() != owner.len() {
            return Err(SceneError::new("unlisted component"));
        }
        Ok(())
    }
}

/// Parse a strict bounded UTF-8 JSON plan, rejecting duplicate and unknown keys.
pub fn parse_plan(bytes: &[u8]) -> Result<ScenePlan> {
    let value = json::parse(bytes)?;
    let plan: ScenePlan = serde_json::from_value(value).map_err(SceneError::new)?;
    plan.validate()?;
    Ok(plan)
}
