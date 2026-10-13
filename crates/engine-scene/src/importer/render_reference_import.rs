use super::{
    bindings,
    documents::{DocumentConversion, RenderDocumentHandling},
    filesystem::{confined, read},
};
use crate::{
    MAX_DEPTH, MAX_OBJECTS, Result, SceneError, ScenePlan,
    render_reference_extraction::{SceneCameraSourceRecords, SceneRenderReferences},
    yaml,
};
use engine_assets::{Guid, MetadataIndex};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

/// Complete scene import output needed by the player render binder.
///
/// The metadata index and render references come from the same confined project
/// and parsed scene snapshot as `plan`.
pub struct ProjectRenderImport {
    /// The validated scene plan used by the existing player scene session.
    pub plan: ScenePlan,
    /// Serialized mesh and material pointers extracted from the same scene text.
    pub render_references: SceneRenderReferences,
    /// Metadata index discovered from the confined project root.
    pub metadata: MetadataIndex,
}

/// Complete render and serialized Camera source views from one scene import.
pub struct ProjectRenderSourceImport {
    /// The validated plan and render-reference data used by the player binder.
    pub render: ProjectRenderImport,
    /// Serialized Camera values retained from the same parsed scene snapshot.
    pub cameras: SceneCameraSourceRecords,
}

struct PreparedImport {
    scene: PathBuf,
    metadata: MetadataIndex,
    scripts: BTreeMap<Guid, bindings::Binding>,
    settings: Value,
}

struct ImportedRenderViews {
    plan: ScenePlan,
    render_references: SceneRenderReferences,
    camera_sources: Option<SceneCameraSourceRecords>,
    metadata: MetadataIndex,
}

pub(super) fn import_project_legacy(
    root: &Path,
    scene: &Path,
    bindings: &Path,
) -> Result<ScenePlan> {
    let prepared = prepare_import(root, scene, bindings)?;
    let mut conversion = DocumentConversion::new(
        &prepared.scene,
        &prepared.scripts,
        prepared.settings,
        RenderDocumentHandling::Reject,
    )?;

    for document in yaml::parse(&read(&prepared.scene)?)? {
        conversion.push(document)?;
    }

    conversion.finish()
}

/// Import a scene once while retaining the metadata and render-reference views.
///
/// Render extraction is intentionally deferred until conversion finishes so an
/// existing scene-conversion error retains precedence over a stricter render
/// reference error.
pub fn import_project_with_render_references(
    root: &Path,
    scene: &Path,
    bindings: &Path,
) -> Result<ProjectRenderImport> {
    let views =
        import_project_with_render_views(root, scene, bindings, RenderDocumentHandling::Extracted)?;
    Ok(ProjectRenderImport {
        plan: views.plan,
        render_references: views.render_references,
        metadata: views.metadata,
    })
}

/// Import render references and serialized Camera source records from one scene snapshot.
///
/// This route accepts Camera documents and preserves their source values; it does not
/// construct runtime Camera objects or derive projection settings from those values.
pub fn import_project_with_render_sources(
    root: &Path,
    scene: &Path,
    bindings: &Path,
) -> Result<ProjectRenderSourceImport> {
    let views = import_project_with_render_views(
        root,
        scene,
        bindings,
        RenderDocumentHandling::ExtractedWithCameras,
    )?;
    let cameras = views
        .camera_sources
        .ok_or_else(|| SceneError::new("missing Camera source extraction result"))?;

    Ok(ProjectRenderSourceImport {
        render: ProjectRenderImport {
            plan: views.plan,
            render_references: views.render_references,
            metadata: views.metadata,
        },
        cameras,
    })
}

fn import_project_with_render_views(
    root: &Path,
    scene: &Path,
    bindings: &Path,
    document_handling: RenderDocumentHandling,
) -> Result<ImportedRenderViews> {
    let prepared = prepare_import(root, scene, bindings)?;
    let documents = yaml::parse(&read(&prepared.scene)?)?;

    let render_references =
        crate::render_reference_extraction::extract_scene_render_references_from_documents(
            &documents,
        );
    let camera_sources = if document_handling.extracts_cameras() {
        Some(crate::render_reference_extraction::extract_scene_cameras_from_documents(&documents))
    } else {
        None
    };

    let mut conversion = DocumentConversion::new(
        &prepared.scene,
        &prepared.scripts,
        prepared.settings,
        document_handling,
    )?;

    for document in documents {
        conversion.push(document)?;
    }

    let plan = conversion.finish()?;

    // Resolve views in contract order after scene conversion succeeds.
    let render_references = render_references?;
    let camera_sources = camera_sources.transpose()?;

    Ok(ImportedRenderViews {
        plan,
        render_references,
        camera_sources,
        metadata: prepared.metadata,
    })
}

fn prepare_import(root: &Path, scene: &Path, bindings: &Path) -> Result<PreparedImport> {
    if !root.is_absolute()
        || root
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        || !root.is_dir()
    {
        return Err(SceneError::new("expected absolute project directory"));
    }

    let root = confined(root, root)?;
    let scene = confined(&root, scene)?;
    let binding_path = confined(&root, bindings)?;
    let metadata = engine_assets::discover_metadata_index(
        &root,
        engine_assets::MetadataDiscoveryLimits {
            max_depth: MAX_DEPTH,
            max_entries: MAX_OBJECTS,
        },
    )
    .map_err(SceneError::new)?;
    let scene_meta = PathBuf::from(format!(
        "{}.meta",
        scene
            .to_str()
            .ok_or_else(|| SceneError::new("nonUnicode scene path"))?
    ));
    engine_assets::read(&scene_meta).map_err(SceneError::new)?;
    let scripts = bindings::load(&binding_path, &metadata)?;
    let settings = crate::parse_json(include_bytes!("../settings-defaults.json"))?;

    Ok(PreparedImport {
        scene,
        metadata,
        scripts,
        settings,
    })
}
