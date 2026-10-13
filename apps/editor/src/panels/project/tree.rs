//! Virtualized folder rows and read-only directory navigation.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_reference_tree(
        &mut self,
        tree: &mut egui::Ui,
        source: ProjectSource<'_>,
        frame: &mut ReferenceFrame,
    ) {
        let directories = source.directories();
        // No explicit style height is configured for these custom folder rows.
        let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
        tree.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both()
            .id_salt(("project.reference.directories", source.key()))
            .show_rows(tree, row_height, directories.len() + 1, |ui, range| {
                if range.contains(&0) {
                    self.show_root_folder(ui, source, frame);
                }
                for index in range.filter(|i| *i > 0) {
                    let entry_index = directories[index - 1];
                    if let Some(entry) = source.entries().get(entry_index) {
                        self.show_directory_folder(ui, source, entry, frame);
                    }
                }
            });
    }

    fn show_root_folder(
        &mut self,
        ui: &mut egui::Ui,
        source: ProjectSource<'_>,
        frame: &mut ReferenceFrame,
    ) {
        let logical_root = source::logical_root(source.key());
        let role = source::folder_role(source.key(), Path::new(""));
        let (root, mut geometry) = folder_row(
            ui,
            &logical_root,
            0,
            self.directory.as_os_str().is_empty(),
            if self.directory.as_os_str().is_empty() {
                "folder_open"
            } else if !source.has_children(Path::new("")) {
                "folder_empty"
            } else {
                "folder"
            },
        );
        geometry["path"] = serde_json::json!("");
        geometry["source"] = source::observation_key(source.key());
        geometry["role"] = serde_json::json!(role);
        if frame.rendered_folders.len() < 4096 {
            frame.rendered_folders.push(geometry);
        } else {
            frame.geometry_truncated = true;
        }
        frame.controls.push(appearance::control(
            role,
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
        source: ProjectSource<'_>,
        entry: &ProjectEntry,
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
        } else if source.has_children(entry.relative_path.as_path()) {
            "folder"
        } else {
            "folder_empty"
        };
        let (response, mut geometry) = folder_row(ui, &label, depth, selected, icon_role);
        let logical_path = source::logical_path(source.key(), &entry.relative_path);
        let response = response.on_hover_text(&logical_path);
        let role = source::folder_role(source.key(), &entry.relative_path);
        geometry["path"] = serde_json::json!(entry.relative_path.to_string_lossy());
        geometry["source"] = source::observation_key(source.key());
        geometry["role"] = serde_json::json!(role);
        if frame.rendered_folders.len() < 4096 {
            frame.rendered_folders.push(geometry);
        } else {
            frame.geometry_truncated = true;
        }
        frame
            .controls
            .push(appearance::control(role, &response, selected));
        if response.clicked() {
            self.directory = entry.relative_path.clone();
        }
    }
}
