//! Virtualized folder rows and read-only directory navigation.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_reference_tree(
        &mut self,
        tree: &mut egui::Ui,
        project: &Project,
        nonempty: &std::collections::BTreeSet<&Path>,
        frame: &mut ReferenceFrame,
    ) {
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
            .show_rows(tree, row_height, directories.len() + 1, |ui, range| {
                if range.contains(&0) {
                    self.show_root_folder(ui, project, frame);
                }
                for index in range.filter(|i| *i > 0) {
                    self.show_directory_folder(ui, directories[index - 1], nonempty, frame);
                }
            });
    }

    fn show_root_folder(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        frame: &mut ReferenceFrame,
    ) {
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
        if frame.rendered_folders.len() < 4096 {
            frame.rendered_folders.push(geometry);
        } else {
            frame.geometry_truncated = true;
        }
        frame.controls.push(appearance::control(
            "project.folder.Assets",
            &root,
            self.directory.as_os_str().is_empty(),
        ));
        if root.clicked() {
            self.directory.clear();
        }
    }

    fn show_directory_folder(
        &mut self,
        ui: &mut egui::Ui,
        entry: &ProjectEntry,
        nonempty: &std::collections::BTreeSet<&Path>,
        frame: &mut ReferenceFrame,
    ) {
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
        let (response, mut geometry) = folder_row(ui, &label, depth, selected, icon_role);
        let response = response.on_hover_text(entry.relative_path.display().to_string());
        geometry["path"] = serde_json::json!(entry.relative_path.to_string_lossy());
        geometry["role"] =
            serde_json::json!(format!("project.folder.{}", entry.relative_path.display()));
        if frame.rendered_folders.len() < 4096 {
            frame.rendered_folders.push(geometry);
        } else {
            frame.geometry_truncated = true;
        }
        frame.controls.push(appearance::control(
            format!("project.folder.{}", entry.relative_path.display()),
            &response,
            selected,
        ));
        if response.clicked() {
            self.directory = entry.relative_path.clone();
        }
    }
}
