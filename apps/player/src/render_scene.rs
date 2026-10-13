//! Player-owned orchestration for the reviewed same-session render route.

use crate::arguments::RenderArguments;
use crate::render_frames;
use crate::render_output::{CleanupOutcome, CleanupResult, PublishError, RenderOutputPublisher};
use crate::render_preflight::{self, PreparedRender};
use engine_render_wgpu::WgpuRenderer;
use engine_scripting_bridge::{ManagedHost, ScriptSession};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum SessionPhase {
    Created,
    Initialized,
}

/// A route admission, execution, teardown, or artifact publication failure.
#[derive(Debug)]
pub struct RenderSceneError(String);

impl Display for RenderSceneError {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        output.write_str(&self.0)
    }
}

impl Error for RenderSceneError {}

/// Execute one admitted route and publish only a post-teardown receipt.
///
/// Scene and manifest input is checked before session/device creation. Once
/// OUTPUT exists, execution failures attempt to preserve a `failure.json`.
pub fn run(args: &[String]) -> Result<(), RenderSceneError> {
    let arguments = RenderArguments::parse(args).map_err(|error| {
        RenderSceneError(format!("invalid --render-scene arguments: {error:?}"))
    })?;
    let prepared = render_preflight::load(&arguments).map_err(RenderSceneError)?;
    let publisher = RenderOutputPublisher::create(
        arguments.output(),
        arguments.target_size(),
        arguments.policy(),
    )
    .map_err(|error| RenderSceneError(error.to_string()))?;

    run_admitted(arguments, prepared, publisher)
}

fn run_admitted(
    arguments: RenderArguments,
    prepared: PreparedRender,
    publisher: RenderOutputPublisher,
) -> Result<(), RenderSceneError> {
    // SAFETY: these explicit trusted paths must implement the reviewed hostfxr,
    // scene-controller, and same-session scene-snapshot ABI contracts.
    let host = match unsafe {
        ManagedHost::load_scene_with_snapshot(
            arguments.root(),
            arguments.hostfxr(),
            arguments.bridge(),
            arguments.config(),
        )
    } {
        Ok(host) => host,
        Err(error) => {
            return publish_failure(
                publisher,
                format!("load snapshot-capable managed host: {error}"),
                unavailable_cleanup(),
            );
        }
    };

    let session = match host.create_scene(arguments.assembly(), &prepared.plan_payload) {
        Ok(session) => session,
        Err(error) => {
            return publish_failure(
                publisher,
                format!("create managed scene session: {error}"),
                unavailable_cleanup(),
            );
        }
    };

    let renderer = match WgpuRenderer::create(arguments.policy()) {
        Ok(renderer) => renderer,
        Err(error) => {
            let (shutdown, close) = cleanup_session(session, SessionPhase::Created);
            let cleanup = CleanupOutcome {
                shutdown,
                close,
                renderer: CleanupResult::NotAttempted,
            };
            let diagnostic = include_cleanup_diagnostics(
                format!("create requested WGPU renderer: {error:?}"),
                &cleanup,
            );
            return publish_failure(publisher, diagnostic, cleanup);
        }
    };

    run_live_session(session, renderer, &arguments, &prepared, publisher)
}

fn run_live_session(
    session: ScriptSession<'_>,
    mut renderer: WgpuRenderer,
    arguments: &RenderArguments,
    prepared: &PreparedRender,
    publisher: RenderOutputPublisher,
) -> Result<(), RenderSceneError> {
    let mut session = session;
    let mut phase = SessionPhase::Created;
    let mut publisher = publisher;
    let execution = render_frames::execute(
        &mut session,
        &mut renderer,
        prepared,
        arguments,
        &mut publisher,
        &mut phase,
    );

    let (shutdown, close) = cleanup_session(session, phase);
    drop(renderer);
    let cleanup = CleanupOutcome {
        shutdown,
        close,
        renderer: CleanupResult::Succeeded,
    };

    let diagnostic = execution
        .err()
        .map(|error| include_cleanup_diagnostics(error, &cleanup))
        .or_else(|| cleanup_failure(&cleanup));
    match diagnostic {
        Some(diagnostic) => publish_failure(publisher, diagnostic, cleanup),
        None => publisher
            .finish(cleanup)
            .map_err(|error| RenderSceneError(error.to_string())),
    }
}

fn cleanup_session(
    mut session: ScriptSession<'_>,
    phase: SessionPhase,
) -> (CleanupResult, CleanupResult) {
    let shutdown = match phase {
        SessionPhase::Created => CleanupResult::NotAttempted,
        SessionPhase::Initialized => match session.shutdown(0) {
            Ok(_) => CleanupResult::Succeeded,
            Err(error) => CleanupResult::Failed {
                diagnostic: error.to_string(),
            },
        },
    };
    let close = match session.close() {
        Ok(()) => CleanupResult::Succeeded,
        Err(error) => CleanupResult::Failed {
            diagnostic: error.to_string(),
        },
    };
    (shutdown, close)
}

fn unavailable_cleanup() -> CleanupOutcome {
    CleanupOutcome {
        shutdown: CleanupResult::NotAttempted,
        close: CleanupResult::NotAttempted,
        renderer: CleanupResult::NotAttempted,
    }
}

fn cleanup_failure(cleanup: &CleanupOutcome) -> Option<String> {
    let mut diagnostics = Vec::new();
    append_cleanup_failure(&mut diagnostics, "shutdown", &cleanup.shutdown);
    append_cleanup_failure(&mut diagnostics, "close", &cleanup.close);
    append_cleanup_failure(&mut diagnostics, "renderer drop", &cleanup.renderer);
    (!diagnostics.is_empty()).then(|| diagnostics.join("; "))
}

fn include_cleanup_diagnostics(primary: String, cleanup: &CleanupOutcome) -> String {
    match cleanup_failure(cleanup) {
        Some(cleanup) => format!("{primary}; cleanup failed: {cleanup}"),
        None => primary,
    }
}

fn append_cleanup_failure(diagnostics: &mut Vec<String>, operation: &str, result: &CleanupResult) {
    if let CleanupResult::Failed { diagnostic } = result {
        diagnostics.push(format!("{operation}: {diagnostic}"));
    }
}

fn publish_failure(
    publisher: RenderOutputPublisher,
    primary: String,
    cleanup: CleanupOutcome,
) -> Result<(), RenderSceneError> {
    match publisher.finish_failure(PublishError::Execution(primary.clone()), cleanup) {
        Ok(()) => Err(RenderSceneError(primary)),
        Err(error) => Err(RenderSceneError(error.to_string())),
    }
}
