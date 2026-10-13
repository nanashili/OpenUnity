//! Asset tile semantics, icon classification, painting and selection.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_asset_entry(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        entry: &ProjectEntry,
        frame: &mut ReferenceFrame,
    ) {
        let selected = self.selected_path.as_deref() == Some(entry.relative_path.as_path());
        let (rect, _) = ui.allocate_exact_size(egui::vec2(76.0, 78.0), egui::Sense::hover());
        let response = ui.put(rect, egui::Button::new("").selected(selected).frame(false));
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
        let icon_role = self.asset_icon_role(project, entry);
        Self::paint_asset_entry(ui, rect, &response, entry, label.as_ref(), icon_role);
        frame.controls.push(appearance::control(
            format!("project.entry.{}", entry.relative_path.display()),
            &response,
            selected,
        ));
        if response.clicked() {
            self.selected_path = Some(entry.relative_path.clone());
        }
        if response.double_clicked() && entry.kind == ProjectEntryKind::Directory {
            self.directory = entry.relative_path.clone();
        }
        response.on_hover_text(format!(
            "Assets/{}\n{} · read-only · not imported",
            entry.relative_path.display(),
            entry_kind(entry.kind)
        ));
    }

    fn asset_icon_role(&self, project: &Project, entry: &ProjectEntry) -> &'static str {
        if entry.kind == ProjectEntryKind::Directory {
            if self.directory == entry.relative_path {
                "folder_open"
            } else if project.has_children(entry.relative_path.as_path()) {
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
                Some("txt" | "md" | "json" | "yaml" | "yml" | "xml" | "toml" | "csv") => {
                    "text_file"
                }
                _ => "file",
            }
        }
    }

    fn paint_asset_entry(
        ui: &mut egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        entry: &ProjectEntry,
        label: &str,
        icon_role: &str,
    ) {
        let icon_rect = egui::Rect::from_center_size(
            rect.center_top() + egui::vec2(0.0, 27.0),
            egui::vec2(40.0, 40.0),
        );
        if !crate::resources::paint(ui, icon_role, "project.entry", response.id, icon_rect) {
            appearance::asset_glyph(
                ui,
                icon_rect,
                entry.kind == ProjectEntryKind::Directory,
                appearance::TEXT,
            );
        }
        ui.put(
            egui::Rect::from_min_max(rect.min + egui::vec2(1.0, 52.0), rect.max),
            egui::Label::new(label).truncate(),
        );
    }
}

pub(super) fn entry_kind(kind: ProjectEntryKind) -> &'static str {
    match kind {
        ProjectEntryKind::Directory => "directory",
        ProjectEntryKind::File => "file",
    }
}
