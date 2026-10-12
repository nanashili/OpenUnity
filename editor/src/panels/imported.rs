//! Direct typed views of an imported document; previews never become document state.

use super::hierarchy_browser::{HierarchyBrowser, Row};
use crate::appearance;
use eframe::egui::{
    self, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};
use openunity_editor_core::imported_scene::{
    FieldValue, ImportedComponent, ImportedDocument, ImportedEntityEdit, ImportedReferenceKind,
};
use openunity_editor_core::scene::EntityId;

/// Compact tree; structural controls remain unsupported for imported documents.
pub fn show_hierarchy_reference(
    ui: &mut Ui,
    document: &ImportedDocument,
    browser: &mut HierarchyBrowser,
) -> Vec<ImportedAction> {
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("+"))
            .on_disabled_hover_text(
                "Create, duplicate, delete and reparent are unsupported for imported documents.",
            );
        browser.search(ui);
    });
    let mut pending: Vec<_> = document
        .roots()
        .into_iter()
        .rev()
        .map(|id| (id, None, 0))
        .collect();
    let mut rows = Vec::new();
    while let Some((id, parent, depth)) = pending.pop() {
        match document.entity(id) {
            Ok(entity) => rows.push(Row {
                id,
                parent,
                name: entity.game_object.name.clone(),
                depth,
                active: entity.game_object.active,
                selected: document.selection().contains(&id),
            }),
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                continue;
            }
        }
        match document.children(id) {
            Ok(children) => pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, Some(id), depth + 1)),
            ),
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
            }
        }
    }
    browser
        .tree(ui, &document.plan().scene_name, &rows)
        .into_iter()
        .map(|(id, toggle)| ImportedAction::Select {
            id: Some(id),
            toggle,
        })
        .collect()
}

/// Requests validated and committed by the app's imported-document dispatcher.
pub enum ImportedAction {
    /// Reference presentation request; the app must attach the displayed scope.
    SetComponentExpanded {
        /// Canonical supported component file ID.
        file_id: i64,
        /// Requested presentation state, independent of serialized data.
        expanded: bool,
    },
    /// Select a mapped handle, toggle it, or clear the current selection.
    Select {
        /// Document-local handle to select, or `None` to clear selection.
        id: Option<EntityId>,
        /// Whether to toggle the handle instead of replacing selection.
        toggle: bool,
    },
    /// Apply one complete, transactional GameObject/local Transform edit.
    EditEntity {
        /// Document-local owner of the GameObject and Transform properties.
        id: EntityId,
        /// Complete candidate properties validated together before commit.
        edit: ImportedEntityEdit,
    },
    /// Change an existing script's serialized enabled flag transactionally.
    SetScriptEnabled {
        /// Serialized file ID identifying the existing script component.
        script_file_id: i64,
        /// Candidate serialized enabled flag.
        enabled: bool,
    },
}

/// Show roots and children in their authored order, using document-local handles.
pub fn show_hierarchy(ui: &mut Ui, document: &ImportedDocument) -> Vec<ImportedAction> {
    let mut actions = Vec::new();
    ui.small("Imported hierarchy · authored order");
    ui.horizontal_wrapped(|ui| {
        for label in ["Create", "Duplicate", "Delete", "Reparent / reorder"] {
            ui.add_enabled(false, egui::Button::new(label))
                .on_disabled_hover_text("Structural edits are unsupported for imported documents.");
        }
    });
    let mut pending: Vec<_> = document
        .roots()
        .into_iter()
        .rev()
        .map(|id| (id, 0))
        .collect();
    egui::ScrollArea::both()
        .id_salt("imported_hierarchy")
        .show(ui, |ui| {
            while let Some((id, depth)) = pending.pop() {
                let entity = match document.entity(id) {
                    Ok(entity) => entity,
                    Err(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                        continue;
                    }
                };
                ui.push_id(id.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * 14.0);
                        let response = ui
                            .selectable_label(
                                document.selection().contains(&id),
                                &entity.game_object.name,
                            )
                            .on_hover_text(format!(
                                "GameObject fileID {} · editor ID {}",
                                entity.game_object.file_id, id.0
                            ));
                        if response.clicked() {
                            actions.push(ImportedAction::Select {
                                id: Some(id),
                                toggle: selection_toggle(ui),
                            });
                        }
                        if !entity.game_object.active {
                            ui.weak("inactive");
                        }
                    });
                });
                match document.children(id) {
                    Ok(children) => {
                        pending.extend(children.into_iter().rev().map(|id| (id, depth + 1)))
                    }
                    Err(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                    }
                }
            }
        });
    actions
}

/// Uncommitted imported-property drafts, reset on replacement by the app.
#[derive(Default)]
pub struct InspectorPanel {
    selected: Vec<EntityId>,
    draft: Option<Draft>,
    controls: Vec<serde_json::Value>,
}

struct Draft {
    id: EntityId,
    original: ImportedEntityEdit,
    edit: ImportedEntityEdit,
}

fn same_edit(a: &ImportedEntityEdit, b: &ImportedEntityEdit) -> bool {
    a.name == b.name
        && a.active == b.active
        && a.local_position
            .iter()
            .chain(&a.local_rotation)
            .chain(&a.local_scale)
            .zip(
                b.local_position
                    .iter()
                    .chain(&b.local_rotation)
                    .chain(&b.local_scale),
            )
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn draft_error(edit: &ImportedEntityEdit) -> Option<&'static str> {
    if edit.name.len() > 1024 * 1024 || edit.name.contains('\0') {
        return Some("Name must contain no NUL and be at most 1 MiB of UTF-8.");
    }
    if edit
        .local_position
        .iter()
        .chain(&edit.local_rotation)
        .chain(&edit.local_scale)
        .any(|value| !value.is_finite())
    {
        return Some("All local Transform components must be finite.");
    }
    if edit.local_rotation.iter().all(|value| *value == 0.0) {
        return Some("The local Transform quaternion must be nonzero.");
    }
    None
}

impl InspectorPanel {
    /// Inspect complete ordered components and return only supported edit requests.
    pub fn show(&mut self, ui: &mut Ui, document: &ImportedDocument) -> Vec<ImportedAction> {
        self.show_impl(ui, document, None, None)
    }

    /// Inspect an explicitly scoped app target without following global selection.
    pub(crate) fn show_target(
        &mut self,
        ui: &mut Ui,
        document: &ImportedDocument,
        target: Option<EntityId>,
        expansion: &crate::component_expansion::ComponentExpansionSnapshot,
    ) -> Vec<ImportedAction> {
        self.show_impl(ui, document, Some(target), Some(expansion))
    }
    pub(crate) fn draft_state(&self) -> (Option<EntityId>, bool) {
        (
            self.draft.as_ref().map(|d| d.id),
            self.draft
                .as_ref()
                .is_some_and(|d| !same_edit(&d.edit, &d.original)),
        )
    }
    pub(crate) fn controls(&self) -> &[serde_json::Value] {
        &self.controls
    }
    pub(crate) fn draft_snapshot(&self) -> serde_json::Value {
        self.draft.as_ref().map_or(serde_json::Value::Null, |d| serde_json::json!({"entity_id":d.id.0,"name":d.edit.name,"original_name":d.original.name,"changed":!same_edit(&d.edit,&d.original),"active":d.edit.active,"local_position":d.edit.local_position,"local_rotation":d.edit.local_rotation,"local_scale":d.edit.local_scale,"original_position":d.original.local_position,"original_rotation":d.original.local_rotation,"original_scale":d.original.local_scale,"validation_error":draft_error(&d.edit)}))
    }

    fn show_impl(
        &mut self,
        ui: &mut Ui,
        document: &ImportedDocument,
        explicit: Option<Option<EntityId>>,
        expansion: Option<&crate::component_expansion::ComponentExpansionSnapshot>,
    ) -> Vec<ImportedAction> {
        self.controls.clear();
        let mut actions = Vec::new();
        let compact = appearance::is_reference(ui);
        let selected: Vec<_> = explicit.map_or_else(
            || document.selection().iter().copied().collect(),
            |id| id.into_iter().collect(),
        );
        let Some(id) = explicit.unwrap_or_else(|| document.primary()) else {
            self.selected = selected;
            self.draft = None;
            if !compact {
                ui.heading("Imported Inspector");
            }
            ui.label("Select an imported GameObject to inspect its complete components.");
            return actions;
        };
        let entity = match document.entity(id) {
            Ok(entity) => entity,
            Err(error) => {
                self.draft = None;
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                return actions;
            }
        };
        let source = ImportedEntityEdit {
            name: entity.game_object.name.clone(),
            active: entity.game_object.active,
            local_position: entity.transform.local_position,
            local_rotation: entity.transform.local_rotation,
            local_scale: entity.transform.local_scale,
        };
        if self.selected != selected
            || self.draft.as_ref().is_none_or(|draft| {
                draft.id != id
                    || (!same_edit(&draft.original, &source)
                        && (explicit.is_none() || same_edit(&draft.edit, &draft.original)))
            })
        {
            self.selected = selected;
            self.draft = Some(Draft {
                id,
                original: source.clone(),
                edit: source,
            });
        }
        let Some(draft) = self.draft.as_mut() else {
            return actions;
        };
        if !compact {
            ui.heading("Imported Inspector");
        }
        if self.selected.len() > 1 {
            ui.label(format!(
                "{} selected; editing the primary GameObject.",
                self.selected.len()
            ));
        }
        let identity = format!(
            "GameObject fileID {} · editor ID {}",
            entity.game_object.file_id, id.0
        );
        if compact {
            ui.add(egui::Label::new(egui::RichText::new(&identity).small()).truncate())
                .on_hover_text(&identity);
        } else {
            ui.label(identity);
        }
        ui.horizontal(|ui| {
            let label = ui.label("Name");
            if compact {
                let response = ui
                    .add_sized(
                        [ui.available_width(), 18.0],
                        egui::TextEdit::singleline(&mut draft.edit.name),
                    )
                    .labelled_by(label.id);
                self.controls.push(crate::inspector_lock::control_json(
                    "inspector.name",
                    &response,
                    false,
                ));
            } else {
                ui.text_edit_singleline(&mut draft.edit.name)
                    .labelled_by(label.id);
            }
        });
        ui.checkbox(&mut draft.edit.active, "Active self");
        if compact {
            ui.separator();
        } else {
            ui.small(
            "Components follow the authored component order. Fields and references are read-only.",
        );
        }
        let components = match document.components(id) {
            Ok(components) => components,
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                return actions;
            }
        };
        for component in components {
            let file_id = match component {
                ImportedComponent::Transform(t) => t.file_id,
                ImportedComponent::Script(s) => s.file_id,
            };
            let scoped = expansion.and_then(|state| {
                state
                    .target
                    .filter(|target| target.entity_id == id)
                    .and_then(|target| {
                        state
                            .sections
                            .iter()
                            .find(|s| s.file_id == file_id)
                            .map(|section| (target, section.expanded))
                    })
            });
            if compact && expansion.is_some() && scoped.is_none() {
                continue;
            }
            let response =
            match component {
                ImportedComponent::Transform(transform) => {
                    component_header(format!("Transform · fileID {}", transform.file_id),"imported_transform",file_id,scoped)
                        .show(ui, |ui| {
                            if compact {egui::CollapsingHeader::new("Source Transform details").id_salt(("transform_source",transform.file_id)).icon(crate::resources::disclosure).show(ui,|ui|{
                                ui.label(format!("Owner fileID {} · parent Transform fileID {}",transform.owner,transform.parent));
                                ui.label(format!("Ordered child Transform fileIDs: {:?}",transform.children));
                                ui.monospace(format!("Stored local position: {:?}",transform.local_position));
                                ui.monospace(format!("Stored raw quaternion: {:?}",transform.local_rotation));
                                ui.monospace(format!("Stored local scale: {:?}",transform.local_scale));
                            });} else {
                            ui.label(format!("Owner fileID {} · parent Transform fileID {}", transform.owner, transform.parent));
                            ui.label(format!("Ordered child Transform fileIDs: {:?}", transform.children));
                            ui.monospace(format!("Stored local position: {:?}", transform.local_position));
                            ui.monospace(format!("Stored raw quaternion: {:?}", transform.local_rotation));
                            ui.monospace(format!("Stored local scale: {:?}", transform.local_scale));
                            }
                            float_inputs(ui, "Local position", &mut draft.edit.local_position,&mut self.controls);
                            float_inputs(ui, "Raw local quaternion", &mut draft.edit.local_rotation,&mut self.controls);
                            float_inputs(ui, "Local scale", &mut draft.edit.local_scale,&mut self.controls);
                            if compact {ui.small("Raw quaternion · exact components").on_hover_text("Quaternion components are stored exactly; no Euler conversion or normalization.");}else{ui.small("Quaternion components are stored exactly; no Euler conversion or normalization.");}
                        }).header_response
                }
                ImportedComponent::Script(script) => {
                    let full_header =
                        format!("Script {} · fileID {}", script.type_name, script.file_id);
                    let header = if compact {
                        format!(
                            "{} · {}",
                            script
                                .type_name
                                .rsplit(['.', '+'])
                                .next()
                                .unwrap_or(&script.type_name),
                            script.file_id
                        )
                    } else {
                        full_header.clone()
                    };
                    component_header(header,"imported_script",file_id,scoped)
                        .show(ui, |ui| {
                            if compact {
                                egui::CollapsingHeader::new("Source script details")
                                    .icon(crate::resources::disclosure)
                                    .id_salt(("script_source", script.file_id))
                                    .show(ui, |ui| {
                                        ui.label(format!("Owner fileID {}", script.owner));
                                        ui.label(format!("GUID: {}", script.guid));
                                        ui.label(format!("Assembly: {}", script.assembly));
                                        ui.add(
                                            egui::Label::new(format!("Type: {}", script.type_name))
                                                .wrap(),
                                        );
                                        ui.label(format!(
                                            "Execution order: {}",
                                            script.execution_order
                                        ));
                                    });
                            } else {
                                ui.label(format!("Owner fileID {}", script.owner));
                                ui.label(format!("GUID: {}", script.guid));
                                ui.label(format!("Assembly: {}", script.assembly));
                                ui.label(format!("Type: {}", script.type_name));
                                ui.label(format!("Execution order: {}", script.execution_order));
                            }
                            let mut enabled = script.enabled;
                            if ui.checkbox(&mut enabled, "Enabled").changed() {
                                actions.push(ImportedAction::SetScriptEnabled {
                                    script_file_id: script.file_id,
                                    enabled,
                                });
                            }
                            egui::CollapsingHeader::new("Serialized fields (read-only)")
                                .icon(crate::resources::disclosure)
                                .id_salt(("imported_fields", script.file_id))
                                .default_open(true)
                                .show(ui, |ui| {
                                    for field in &script.fields {
                                        ui.push_id(&field.name, |ui| {
                                            show_field(
                                                ui,
                                                document,
                                                &field.name,
                                                &field.value,
                                                &mut actions,
                                            );
                                        });
                                    }
                                    if script.fields.is_empty() {
                                        ui.weak("No serialized fields.");
                                    }
                                });
                        })
                        .header_response
                        .on_hover_text(full_header)
                }
            };
            if let Some((_, expanded)) = scoped {
                self.controls.push(crate::inspector_lock::control_json(
                    &format!("inspector.component.{file_id}"),
                    &response,
                    expanded,
                ));
                if response.clicked() {
                    actions.push(ImportedAction::SetComponentExpanded {
                        file_id,
                        expanded: !expanded,
                    });
                }
            }
        }
        let error = draft_error(&draft.edit);
        if let Some(error) = error {
            let response = ui.colored_label(ui.visuals().error_fg_color, error);
            if compact {
                self.controls.push(crate::inspector_lock::control_json(
                    "inspector.validation_error",
                    &response,
                    false,
                ));
            }
        }
        let apply = egui::Button::new("Apply imported properties");
        let apply = ui.add_enabled(
            !same_edit(&draft.edit, &draft.original) && error.is_none(),
            apply,
        );
        if compact {
            self.controls.push(crate::inspector_lock::control_json(
                "inspector.apply",
                &apply,
                false,
            ));
        }
        if apply.clicked() {
            actions.push(ImportedAction::EditEntity {
                id,
                edit: draft.edit.clone(),
            });
        }
        ui.add_enabled(
            false,
            egui::Button::new("Edit serialized fields / references"),
        )
        .on_disabled_hover_text(
            "Serialized field/reference edits are unsupported; values remain fully inspectable.",
        );
        ui.add_enabled(false, egui::Button::new("Euler rotation / primitive / component edits"))
            .on_disabled_hover_text("Imported documents preserve raw quaternions and authored components; these edits are unsupported.");
        actions
    }
}

fn component_header(
    label: String,
    kind: &str,
    file_id: i64,
    scoped: Option<(crate::inspector_lock::InspectorTarget, bool)>,
) -> egui::CollapsingHeader {
    let header = egui::CollapsingHeader::new(label)
        .icon(crate::resources::disclosure)
        .default_open(true);
    if let Some((target, expanded)) = scoped {
        let salt = (
            "inspector.component",
            target.document_scope,
            target.entity_id.0,
            file_id,
        );
        // Let the header resolve its own child-UI identity. Individual clicks
        // emit scoped requests above; the shared controller supplies this state.
        header.id_salt(salt).open(Some(expanded))
    } else {
        header.id_salt((kind, file_id))
    }
}

fn float_inputs<const N: usize>(
    ui: &mut Ui,
    name: &str,
    values: &mut [f32; N],
    controls: &mut Vec<serde_json::Value>,
) {
    if appearance::is_reference(ui) {
        ui.label(name);
        ui.horizontal(|ui| {
            let width = ((ui.available_width() - N as f32 * 16.0) / N as f32).max(25.0);
            for (axis, value) in ["X", "Y", "Z", "W"].into_iter().zip(values) {
                ui.push_id((name, axis), |ui| {
                    let label = ui.small(axis);
                    let response = ui
                        .add_sized([width, 18.0], egui::DragValue::new(value).speed(0.1))
                        .labelled_by(label.id)
                        .on_hover_text(format!("{name} {axis}"));
                    let property = match name {
                        "Local position" => "position",
                        "Raw local quaternion" => "rotation",
                        _ => "scale",
                    };
                    controls.push(crate::inspector_lock::control_json(
                        &format!(
                            "inspector.transform.{property}.{}",
                            axis.to_ascii_lowercase()
                        ),
                        &response,
                        false,
                    ));
                });
            }
        });
        return;
    }
    ui.label(name);
    ui.horizontal_wrapped(|ui| {
        for (axis, value) in ["X", "Y", "Z", "W"].into_iter().zip(values) {
            ui.push_id((name, axis), |ui| {
                let label = ui.label(axis);
                ui.add(egui::DragValue::new(value).speed(0.1))
                    .labelled_by(label.id)
                    .on_hover_text(format!("{name} {axis}"));
            });
        }
    });
}

fn show_field(
    ui: &mut Ui,
    document: &ImportedDocument,
    name: &str,
    value: &FieldValue,
    actions: &mut Vec<ImportedAction>,
) {
    let (kind, text) = match value {
        FieldValue::I32(value) => ("i32", value.to_string()),
        FieldValue::F32(value) => ("f32", format!("{value:?}")),
        FieldValue::Bool(value) => ("bool", value.to_string()),
        FieldValue::String(value) => ("string", format!("{value:?}")),
        FieldValue::Vec3(value) => (
            "vec3",
            format!("X {:?} · Y {:?} · Z {:?}", value[0], value[1], value[2]),
        ),
        FieldValue::Quat(value) => (
            "quat",
            format!(
                "X {:?} · Y {:?} · Z {:?} · W {:?}",
                value[0], value[1], value[2], value[3]
            ),
        ),
        FieldValue::Reference(file_id) => {
            ui.label(format!("{name} (reference)"));
            match document.resolve_reference(*file_id) {
                Ok(None) => {
                    ui.label("fileID 0 · null");
                }
                Ok(Some(reference)) => {
                    let kind = match reference.kind {
                        ImportedReferenceKind::GameObject => "GameObject",
                        ImportedReferenceKind::Transform => "Transform",
                        ImportedReferenceKind::Script => "Script",
                    };
                    if ui
                        .button(format!(
                            "fileID {} · {kind} · {}",
                            reference.file_id, reference.name
                        ))
                        .on_hover_text(format!(
                            "Select canonical owner · editor ID {}",
                            reference.owner.0
                        ))
                        .clicked()
                    {
                        actions.push(ImportedAction::Select {
                            id: Some(reference.owner),
                            toggle: false,
                        });
                    }
                }
                Err(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                }
            }
            return;
        }
    };
    if appearance::is_reference(ui) {
        // Keep wrapped values in their own column. horizontal_wrapped's text
        // continuation path can extend the parent left by the preceding label.
        ui.horizontal(|ui| {
            ui.add_sized([112.0, 16.0], egui::Label::new(name))
                .on_hover_text(format!("{name} ({kind}) · read-only"));
            ui.add(egui::Label::new(&text).wrap())
                .on_hover_text(format!("{kind}: {text}"));
        });
    } else {
        ui.label(format!("{name} ({kind})"));
        ui.label(text);
    }
}

fn selection_toggle(ui: &Ui) -> bool {
    ui.input(|input| input.modifiers.command || input.modifiers.ctrl)
}

/// Independent XY Transform markers and axes, with handle-based picking.
pub struct ViewportPanel {
    center: [f64; 2],
    zoom: f64,
    grid_visible: bool,
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self {
            center: [0.0; 2],
            zoom: 64.0,
            grid_visible: true,
        }
    }
}

impl ViewportPanel {
    /// Apply the app's authoring grid preference.
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.grid_visible = visible;
    }
    /// Read the grid preference, including toolbar changes.
    pub fn grid_visible(&self) -> bool {
        self.grid_visible
    }

    /// Draw a disposable derived preview without modifying any serialized value.
    pub fn show(&mut self, ui: &mut Ui, document: &ImportedDocument) -> Vec<ImportedAction> {
        let compact = appearance::is_reference(ui);
        ui.horizontal_wrapped(|ui| {
            ui.strong(if compact {"Scene · Transform XY"} else {"Imported Transform preview · XY"});
            if ui.button("Reset View").clicked() {
                self.center = [0.0; 2];
                self.zoom = 64.0;
            }
            ui.checkbox(&mut self.grid_visible, "World grid");
            if compact {ui.label("ⓘ").on_hover_text("Read-only Transform markers and axes only; no meshes, cameras, materials or Game rendering. Independent quaternion-derived preview. Middle/right drag to pan; scroll to zoom; click to select.");}
        });
        if !compact {
            ui.small(
                "Read-only markers and axes · no meshes, cameras, materials or Game rendering.",
            );
            ui.small("Independent quaternion-derived preview · middle/right drag to pan · scroll to zoom · click to select.");
        }
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        response.clone().on_hover_text(
            "Independent orthographic XY Transform preview; no Unity renderer equivalence.",
        );
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            let delta = response.drag_delta();
            if delta.is_finite() {
                self.center[0] -= f64::from(delta.x) / self.zoom;
                self.center[1] += f64::from(delta.y) / self.zoom;
            }
        }
        if response.hovered() {
            let (scroll, pointer) =
                ui.input(|input| (input.raw_scroll_delta.y, input.pointer.hover_pos()));
            if scroll.is_finite() && scroll != 0.0 {
                let point = pointer.unwrap_or(rect.center());
                let before = self.unproject(rect, point);
                self.zoom = (self.zoom * (f64::from(scroll).clamp(-1000.0, 1000.0) * 0.002).exp())
                    .clamp(4.0, 1024.0);
                let after = self.unproject(rect, point);
                self.center[0] += before[0] - after[0];
                self.center[1] += before[1] - after[1];
            }
        }
        for coordinate in &mut self.center {
            *coordinate = coordinate.clamp(-1.0e9, 1.0e9);
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(23, 29, 34));
        if self.grid_visible {
            for axis in 0..2 {
                let half = f64::from(if axis == 0 {
                    rect.width()
                } else {
                    rect.height()
                }) / (2.0 * self.zoom);
                let start = (self.center[axis] - half).floor();
                let end = (self.center[axis] + half).ceil();
                // Bounded grid density, independent of scene magnitudes.
                let step = ((end - start) / 128.0).ceil().max(1.0);
                for index in 0..=128 {
                    let coordinate = start + f64::from(index) * step;
                    if coordinate > end {
                        break;
                    }
                    let mut point = self.center;
                    point[axis] = coordinate;
                    if let Some(point) = self.project(rect, [point[0], point[1], 0.0]) {
                        let segment = if axis == 0 {
                            [
                                Pos2::new(point.x, rect.top()),
                                Pos2::new(point.x, rect.bottom()),
                            ]
                        } else {
                            [
                                Pos2::new(rect.left(), point.y),
                                Pos2::new(rect.right(), point.y),
                            ]
                        };
                        painter.line_segment(segment, Stroke::new(1.0, Color32::from_gray(43)));
                    }
                }
            }
        }
        let mut pending: Vec<_> = document
            .roots()
            .into_iter()
            .rev()
            .map(|id| (id, Some(Affine::IDENTITY)))
            .collect();
        let mut omitted = 0;
        let mut view_error = None;
        let mut hit = None;
        while let Some((id, parent)) = pending.pop() {
            let entity = match document.entity(id) {
                Ok(entity) => entity,
                Err(error) => {
                    view_error.get_or_insert_with(|| error.to_string());
                    omitted += 1;
                    continue;
                }
            };
            let world = parent.and_then(|parent| {
                parent.compose(Affine::local(
                    entity.transform.local_position,
                    entity.transform.local_rotation,
                    entity.transform.local_scale,
                ))
            });
            match document.children(id) {
                Ok(children) => pending.extend(children.into_iter().rev().map(|id| (id, world))),
                Err(error) => {
                    view_error.get_or_insert_with(|| error.to_string());
                }
            }
            let Some(world) = world else {
                omitted += 1;
                continue;
            };
            let Some(origin) = self.project(rect, world.apply([0.0; 3])) else {
                omitted += 1;
                continue;
            };
            if !rect.expand(32.0).contains(origin) {
                continue;
            }
            let selected = document.selection().contains(&id);
            let color = if selected {
                Color32::from_rgb(255, 199, 92)
            } else if entity.game_object.active {
                Color32::from_rgb(133, 192, 219)
            } else {
                Color32::from_gray(110)
            };
            for (index, axis_color) in [
                Color32::LIGHT_RED,
                Color32::LIGHT_GREEN,
                Color32::LIGHT_BLUE,
            ]
            .into_iter()
            .enumerate()
            {
                let mut endpoint = [0.0; 3];
                endpoint[index] = 0.5;
                if let Some(end) = self.project(rect, world.apply(endpoint)) {
                    painter.line_segment(
                        [origin, end],
                        Stroke::new(if selected { 2.0 } else { 1.0 }, axis_color),
                    );
                    painter.text(
                        end + Vec2::new(3.0, index as f32 * 11.0),
                        Align2::LEFT_TOP,
                        ["X", "Y", "Z"][index],
                        FontId::proportional(10.0),
                        axis_color,
                    );
                } else {
                    omitted += 1;
                }
            }
            painter.circle_filled(origin, if selected { 5.0 } else { 3.5 }, color);
            painter.text(
                origin + Vec2::new(8.0, -8.0),
                Align2::LEFT_BOTTOM,
                format!(
                    "{} · fileID {}",
                    entity.game_object.name, entity.game_object.file_id
                ),
                FontId::proportional(12.0),
                color,
            );
            if response
                .interact_pointer_pos()
                .is_some_and(|point| point.distance(origin) <= 12.0)
            {
                hit = Some(id);
            }
        }
        if omitted > 0 {
            painter.text(rect.left_top() + Vec2::splat(8.0), Align2::LEFT_TOP,
                format!("{omitted} Transform markers/axes omitted: preview range or unavailable view. Hierarchy/Inspector remain available."),
                FontId::proportional(12.0), Color32::LIGHT_RED);
        }
        if let Some(error) = view_error {
            painter.text(
                rect.left_top() + Vec2::new(8.0, 26.0),
                Align2::LEFT_TOP,
                error,
                FontId::proportional(12.0),
                Color32::LIGHT_RED,
            );
        }
        if response.clicked_by(PointerButton::Primary) {
            return vec![ImportedAction::Select {
                id: hit,
                toggle: selection_toggle(ui),
            }];
        }
        Vec::new()
    }

    fn unproject(&self, rect: Rect, point: Pos2) -> [f64; 2] {
        [
            self.center[0] + f64::from(point.x - rect.center().x) / self.zoom,
            self.center[1] - f64::from(point.y - rect.center().y) / self.zoom,
        ]
    }

    fn project(&self, rect: Rect, point: [f64; 3]) -> Option<Pos2> {
        let x = f64::from(rect.center().x) + (point[0] - self.center[0]) * self.zoom;
        let y = f64::from(rect.center().y) - (point[1] - self.center[1]) * self.zoom;
        (x.is_finite() && y.is_finite() && x.abs() < 1.0e7 && y.abs() < 1.0e7)
            .then_some(Pos2::new(x as f32, y as f32))
    }
}

/// Disposable independent T*R*S affine calculation, using f64 preview precision.
#[derive(Clone, Copy)]
struct Affine([[f64; 4]; 3]);

impl Affine {
    const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]);

    fn local(position: [f32; 3], quaternion: [f32; 4], scale: [f32; 3]) -> Self {
        let q = quaternion.map(f64::from);
        let norm = q.iter().map(|value| value * value).sum::<f64>().sqrt();
        let [x, y, z, w] = q.map(|value| value / norm);
        let mut matrix = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
                f64::from(position[0]),
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
                f64::from(position[1]),
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
                f64::from(position[2]),
            ],
        ];
        for row in &mut matrix {
            for (value, scale) in row[..3].iter_mut().zip(scale) {
                *value *= f64::from(scale);
            }
        }
        Self(matrix)
    }

    fn compose(self, local: Self) -> Option<Self> {
        let mut result = [[0.0; 4]; 3];
        for (i, row) in result.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = (0..3).map(|k| self.0[i][k] * local.0[k][j]).sum::<f64>();
                if j == 3 {
                    *value += self.0[i][3];
                }
            }
        }
        result
            .iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(Self(result))
    }

    fn apply(self, point: [f64; 3]) -> [f64; 3] {
        self.0
            .map(|row| row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3])
    }
}
