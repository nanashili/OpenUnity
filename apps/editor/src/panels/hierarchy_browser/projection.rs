use super::HierarchyBrowser;

impl HierarchyBrowser {
    pub(super) fn ensure_visible_indices(&mut self) {
        if !self.lowercase_query.is_empty() && self.search_included.is_none() {
            self.search_included = Some(self.build_search_inclusion());
        }

        if self.visible_indices.is_some() {
            return;
        }

        let Some(source) = &self.source else {
            self.visible_indices = Some(Vec::new());
            return;
        };

        let visible = if !self.lowercase_query.is_empty() {
            self.search_included
                .as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
                .filter_map(|(index, included)| included.then_some(index))
                .collect()
        } else {
            self.build_expanded_visibility(source)
        };

        self.visible_indices = Some(visible);
    }

    pub(super) fn toggle_scene_expansion(&mut self) {
        self.scene_expanded = !self.scene_expanded;
        self.invalidate_visibility_after_expansion();
    }

    pub(super) fn toggle_branch_expansion(&mut self, row_index: usize) {
        let Some((id, has_children)) = self
            .source
            .as_ref()
            .and_then(|source| source.rows.get(row_index))
            .map(|row| (row.id, row.has_children))
        else {
            return;
        };
        if !has_children {
            return;
        }

        let is_collapsed = {
            let Some(collapsed) = self.collapsed_rows.get_mut(row_index) else {
                return;
            };
            *collapsed = !*collapsed;
            *collapsed
        };

        if is_collapsed {
            self.collapsed_branches.insert(id);
        } else {
            self.collapsed_branches.remove(&id);
        }

        self.invalidate_visibility_after_expansion();
    }

    fn build_search_inclusion(&self) -> Vec<bool> {
        let Some(source) = &self.source else {
            return Vec::new();
        };

        let mut included = vec![false; source.rows.len()];
        for (index, row) in source.rows.iter().enumerate() {
            if !row.lowercase_name.contains(&self.lowercase_query) {
                continue;
            }

            let mut ancestor = Some(index);
            while let Some(current) = ancestor {
                if included[current] {
                    break;
                }
                included[current] = true;
                ancestor = source.parent_indices[current];
            }
        }

        included
    }

    fn build_expanded_visibility(&self, source: &super::CachedForest) -> Vec<usize> {
        if !self.scene_expanded {
            return Vec::new();
        }

        let mut visible = Vec::with_capacity(source.rows.len());
        let mut hidden_ancestor_depth = None;

        for (index, row) in source.rows.iter().enumerate() {
            if hidden_ancestor_depth.is_some_and(|depth| row.depth <= depth) {
                hidden_ancestor_depth = None;
            }

            if hidden_ancestor_depth.is_some() {
                continue;
            }

            visible.push(index);
            if self.collapsed_rows.get(index).copied().unwrap_or(false) {
                hidden_ancestor_depth = Some(row.depth);
            }
        }

        visible
    }

    fn invalidate_visibility_after_expansion(&mut self) {
        if self.lowercase_query.is_empty() {
            self.visible_indices = None;
        }
    }
}
