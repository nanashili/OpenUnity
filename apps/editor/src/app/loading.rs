use super::*;

impl EditorApp {
    pub(super) fn load_document(&self, path: &Path) -> Result<Pending, String> {
        if path
            .extension()
            .is_some_and(|extension| extension == "ouimport")
        {
            self.check_input(path)?;
            let snapshot =
                imported_scene_io::load(path, &self.policy).map_err(|e| e.to_string())?;
            let mut document =
                ImportedDocument::from_snapshot(snapshot, self.preferences.history_capacity)
                    .map_err(|e| e.to_string())?;
            if self
                .forbidden_reads
                .iter()
                .any(|root| document.source().project_root.starts_with(root))
            {
                return Err("Archive provenance cannot name a reference installation or restricted research root.".into());
            }
            document.mark_saved();
            Ok(Pending::Imported(
                Box::new(document),
                Some(path.to_path_buf()),
            ))
        } else {
            Ok(Pending::Open(self.load_scene(path)?, path.to_path_buf()))
        }
    }

    /// Read a bounded scene from the explicitly opened source project.
    pub fn request_import(&mut self, scene: &Path, bindings: &Path) -> Result<(), String> {
        self.check_idle()?;
        let project = self
            .project
            .as_ref()
            .ok_or("Open a source project before importing a saved scene.")?;
        let scene = if scene.is_absolute() {
            scene.to_path_buf()
        } else {
            project.root().join(scene)
        };
        let bindings = if bindings.is_absolute() {
            bindings.to_path_buf()
        } else {
            project.root().join(bindings)
        };
        self.check_input(&scene)?;
        self.check_input(&bindings)?;
        let document = ImportedDocument::import_project(
            project.root(),
            &scene,
            &bindings,
            self.preferences.history_capacity,
        )
        .map_err(|e| e.to_string())?;
        let candidate = Pending::Imported(Box::new(document), None);
        self.protect_candidate(&candidate);
        self.request(candidate)
    }

    /// Read a project and retain protection for all previously opened source roots.
    pub fn open_project(&mut self, path: &Path) -> Result<(), String> {
        self.check_input(path)?;
        let project = Project::open(path).map_err(|e| e.to_string())?;
        let mut roots = self.protected_roots.clone();
        roots.push(project.root().to_path_buf());
        let policy = WritePolicy::new(roots.clone()).map_err(|e| e.to_string())?;

        super::project_warnings::record_project_warnings(&mut self.console, project.diagnostics())?;

        self.project_text = project.root().display().to_string();
        self.protected_roots = roots;
        self.policy = policy;
        self.project = Some(project);
        Ok(())
    }

    pub(super) fn load_scene(&self, path: &Path) -> Result<Scene, String> {
        if path
            .extension()
            .is_some_and(|extension| extension == "unity")
        {
            return Err(capabilities::attempt(Capability::UnitySceneImport)
                .unwrap_err()
                .to_string());
        }
        self.check_input(path)?;
        scene_io::load(path, &self.policy).map_err(|e| e.to_string())
    }
}
