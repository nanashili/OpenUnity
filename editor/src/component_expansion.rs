//! Independent, document-scoped presentation state for imported components.

use std::collections::{BTreeMap, BTreeSet};

use crate::inspector_lock::InspectorTarget;

/// One authored component section in the displayed target.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentSection {
    /// Canonical imported component file ID.
    pub file_id: i64,
    /// Committed presentation state, independent of component data.
    pub expanded: bool,
}

/// A complete committed view result, including aggregate availability.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentExpansionSnapshot {
    /// Explicit displayed Inspector identity.
    pub target: Option<InspectorTarget>,
    /// Supported sections in authored order.
    pub sections: Vec<ComponentSection>,
    /// Whether expanding would change a section.
    pub expand_all_enabled: bool,
    /// Whether collapsing would change a section.
    pub collapse_all_enabled: bool,
    /// Why no component operation is available.
    pub reason: Option<String>,
    /// Retained live section state across same-document target visits.
    pub cache: Vec<(InspectorTarget, ComponentSection)>,
}

impl ComponentExpansionSnapshot {
    /// Observable committed identities and states; no document values are copied.
    pub fn to_json(&self) -> serde_json::Value {
        let target = |t: InspectorTarget| serde_json::json!({"document_scope":t.document_scope,"entity_id":t.entity_id.0});
        serde_json::json!({
            "target":self.target.map(target),
            "sections":self.sections.iter().map(|s|serde_json::json!({"file_id":s.file_id,"expanded":s.expanded})).collect::<Vec<_>>(),
            "expand_all_enabled":self.expand_all_enabled,"collapse_all_enabled":self.collapse_all_enabled,"reason":self.reason,
            "cache":self.cache.iter().map(|(t,s)|serde_json::json!({"target":target(*t),"file_id":s.file_id,"expanded":s.expanded})).collect::<Vec<_>>()
        })
    }
}

type SectionKey = (u64, u64, i64);
type Observer = Box<dyn FnMut(&ComponentExpansionSnapshot)>;

/// Shared individual/bulk state. Reconciliation accepts already committed membership.
#[derive(Default)]
pub struct ComponentExpansionController {
    scope: Option<u64>,
    state: ComponentExpansionSnapshot,
    cache: BTreeMap<SectionKey, bool>,
    observers: Vec<Observer>,
}

impl ComponentExpansionController {
    /// Read the complete resulting state and availability together.
    pub fn snapshot(&self) -> &ComponentExpansionSnapshot {
        &self.state
    }

    /// Observe one notification per changed individual/bulk result, after commit.
    pub fn observe(&mut self, observer: impl FnMut(&ComponentExpansionSnapshot) + 'static) {
        self.observers.push(Box::new(observer));
    }

    /// Reconcile committed live membership, pruning removed identities before attachment.
    /// Each member's boolean is its existing view initialization, used only on attachment.
    pub fn reconcile(
        &mut self,
        scope: u64,
        target: Option<InspectorTarget>,
        live: &[(InspectorTarget, Vec<(i64, bool)>)],
    ) {
        if self.scope != Some(scope) {
            self.cache.clear();
            self.scope = Some(scope);
        }
        let live_keys: BTreeSet<_> = live
            .iter()
            .filter(|(t, _)| t.document_scope == scope)
            .flat_map(|(t, sections)| {
                sections
                    .iter()
                    .map(move |(id, _)| (scope, t.entity_id.0, *id))
            })
            .collect();
        self.cache.retain(|key, _| live_keys.contains(key));
        self.state.target = target
            .filter(|t| t.document_scope == scope && live.iter().any(|(member, _)| member == t));
        self.state.sections.clear();
        if let Some(target) = self.state.target
            && let Some((_, sections)) = live.iter().find(|(member, _)| *member == target)
        {
            for (id, initial) in sections {
                let expanded = *self
                    .cache
                    .entry((scope, target.entity_id.0, *id))
                    .or_insert(*initial);
                self.state.sections.push(ComponentSection {
                    file_id: *id,
                    expanded,
                });
            }
        }
        self.refresh();
    }

    fn refresh(&mut self) {
        self.state.expand_all_enabled = self.state.sections.iter().any(|s| !s.expanded);
        self.state.collapse_all_enabled = self.state.sections.iter().any(|s| s.expanded);
        self.state.reason = self
            .state
            .sections
            .is_empty()
            .then(|| "No supported component sections".into());
        self.state.cache = self
            .cache
            .iter()
            .map(|((scope, entity, id), expanded)| {
                (
                    InspectorTarget {
                        document_scope: *scope,
                        entity_id: openunity_editor_core::scene::EntityId(*entity),
                    },
                    ComponentSection {
                        file_id: *id,
                        expanded: *expanded,
                    },
                )
            })
            .collect();
    }

    fn commit(&mut self) {
        self.refresh();
        for observer in &mut self.observers {
            observer(&self.state);
        }
    }

    fn validate(&self, context: InspectorTarget) -> Result<(), String> {
        if self.state.target != Some(context) {
            return Err("Inspector component view changed".into());
        }
        Ok(())
    }

    /// Set only a currently eligible component; reject missing identities without attachment.
    pub fn set_component(
        &mut self,
        context: InspectorTarget,
        file_id: i64,
        expanded: bool,
    ) -> Result<(), String> {
        self.validate(context)?;
        let section = self
            .state
            .sections
            .iter_mut()
            .find(|s| s.file_id == file_id)
            .ok_or_else(|| "Inspector component no longer available".to_string())?;
        if section.expanded == expanded {
            return Ok(());
        }
        section.expanded = expanded;
        self.cache.insert(
            (context.document_scope, context.entity_id.0, file_id),
            expanded,
        );
        self.commit();
        Ok(())
    }

    /// Atomically set current live membership for the captured displayed identity.
    pub fn set_all(&mut self, context: InspectorTarget, expanded: bool) -> Result<(), String> {
        self.validate(context)?;
        let mut changed = false;
        for section in &mut self.state.sections {
            changed |= section.expanded != expanded;
            section.expanded = expanded;
            self.cache.insert(
                (context.document_scope, context.entity_id.0, section.file_id),
                expanded,
            );
        }
        if changed {
            self.commit();
        }
        Ok(())
    }
}
