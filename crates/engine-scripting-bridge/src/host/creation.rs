use super::{HostMode, ManagedHost, ScriptSession};
use crate::HostError;
use crate::platform::absolute_file;
use std::ffi::CString;
use std::path::Path;

impl ManagedHost {
    /// Construct a script object implementing the bridge's IHostedScript contract.
    pub fn create_script(
        &self,
        assembly: &Path,
        type_name: &str,
    ) -> Result<ScriptSession<'_>, HostError> {
        if self.mode != HostMode::Script {
            return Err(HostError::setup(
                "create script",
                "host was loaded for another controller",
            ));
        }
        self.create_session(assembly, type_name)
    }

    /// Construct a controller for an ordinary MonoBehaviour gameplay type.
    /// Initialization attaches it to a native-owned root. Requires a host
    /// constructed by [`Self::load_gameplay`].
    pub fn create_gameplay(
        &self,
        assembly: &Path,
        type_name: &str,
    ) -> Result<ScriptSession<'_>, HostError> {
        if self.mode != HostMode::Gameplay {
            return Err(HostError::setup(
                "create gameplay",
                "gameplay capability was not loaded",
            ));
        }
        self.create_session(assembly, type_name)
    }

    /// Create a saved-scene controller from an immutable UTF-8 transfer plan.
    /// Reflection and native graph validation occur before native publication.
    pub fn create_scene(
        &self,
        assembly: &Path,
        plan: &[u8],
    ) -> Result<ScriptSession<'_>, HostError> {
        let create = self
            .create_scene
            .ok_or_else(|| HostError::setup("create scene", "scene capability was not loaded"))?;
        absolute_file(assembly)?;
        if plan.is_empty() || plan.len() > 16 * 1024 * 1024 {
            return Err(HostError::setup("create scene", "plan byte limit"));
        }
        let path = CString::new(
            assembly
                .to_str()
                .ok_or_else(|| HostError::setup("scene assembly", "nonUnicode path"))?,
        )
        .map_err(|e| HostError::setup("scene assembly", e))?;
        // SAFETY: trusted exact entrypoint; both byte buffers live through the call.
        let id = unsafe { create(path.as_ptr().cast(), plan.as_ptr(), plan.len() as i32) };
        if id <= 0 {
            return Err(HostError::status(
                "create scene",
                i32::try_from(id).unwrap_or(-6),
            ));
        }
        Ok(ScriptSession::new(self, id))
    }

    /// Create a timed scene using an explicitly selected native elapsed source.
    /// Source zero requires step zero; source one requires a finite 1us..60s step.
    pub fn create_timed_scene(
        &self,
        assembly: &Path,
        plan: &[u8],
        source: i32,
        step: f64,
    ) -> Result<ScriptSession<'_>, HostError> {
        let create = self.create_timed_scene.ok_or_else(|| {
            HostError::setup(
                "create timed scene",
                "timed scene capability was not loaded",
            )
        })?;
        validate_time_source(source, step)?;
        absolute_file(assembly)?;
        if plan.is_empty() || plan.len() > 16 * 1024 * 1024 {
            return Err(HostError::setup("create timed scene", "plan byte limit"));
        }
        let path = CString::new(
            assembly
                .to_str()
                .ok_or_else(|| HostError::setup("timed scene assembly", "nonUnicode path"))?,
        )
        .map_err(|error| HostError::setup("timed scene assembly", error))?;
        // SAFETY: trusted exact entrypoint and live immutable buffers for this call.
        let id = unsafe {
            create(
                path.as_ptr().cast(),
                plan.as_ptr(),
                plan.len() as i32,
                source,
                step,
            )
        };
        if id <= 0 {
            return Err(HostError::status(
                "create timed scene",
                i32::try_from(id).unwrap_or(-6),
            ));
        }
        Ok(ScriptSession::new(self, id))
    }

    fn create_session(
        &self,
        assembly: &Path,
        type_name: &str,
    ) -> Result<ScriptSession<'_>, HostError> {
        absolute_file(assembly)?;
        let path = CString::new(
            assembly
                .to_str()
                .ok_or_else(|| HostError::setup("script path", "path is not Unicode"))?,
        )
        .map_err(|e| HostError::setup("script path", e))?;
        let name = CString::new(type_name).map_err(|e| HostError::setup("script type", e))?;
        // SAFETY: bridge is trusted; both UTF-8 CStrings remain valid for the call.
        let id = unsafe { (self.create)(path.as_ptr().cast(), name.as_ptr().cast()) };
        if id <= 0 {
            return Err(HostError::status(
                if self.mode == HostMode::Gameplay {
                    "create gameplay"
                } else {
                    "create script"
                },
                i32::try_from(id).unwrap_or(-6),
            ));
        }
        Ok(ScriptSession::new(self, id))
    }
}

/// Validate the reviewed source configuration before import or host creation.
pub fn validate_time_source(source: i32, step: f64) -> Result<(), HostError> {
    let valid = step.is_finite()
        && match source {
            0 => step == 0.0,
            1 => (0.000001..=60.0).contains(&step),
            _ => false,
        };
    if !valid {
        return Err(HostError::setup(
            "time source",
            "invalid source or elapsed step",
        ));
    }
    Ok(())
}
