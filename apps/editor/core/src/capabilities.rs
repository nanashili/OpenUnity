//! Capability admission for the independent editor defined by INFRA-EDITOR-001.
//!
//! Supported actions refer only to OpenUnity's independent design. They do not
//! establish Unity behavior, file-format compatibility, or UnityEditor API parity.
//! Admission does not execute an action or certify that a particular operation
//! (such as saving to a particular path) has completed successfully.

use std::fmt;

/// Actions exposed by the editor, including explicitly unavailable Unity features.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Capability {
    /// Create and edit the independent OpenUnity scene document.
    AuthoringScene,
    /// Discover project entries without importing, executing, or modifying them.
    ProjectDiscovery,
    /// Save a validated independent `.ouscene` document to an allowed location.
    SaveIndependentScene,
    /// Import only the reviewed saved-scene dialect with explicit script bindings.
    BoundedSavedSceneImport,
    /// Import a general Unity `.unity` scene into the editor.
    UnitySceneImport,
    /// Execute a game through Unity-compatible Play behavior.
    Play,
    /// Compile C# project scripts.
    CSharpCompilation,
    /// Build a player from an editor project.
    Build,
    /// Call UnityEditor APIs.
    UnityEditorApi,
}

impl Capability {
    /// Stable presentation order, with independent actions before unavailable features.
    pub const ALL: &'static [Self] = &[
        Self::AuthoringScene,
        Self::ProjectDiscovery,
        Self::SaveIndependentScene,
        Self::BoundedSavedSceneImport,
        Self::UnitySceneImport,
        Self::Play,
        Self::CSharpCompilation,
        Self::Build,
        Self::UnityEditorApi,
    ];

    /// An independently authored user-facing action label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AuthoringScene => "Independent scene authoring",
            Self::ProjectDiscovery => "Read-only project discovery",
            Self::SaveIndependentScene => "Save independent OpenUnity scene",
            Self::BoundedSavedSceneImport => "Import supported saved scene",
            Self::UnitySceneImport => "Unity scene import",
            Self::Play => "Play",
            Self::CSharpCompilation => "C# compilation",
            Self::Build => "Build",
            Self::UnityEditorApi => "UnityEditor APIs",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Whether an action is admitted by the independent editor contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Support {
    /// Independent design support only; the operation must still perform validation.
    Supported,
    /// Unavailable, with a user-facing explanation.
    Unsupported(&'static str),
}

/// An unavailable action and the reason that its attempt was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedFeature {
    /// The action that was attempted.
    pub capability: Capability,
    /// The reason the editor cannot perform the action.
    pub reason: &'static str,
}

impl fmt::Display for UnsupportedFeature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} is unavailable: {}",
            self.capability, self.reason
        )
    }
}

impl std::error::Error for UnsupportedFeature {}

/// List all actions in stable UI presentation order.
pub const fn list() -> &'static [Capability] {
    Capability::ALL
}

/// Describe support without executing or modifying anything.
pub const fn query(capability: Capability) -> Support {
    match capability {
        Capability::AuthoringScene
        | Capability::ProjectDiscovery
        | Capability::SaveIndependentScene
        | Capability::BoundedSavedSceneImport => Support::Supported,
        Capability::UnitySceneImport => Support::Unsupported(
            "General Unity scene import is unsupported; use the separately bounded saved-scene import with explicit bindings.",
        ),
        Capability::Play => Support::Unsupported(
            "Game execution is not implemented by the independent authoring editor; the viewport is for authoring only.",
        ),
        Capability::CSharpCompilation => Support::Unsupported(
            "C# project script compilation is not implemented by the independent editor.",
        ),
        Capability::Build => Support::Unsupported(
            "Building a player from an editor project is not implemented by the independent editor.",
        ),
        Capability::UnityEditorApi => Support::Unsupported(
            "UnityEditor API compatibility has no implemented reviewed contract; independent editor services do not provide those APIs.",
        ),
    }
}

/// Admit an independent action or reject an unsupported action with diagnostics.
///
/// `Ok(())` grants admission only. The caller must execute the actual action,
/// enforce its validation and write protections, and report its resulting errors.
/// This function never executes Play, compilation, import, or Build.
pub const fn attempt(capability: Capability) -> Result<(), UnsupportedFeature> {
    match query(capability) {
        Support::Supported => Ok(()),
        Support::Unsupported(reason) => Err(UnsupportedFeature { capability, reason }),
    }
}

#[cfg(test)]
mod tests {
    use super::{Capability, Support, attempt, query};

    #[test]
    fn unsupported_attempts_reject_with_action_and_reason() {
        for capability in [
            Capability::UnitySceneImport,
            Capability::Play,
            Capability::CSharpCompilation,
            Capability::Build,
            Capability::UnityEditorApi,
        ] {
            let error = attempt(capability).expect_err("unavailable actions must reject");
            assert_eq!(error.capability, capability);
            assert!(!error.reason.trim().is_empty());
            assert_eq!(query(capability), Support::Unsupported(error.reason));
            let diagnostic = error.to_string();
            assert!(diagnostic.contains(capability.label()));
            assert!(diagnostic.contains(error.reason));
        }
    }

    #[test]
    fn independent_actions_are_admitted_without_execution() {
        for capability in [
            Capability::AuthoringScene,
            Capability::ProjectDiscovery,
            Capability::SaveIndependentScene,
        ] {
            assert_eq!(attempt(capability), Ok(()));
        }
    }
}
