//! Read-only project discovery panel; listed files are not imported engine assets.

use std::path::{Path, PathBuf};

use crate::appearance;
use eframe::egui;
use openunity_editor_core::project::{Project, ProjectEntryKind};
use openunity_editor_core::search::ranked_indices;

/// Displays the project reader's bounded snapshot without changing project files.
#[derive(Default)]
pub struct ProjectBrowserPanel {
    filter: String,
    project_root: Option<PathBuf>,
    selected_path: Option<PathBuf>,
    directory: PathBuf,
    snapshot: serde_json::Value,
}

impl ProjectBrowserPanel {
    /// The selected Assets-relative path, scoped to the most recently shown project.
    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_path.as_deref()
    }
    /// Actual reference browser state and live controls from the last frame.
    pub fn snapshot(&self) -> serde_json::Value {
        self.snapshot.clone()
    }

    /// Shows source metadata, searchable entries, and read-only selection details.
    pub fn show(&mut self, ui: &mut egui::Ui, project: Option<&Project>) {
        if appearance::is_reference(ui) {
            self.show_reference(ui, project);
            return;
        }
        ui.heading("Project");
        let Some(project) = project else {
            self.project_root = None;
            self.selected_path = None;
            ui.label("No project is open.");
            ui.label("Open a project directory containing Assets and ProjectSettings/ProjectVersion.txt.");
            return;
        };

        if self.project_root.as_deref() != Some(project.root()) {
            self.project_root = Some(project.root().to_path_buf());
            self.selected_path = None;
        }
        if self.selected_path.as_ref().is_some_and(|selected| {
            !project
                .entries()
                .iter()
                .any(|entry| &entry.relative_path == selected)
        }) {
            self.selected_path = None;
        }

        ui.label(format!("Root: {}", project.root().display()));
        ui.label(format!(
            "Declared editor version: {}",
            project.editor_version()
        ));
        ui.label(format!(
            "Discovery diagnostics: {}",
            project.diagnostics().len()
        ));
        ui.label("Read-only source snapshot. Files have not been imported.");
        ui.horizontal(|ui| {
            let label = ui.label("Filter Assets paths:");
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Search Assets paths"))
                .labelled_by(label.id);
            if ui.button("Clear filter").clicked() {
                self.filter.clear();
            }
        });

        let labels: Vec<_> = project
            .entries()
            .iter()
            .map(|entry| entry.relative_path.to_string_lossy())
            .collect();
        let matching = ranked_indices(&self.filter, labels.iter().map(|label| label.as_ref()));
        ui.label(format!(
            "{} of {} entries",
            matching.len(),
            project.entries().len()
        ));
        if project.entries().is_empty() {
            ui.label("Assets contains no discovered files or directories.");
        } else if matching.is_empty() {
            ui.label("No entries match this filter.");
        } else {
            // The reader bounds the snapshot; virtual rows keep rendering bounded
            // to the visible portion even when discovery reaches its entry limit.
            let row_height = ui
                .text_style_height(&egui::TextStyle::Body)
                .max(ui.spacing().interact_size.y);
            egui::ScrollArea::vertical()
                .id_salt("project_entries")
                .max_height((ui.available_height() - 110.0).max(row_height * 2.0))
                .show_rows(ui, row_height, matching.len(), |ui, rows| {
                    for row in rows {
                        let index = matching[row];
                        let entry = &project.entries()[index];
                        let kind = entry_kind(entry.kind);
                        let selected =
                            self.selected_path.as_deref() == Some(entry.relative_path.as_path());
                        if ui
                            .selectable_label(selected, format!("{} ({kind})", labels[index]))
                            .clicked()
                        {
                            self.selected_path = Some(entry.relative_path.clone());
                        }
                    }
                });
        }

        ui.separator();
        if let Some(entry) = project
            .entries()
            .iter()
            .find(|entry| self.selected_path.as_deref() == Some(entry.relative_path.as_path()))
        {
            ui.label(format!("Relative path: {}", entry.relative_path.display()));
            ui.label(format!("Type: {}", entry_kind(entry.kind)));
            ui.label("Status: discovered on disk; read-only; not imported.");
        } else {
            ui.label("Select a file or directory to view its path and discovery status.");
        }
    }

    fn show_reference(&mut self, ui: &mut egui::Ui, project: Option<&Project>) {
        let mut controls = Vec::new();
        let mut rendered_folders = Vec::new();
        let mut geometry_truncated = false;
        let mut search_geometry = serde_json::Value::Null;
        let mut create_icon = serde_json::Value::Null;
        if self.project_root.as_deref() != project.map(Project::root) {
            self.project_root = project.map(|p| p.root().to_path_buf());
            self.selected_path = None;
            self.directory.clear();
        }
        if let Some(project) = project
            && self.selected_path.as_ref().is_some_and(|selected| {
                !project
                    .entries()
                    .iter()
                    .any(|entry| &entry.relative_path == selected)
            })
        {
            self.selected_path = None;
        }
        let toolbar_rect =
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 20.0));
        ui.painter().rect_filled(toolbar_rect, 0.0, appearance::BAR);
        ui.horizontal(|ui| {
            // add_enabled disables this same UI, preserving the original button ID
            // and applying the ordinary disabled painter to the exact resource.
            let create = ui.add_enabled(false, |ui: &mut egui::Ui| {
                let response = ui.add(egui::Button::new(egui::RichText::new("+").color(egui::Color32::TRANSPARENT)).min_size(egui::vec2(16.0, 0.0)));
                let icon_rect = egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0,16.0));
                let painted = crate::resources::paint(ui,"project_create","project.create",response.id,icon_rect);
                if !painted {
                    ui.painter().text(response.rect.center(),egui::Align2::CENTER_CENTER,"+",
                        egui::TextStyle::Button.resolve(ui.style()),ui.style().interact(&response).text_color());
                }
                create_icon = serde_json::json!({"widget_id":response.id.value(),"button_rect":appearance::rect_json(response.rect),
                    "icon_rect":appearance::rect_json(icon_rect),"paint_success":painted,"enabled":response.enabled()});
                response
            }).on_disabled_hover_text("Asset creation and import are unsupported. This browser is read-only.");
            controls.push(appearance::control("project.create",&create,false));
            if let Some(project) = project {
                ui.label("Assets").on_hover_text(format!("{}\nDeclared editor: {}\n{} discovery diagnostics\nRead-only snapshot; files have not been imported.",project.root().display(),project.editor_version(),project.diagnostics().len()));
            } else { ui.weak("No project"); }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                let clear=crate::resources::search_cancel_button(ui,!self.filter.is_empty(),"project.clear").on_hover_text("Clear search");
                controls.push(appearance::control("project.clear",&clear,false));
                if clear.clicked() { self.filter.clear(); }
                // Reserve the existing8px search icon and ordinary item gap.
                // These are layout requests; the receipt retains the actual response.
                let available=ui.available_rect_before_wrap();
                let icon_reservation=8.0+ui.spacing().item_spacing.x;
                let available_width=(available.width()-icon_reservation).max(0.0);
                let requested_width=available_width.clamp(65.0,300.0).min(available_width);
                let requested_rect=egui::Rect::from_min_max(
                    egui::pos2(available.right()-requested_width,available.top()),
                    egui::pos2(available.right(),available.top()+18.0));
                let search=ui.add_sized([requested_width,18.0],egui::TextEdit::singleline(&mut self.filter).hint_text("Search Assets").desired_width(requested_width));
                let (icon,_) = ui.allocate_exact_size(egui::vec2(8.0,18.0),egui::Sense::hover());
                crate::resources::paint(ui,"search","project.search",search.id,egui::Rect::from_center_size(icon.center(),egui::vec2(8.0,8.0)));
                search_geometry=serde_json::json!({"role":"project.search","widget_id":search.id.value(),
                    "requested":{"height":18.0,"min_width":65.0,"max_width":300.0},
                    "available_rect":appearance::rect_json(egui::Rect::from_min_max(egui::pos2(available.left()+icon_reservation,available.top()),available.max)),
                    "requested_rect":appearance::rect_json(requested_rect),"actual_rect":appearance::rect_json(search.rect)});
                controls.push(appearance::control("project.search",&search,false));
            });
        });
        let available = ui.available_rect_before_wrap();
        let footer_height = 18.0;
        let body = egui::Rect::from_min_max(
            available.min,
            egui::pos2(
                available.right(),
                (available.bottom() - footer_height).max(available.top() + 1.0),
            ),
        );
        let tree_width = (body.width() * 0.124)
            .clamp(140.0, 240.0)
            .min((body.width() - 90.0).max(1.0));
        let tree_rect = egui::Rect::from_min_size(body.min, egui::vec2(tree_width, body.height()));
        let content_rect =
            egui::Rect::from_min_max(egui::pos2(tree_rect.right() + 1.0, body.top()), body.max);
        ui.painter()
            .rect_filled(content_rect, 0.0, appearance::ASSETS);
        ui.painter().line_segment(
            [tree_rect.right_top(), tree_rect.right_bottom()],
            egui::Stroke::new(1.0, appearance::DIVIDER),
        );
        let mut tree = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("project.reference.tree")
                .max_rect(tree_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        tree.set_clip_rect(tree_rect.intersect(ui.clip_rect()));
        let Some(project) = project else {
            let mut content =
                ui.new_child(egui::UiBuilder::new().max_rect(content_rect.shrink(8.0)));
            content.label("No project is open.");
            content.small("File → Open Project to browse an existing Assets directory.");
            self.snapshot = serde_json::json!({"folder":"","query":self.filter,"selected_path":null,"visible_entries":[],"tree_rect":appearance::rect_json(tree_rect),"content_rect":appearance::rect_json(content_rect),"controls":controls,"rendered_folders":rendered_folders,"geometry_truncated":geometry_truncated,"search_geometry":search_geometry,"create_icon":create_icon});
            return;
        };
        if !self.directory.as_os_str().is_empty()
            && !project
                .entries()
                .iter()
                .any(|e| e.kind == ProjectEntryKind::Directory && e.relative_path == self.directory)
        {
            self.directory.clear();
        }
        let nonempty: std::collections::BTreeSet<_> = project
            .entries()
            .iter()
            .filter_map(|entry| entry.relative_path.parent())
            .collect();
        let directories: Vec<_> = project
            .entries()
            .iter()
            .filter(|e| e.kind == ProjectEntryKind::Directory)
            .collect();
        // No explicit style height is configured for these custom folder rows.
        let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
        tree.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both()
            .id_salt("project.reference.directories")
            .show_rows(&mut tree, row_height, directories.len() + 1, |ui, range| {
                if range.contains(&0) {
                    let (root, mut geometry) = folder_row(
                        ui,
                        "Assets",
                        0,
                        self.directory.as_os_str().is_empty(),
                        if self.directory.as_os_str().is_empty() {
                            "folder_open"
                        } else if project.entries().is_empty() {
                            "folder_empty"
                        } else {
                            "folder"
                        },
                    );
                    geometry["path"] = serde_json::json!("");
                    geometry["role"] = serde_json::json!("project.folder.Assets");
                    if rendered_folders.len() < 4096 {
                        rendered_folders.push(geometry);
                    } else {
                        geometry_truncated = true;
                    }
                    controls.push(appearance::control(
                        "project.folder.Assets",
                        &root,
                        self.directory.as_os_str().is_empty(),
                    ));
                    if root.clicked() {
                        self.directory.clear();
                    }
                }
                for index in range.filter(|i| *i > 0) {
                    let entry = directories[index - 1];
                    let selected = self.directory == entry.relative_path;
                    let depth = entry.relative_path.components().count();
                    let label = entry
                        .relative_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let icon_role = if selected {
                        "folder_open"
                    } else if nonempty.contains(entry.relative_path.as_path()) {
                        "folder"
                    } else {
                        "folder_empty"
                    };
                    let (response, mut geometry) =
                        folder_row(ui, &label, depth, selected, icon_role);
                    let response =
                        response.on_hover_text(entry.relative_path.display().to_string());
                    geometry["path"] = serde_json::json!(entry.relative_path.to_string_lossy());
                    geometry["role"] = serde_json::json!(format!(
                        "project.folder.{}",
                        entry.relative_path.display()
                    ));
                    if rendered_folders.len() < 4096 {
                        rendered_folders.push(geometry);
                    } else {
                        geometry_truncated = true;
                    }
                    controls.push(appearance::control(
                        format!("project.folder.{}", entry.relative_path.display()),
                        &response,
                        selected,
                    ));
                    if response.clicked() {
                        self.directory = entry.relative_path.clone();
                    }
                }
            });
        let query = self.filter.to_lowercase();
        let entries: Vec<_> = project
            .entries()
            .iter()
            .filter(|e| {
                let parent = e.relative_path.parent().unwrap_or_else(|| Path::new(""));
                parent == self.directory
                    && e.relative_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("project.reference.content")
                .max_rect(content_rect.shrink2(egui::vec2(4.0, 0.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        content.set_clip_rect(content_rect.intersect(ui.clip_rect()));
        let breadcrumb = if self.directory.as_os_str().is_empty() {
            "Assets".to_owned()
        } else {
            format!("Assets / {}", self.directory.display())
        };
        content
            .add_sized(
                [content.available_width(), 18.0],
                egui::Label::new(&breadcrumb).truncate(),
            )
            .on_hover_text(&breadcrumb);
        if entries.is_empty() {
            content.weak(if query.is_empty() {
                "This directory contains no discovered entries."
            } else {
                "No entries match this search."
            });
        }
        let grid_columns = ((content.available_width() + 5.0) / 81.0).floor().max(1.0) as usize;
        let grid_rows = entries.len().div_ceil(grid_columns);
        content.spacing_mut().item_spacing.y = 5.0;
        egui::ScrollArea::vertical()
            .id_salt("project.reference.assets")
            .show_rows(&mut content, 78.0, grid_rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                        let start = row * grid_columns;
                        let end = (start + grid_columns).min(entries.len());
                        for entry in &entries[start..end] {
                            let selected = self.selected_path.as_deref()
                                == Some(entry.relative_path.as_path());
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(76.0, 78.0), egui::Sense::hover());
                            let response =
                                ui.put(rect, egui::Button::new("").selected(selected).frame(false));
                            let label = entry
                                .relative_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    ui.is_enabled(),
                                    selected,
                                    label.as_ref(),
                                )
                            });
                            let icon_role = if entry.kind == ProjectEntryKind::Directory {
                                if self.directory == entry.relative_path {
                                    "folder_open"
                                } else if nonempty.contains(entry.relative_path.as_path()) {
                                    "folder"
                                } else {
                                    "folder_empty"
                                }
                            } else {
                                match entry
                                    .relative_path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(str::to_ascii_lowercase)
                                    .as_deref()
                                {
                                    Some("cs") => "cs_file",
                                    Some(
                                        "txt" | "md" | "json" | "yaml" | "yml" | "xml" | "toml"
                                        | "csv",
                                    ) => "text_file",
                                    _ => "file",
                                }
                            };
                            let icon_rect = egui::Rect::from_center_size(
                                rect.center_top() + egui::vec2(0.0, 27.0),
                                egui::vec2(40.0, 40.0),
                            );
                            if !crate::resources::paint(
                                ui,
                                icon_role,
                                "project.entry",
                                response.id,
                                icon_rect,
                            ) {
                                appearance::asset_glyph(
                                    ui,
                                    icon_rect,
                                    entry.kind == ProjectEntryKind::Directory,
                                    appearance::TEXT,
                                );
                            }
                            ui.put(
                                egui::Rect::from_min_max(
                                    rect.min + egui::vec2(1.0, 52.0),
                                    rect.max,
                                ),
                                egui::Label::new(label.as_ref()).truncate(),
                            );
                            controls.push(appearance::control(
                                format!("project.entry.{}", entry.relative_path.display()),
                                &response,
                                selected,
                            ));
                            if response.clicked() {
                                self.selected_path = Some(entry.relative_path.clone());
                            }
                            if response.double_clicked()
                                && entry.kind == ProjectEntryKind::Directory
                            {
                                self.directory = entry.relative_path.clone();
                            }
                            response.on_hover_text(format!(
                                "Assets/{}\n{} · read-only · not imported",
                                entry.relative_path.display(),
                                entry_kind(entry.kind)
                            ));
                        }
                    });
                }
            });
        let footer =
            egui::Rect::from_min_max(egui::pos2(body.left(), body.bottom() + 1.0), available.max);
        ui.painter().rect_filled(footer, 0.0, appearance::BAR);
        let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer));
        let details = self.selected_path.as_ref().map_or_else(
            || format!("{} entries · read-only", entries.len()),
            |p| format!("Assets/{} · read-only · not imported", p.display()),
        );
        footer_ui
            .add(egui::Label::new(&details).truncate())
            .on_hover_text(details);
        self.snapshot = serde_json::json!({"folder":self.directory.to_string_lossy(),"query":self.filter,"selected_path":self.selected_path.as_ref().map(|p|p.to_string_lossy()),
            "visible_entries":entries.iter().map(|e|serde_json::json!({"path":e.relative_path.to_string_lossy(),"kind":entry_kind(e.kind)})).collect::<Vec<_>>(),
            "tree_rect":appearance::rect_json(tree_rect),"content_rect":appearance::rect_json(content_rect),"controls":controls,"rendered_folders":rendered_folders,"geometry_truncated":geometry_truncated,"search_geometry":search_geometry,"create_icon":create_icon});
    }
}

fn folder_row(
    ui: &mut egui::Ui,
    label: &str,
    depth: usize,
    selected: bool,
    icon_role: &str,
) -> (egui::Response, serde_json::Value) {
    let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(120.0), row_height),
        egui::Sense::hover(),
    );
    let minimum_height = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = row_height;
    let response = ui.put(rect, egui::Button::new("").selected(selected).frame(false));
    ui.spacing_mut().interact_size.y = minimum_height;
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    let indent = appearance::tree_indent(depth);
    let x = rect.left() + indent;
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(x, rect.center().y - appearance::TREE_ICON_WIDTH * 0.5),
        egui::vec2(appearance::TREE_ICON_WIDTH, appearance::TREE_ICON_WIDTH),
    );
    if !crate::resources::paint(ui, icon_role, "project.folder", response.id, icon_rect) {
        appearance::asset_glyph(ui, icon_rect, true, appearance::TEXT);
    }
    let label_origin = egui::pos2(
        icon_rect.right() + appearance::TREE_ICON_LABEL_GAP,
        rect.center().y,
    );
    let label_rect = ui.painter().text(
        label_origin,
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        appearance::TEXT,
    );
    let geometry = serde_json::json!({"widget_id":response.id.value(),"label":label,"depth":depth,
        "row_rect":appearance::rect_json(response.rect),"indent_anchor":[x,rect.center().y],
        "indent_without_icon_padding":indent,"disclosure_reservation":0.0,"disclosure_rect":null,
        "icon_rect":appearance::rect_json(icon_rect),"label_origin":[label_origin.x,label_origin.y],
        "label_rect":appearance::rect_json(label_rect),"row_height":response.rect.height(),
        "style_resolved_line_height":null,"row_height_source":"fallback"});
    (response, geometry)
}

fn entry_kind(kind: ProjectEntryKind) -> &'static str {
    match kind {
        ProjectEntryKind::Directory => "directory",
        ProjectEntryKind::File => "file",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "openunity-project-panel-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("Assets")).unwrap();
            fs::create_dir(root.join("ProjectSettings")).unwrap();
            fs::write(
                root.join("ProjectSettings/ProjectVersion.txt"),
                "m_EditorVersion: 6000.0.23f1\n",
            )
            .unwrap();
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn frame(
        ctx: &egui::Context,
        panel: &mut ProjectBrowserPanel,
        project: Option<&Project>,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, project));
            },
        )
    }

    fn text_position(output: &egui::FullOutput, needle: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.text().contains(needle) => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
    }

    fn click(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[test]
    fn renders_missing_and_empty_projects_with_honest_guidance() {
        let ctx = egui::Context::default();
        let mut panel = ProjectBrowserPanel::default();
        let output = frame(&ctx, &mut panel, None, vec![]);
        assert!(text_position(&output, "No project is open").is_some());
        assert!(panel.selected_path().is_none());

        let fixture = Fixture::new();
        let project = Project::open(&fixture.0).unwrap();
        let output = frame(&ctx, &mut panel, Some(&project), vec![]);
        assert!(text_position(&output, "6000.0.23f1").is_some());
        assert!(text_position(&output, "no discovered files").is_some());
    }

    #[test]
    fn selects_real_paths_filters_and_clears_selection_on_project_replacement() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("Assets/Models")).unwrap();
        fs::write(
            fixture.0.join("Assets/Models/Cube.txt"),
            "independent fixture",
        )
        .unwrap();
        fs::write(
            fixture.0.join("Assets/Camera.txt"),
            "independent camera fixture",
        )
        .unwrap();
        let project = Project::open(&fixture.0).unwrap();
        let ctx = egui::Context::default();
        let mut panel = ProjectBrowserPanel::default();
        let output = frame(&ctx, &mut panel, Some(&project), vec![]);
        let cube = text_position(&output, "Models/Cube.txt (file)").unwrap();
        let output = frame(&ctx, &mut panel, Some(&project), click(cube));
        assert_eq!(panel.selected_path(), Some(Path::new("Models/Cube.txt")));
        assert!(text_position(&output, "not imported").is_some());
        assert!(text_position(&output, "Relative path: Models/Cube.txt").is_some());

        let filter = text_position(&output, "Search Assets paths").unwrap();
        frame(&ctx, &mut panel, Some(&project), click(filter));
        let output = frame(
            &ctx,
            &mut panel,
            Some(&project),
            vec![egui::Event::Text("cAmErA".into())],
        );
        assert!(text_position(&output, "Camera.txt (file)").is_some());
        assert!(text_position(&output, "Models/Cube.txt (file)").is_none());
        assert!(text_position(&output, "1 of 3 entries").is_some());

        let replacement = Fixture::new();
        let replacement = Project::open(&replacement.0).unwrap();
        frame(&ctx, &mut panel, Some(&replacement), vec![]);
        assert!(panel.selected_path().is_none());
        frame(&ctx, &mut panel, None, vec![]);
        assert!(panel.selected_path().is_none());
        assert_eq!(
            fs::read_to_string(fixture.0.join("Assets/Models/Cube.txt")).unwrap(),
            "independent fixture"
        );
    }
}
