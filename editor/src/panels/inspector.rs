//! Draft-based editing of the selected independent scene entity.

use eframe::egui::{self, Ui};
use openunity_editor_core::scene::{Entity, EntityId, MAX_NAME_BYTES, Scene, Transform};
use openunity_editor_core::selection::Selection;

use crate::EditorAction;
use crate::appearance;

/// Inspector drafts stay local until the shell executes an Apply action.
#[derive(Default)]
pub struct InspectorPanel {
    selected: Vec<EntityId>,
    draft: Option<Draft>,
    controls: Vec<serde_json::Value>,
}

struct Draft {
    source: Entity,
    name: String,
    active: bool,
    transform: Transform,
}

impl Draft {
    fn new(source: &Entity) -> Self {
        Self {
            source: source.clone(),
            name: source.name.clone(),
            active: source.active,
            transform: source.transform,
        }
    }

    fn changed(&self) -> bool {
        self.name != self.source.name
            || self.active != self.source.active
            || self.transform != self.source.transform
    }

    fn validation_error(&self) -> Option<&'static str> {
        if self.name.is_empty() {
            return Some("Edit entity: name must not be empty.");
        }
        if self.name.len() > MAX_NAME_BYTES {
            return Some("Edit entity: name exceeds 1024 UTF-8 bytes.");
        }
        if self
            .transform
            .translation
            .iter()
            .chain(&self.transform.rotation_deg)
            .chain(&self.transform.scale)
            .any(|value| !value.is_finite())
        {
            return Some("Edit entity: all transform values must be finite.");
        }
        None
    }
}

impl InspectorPanel {
    /// Render the primary selection and return at most one complete edit.
    ///
    /// Selection changes and external entity edits replace any uncommitted draft.
    pub fn show(&mut self, ui: &mut Ui, scene: &Scene, selection: &Selection) -> Vec<EditorAction> {
        self.show_impl(ui, scene, selection, None)
    }

    /// Render one explicitly scoped app target without following global selection.
    pub(crate) fn show_target(
        &mut self,
        ui: &mut Ui,
        scene: &Scene,
        selection: &Selection,
        target: Option<EntityId>,
    ) -> Vec<EditorAction> {
        self.show_impl(ui, scene, selection, Some(target))
    }

    pub(crate) fn draft_state(&self) -> (Option<EntityId>, bool) {
        (
            self.draft.as_ref().map(|d| d.source.id),
            self.draft.as_ref().is_some_and(Draft::changed),
        )
    }
    pub(crate) fn controls(&self) -> &[serde_json::Value] {
        &self.controls
    }
    pub(crate) fn draft_snapshot(&self) -> serde_json::Value {
        self.draft.as_ref().map_or(serde_json::Value::Null, |d| serde_json::json!({"entity_id":d.source.id.0,"name":d.name,"original_name":d.source.name,"changed":d.changed()}))
    }

    fn show_impl(
        &mut self,
        ui: &mut Ui,
        scene: &Scene,
        selection: &Selection,
        explicit: Option<Option<EntityId>>,
    ) -> Vec<EditorAction> {
        self.controls.clear();
        let selected: Vec<_> =
            explicit.map_or_else(|| selection.ids().collect(), |id| id.into_iter().collect());
        let entity = explicit
            .unwrap_or_else(|| selection.primary())
            .and_then(|id| scene.get(id));
        let source_changed = self.draft.as_ref().map(|draft| &draft.source) != entity;
        let dirty = self.draft.as_ref().is_some_and(Draft::changed);
        if self.selected != selected || (source_changed && (explicit.is_none() || !dirty)) {
            self.selected = selected;
            self.draft = entity.map(Draft::new);
        }

        let compact = appearance::is_reference(ui);
        if !compact {
            ui.heading("Inspector");
        }
        let Some(draft) = &mut self.draft else {
            ui.label("Select an entity to inspect its local properties.");
            return Vec::new();
        };

        if self.selected.len() > 1 {
            ui.label(format!(
                "{} entities selected. Editing only the primary entity.",
                self.selected.len()
            ));
        }
        if compact {
            ui.small(format!(
                "{:?} · ID {}",
                draft.source.primitive, draft.source.id.0
            ))
            .on_hover_text(match draft.source.parent {
                Some(parent) => format!("Parent: {}", parent.0),
                None => "Parent: none (root)".into(),
            });
        } else {
            ui.label(format!("ID: {}", draft.source.id.0));
            ui.label(format!("Primitive: {:?}", draft.source.primitive));
            ui.label(match draft.source.parent {
                Some(parent) => format!("Parent: {}", parent.0),
                None => "Parent: none (root)".to_owned(),
            });
            ui.separator();
        }
        ui.horizontal(|ui| {
            let label = ui.label("Name");
            if compact {
                let response = ui
                    .add_sized(
                        [ui.available_width(), 18.0],
                        egui::TextEdit::singleline(&mut draft.name),
                    )
                    .labelled_by(label.id);
                self.controls.push(crate::inspector_lock::control_json(
                    "inspector.name",
                    &response,
                    false,
                ));
            } else {
                ui.text_edit_singleline(&mut draft.name)
                    .labelled_by(label.id);
            }
        });
        ui.checkbox(&mut draft.active, "Active");
        if compact {
            ui.separator();
            ui.strong("Transform");
        }
        transform_row(ui, "Translation", &mut draft.transform.translation);
        transform_row(ui, "Rotation (degrees)", &mut draft.transform.rotation_deg);
        transform_row(ui, "Scale", &mut draft.transform.scale);

        let error = draft.validation_error();
        if let Some(error) = error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let apply = ui.add_enabled(
            draft.changed() && error.is_none(),
            egui::Button::new("Apply"),
        );
        if compact {
            self.controls.push(crate::inspector_lock::control_json(
                "inspector.apply",
                &apply,
                false,
            ));
        }
        if apply.clicked() {
            return vec![EditorAction::Edit {
                id: draft.source.id,
                name: draft.name.clone(),
                active: draft.active,
                transform: draft.transform,
            }];
        }
        Vec::new()
    }
}

fn transform_row(ui: &mut Ui, name: &str, values: &mut [f32; 3]) {
    if appearance::is_reference(ui) {
        ui.horizontal(|ui| {
            ui.add_sized([94.0, 18.0], egui::Label::new(name));
            let width = ((ui.available_width() - 38.0) / 3.0).max(26.0);
            for (axis, value) in ["X", "Y", "Z"].into_iter().zip(values) {
                ui.push_id((name, axis), |ui| {
                    let label = ui.small(axis);
                    ui.add_sized([width, 18.0], egui::DragValue::new(value).speed(0.1))
                        .labelled_by(label.id)
                        .on_hover_text(format!("{name} {axis}"));
                });
            }
        });
        return;
    }
    ui.label(name);
    ui.horizontal_wrapped(|ui| {
        for (axis, value) in ["X", "Y", "Z"].into_iter().zip(values) {
            ui.push_id((name, axis), |ui| {
                let label = ui.label(axis);
                ui.add(egui::DragValue::new(value).speed(0.1))
                    .labelled_by(label.id)
                    .on_hover_text(format!("{name} {axis}"));
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Vec2};
    use openunity_editor_core::scene::Primitive;

    fn frame(
        context: &Context,
        panel: &mut InspectorPanel,
        scene: &Scene,
        selection: &Selection,
        events: Vec<Event>,
    ) -> (Vec<EditorAction>, FullOutput) {
        let mut actions = Vec::new();
        let output = context.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 600.0))),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    actions = panel.show(ui, scene, selection);
                });
            },
        );
        (actions, output)
    }

    fn text_position(output: &FullOutput, text: &str) -> Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(shape) if shape.galley.job.text == text => {
                    Some(shape.pos + shape.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing visible text: {text}"))
    }

    fn click(
        context: &Context,
        panel: &mut InspectorPanel,
        scene: &Scene,
        selection: &Selection,
        position: Pos2,
    ) -> Vec<EditorAction> {
        frame(
            context,
            panel,
            scene,
            selection,
            vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            context,
            panel,
            scene,
            selection,
            vec![Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        )
        .0
    }

    #[test]
    fn apply_emits_one_complete_edit_without_mutating_the_scene() {
        let context = Context::default();
        let mut panel = InspectorPanel::default();
        let mut scene = Scene::new("Inspector input").unwrap();
        let id = scene.create("Before", Primitive::Cube, None).unwrap();
        let before = scene.clone();
        let mut selection = Selection::new();
        selection.replace(Some(id), &scene).unwrap();
        frame(&context, &mut panel, &scene, &selection, Vec::new());

        // Author a draft, then exercise the actual Apply widget through pointer input.
        let draft = panel.draft.as_mut().unwrap();
        draft.name = "After".into();
        draft.active = false;
        draft.transform = Transform {
            translation: [1.0, 2.0, 3.0],
            rotation_deg: [10.0, 20.0, 30.0],
            scale: [-1.0, 0.0, 2.0],
        };
        let expected = EditorAction::Edit {
            id,
            name: "After".into(),
            active: false,
            transform: draft.transform,
        };
        let (actions, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
        assert!(actions.is_empty());
        let position = text_position(&output, "Apply");
        assert_eq!(
            click(&context, &mut panel, &scene, &selection, position),
            [expected]
        );
        assert_eq!(scene, before);

        // Invalid drafts remain visible and cannot produce actions from Apply.
        for invalid in ["empty", "long", "nonfinite"] {
            let draft = panel.draft.as_mut().unwrap();
            draft.name = "After".into();
            draft.transform = Transform::default();
            let error = match invalid {
                "empty" => {
                    draft.name.clear();
                    "Edit entity: name must not be empty."
                }
                "long" => {
                    draft.name = "é".repeat(MAX_NAME_BYTES / 2 + 1);
                    "Edit entity: name exceeds 1024 UTF-8 bytes."
                }
                _ => {
                    draft.transform.scale[2] = f32::INFINITY;
                    "Edit entity: all transform values must be finite."
                }
            };
            let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
            text_position(&output, error);
            assert!(
                click(
                    &context,
                    &mut panel,
                    &scene,
                    &selection,
                    text_position(&output, "Apply"),
                )
                .is_empty()
            );
            assert_eq!(scene, before);
        }
    }

    #[test]
    fn selection_and_external_edits_refresh_drafts_and_empty_selection_clears_form() {
        let context = Context::default();
        let mut panel = InspectorPanel::default();
        let mut scene = Scene::new("Inspector lifecycle").unwrap();
        let first = scene.create("First", Primitive::Empty, None).unwrap();
        let second = scene
            .create("Second", Primitive::Camera, Some(first))
            .unwrap();
        let mut selection = Selection::new();
        let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
        text_position(&output, "Select an entity to inspect its local properties.");

        selection.replace(Some(second), &scene).unwrap();
        let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
        text_position(&output, "Primitive: Camera");
        text_position(&output, &format!("Parent: {}", first.0));
        panel.draft.as_mut().unwrap().name = "Uncommitted".into();
        scene
            .edit(second, |entity| entity.name = "Restored".into())
            .unwrap();
        frame(&context, &mut panel, &scene, &selection, Vec::new());
        assert_eq!(panel.draft.as_ref().unwrap().name, "Restored");

        selection.toggle(first, &scene).unwrap();
        let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
        text_position(
            &output,
            "2 entities selected. Editing only the primary entity.",
        );
        assert_eq!(panel.draft.as_ref().unwrap().name, "First");
        selection.clear();
        let (actions, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
        assert!(actions.is_empty());
        text_position(&output, "Select an entity to inspect its local properties.");
        assert!(panel.draft.is_none());
    }
}
