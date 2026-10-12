use super::entries::ManagedEntries;
use super::{HostMode, ManagedHost};
use crate::HostError;
use crate::hostfxr::load_runtime;
use crate::platform::HostPaths;
use std::marker::PhantomData;
use std::path::Path;

impl ManagedHost {
    /// Initialize hostfxr and obtain the four independently specified delegates.
    ///
    /// # Safety
    /// `hostfxr` must be a trusted .NET host library implementing the public
    /// hostfxr ABI. `bridge` must be the reviewed OpenUnity.ManagedHosting assembly
    /// implementing INFRA-HOST-001 with exactly its documented native signatures.
    /// Loading arbitrary native code or mismatched managed signatures is unsafe.
    pub unsafe fn load(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's trusted host and bridge guarantees.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::Script,
            )
        }
    }

    /// Load the independently versioned MonoBehaviour gameplay controller.
    /// Legacy hosted-script clients do not resolve this additional capability.
    ///
    /// # Safety
    /// The paths must satisfy [`Self::load`]'s guarantees. The managed bridge
    /// must additionally implement the exact INFRA-LIFECYCLE-001 Cdecl
    /// GameplayVersion and CreateGameplay signatures.
    pub unsafe fn load_gameplay(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's additional reviewed gameplay ABI guarantee.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::Gameplay,
            )
        }
    }

    /// Load the independently versioned saved-scene controller.
    ///
    /// # Safety
    /// Paths must satisfy [`Self::load`] and the bridge must implement the exact
    /// INFRA-SCENE-001 SceneVersion and three-argument CreateScene signatures.
    pub unsafe fn load_scene(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's reviewed scene ABI guarantees.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::Scene,
            )
        }
    }

    /// Load the independently versioned timed saved-scene controller.
    ///
    /// # Safety
    /// Paths must satisfy [`Self::load`]. The bridge must additionally implement
    /// INFRA-TIME-001 TimedSceneVersion and the exact five-argument CreateTimedScene ABI.
    pub unsafe fn load_timed_scene(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's reviewed timed-scene signature guarantee.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::TimedScene,
            )
        }
    }

    /// Load a saved-scene controller and require the additive snapshot capability.
    ///
    /// # Safety
    /// Paths must satisfy [`Self::load`]. The bridge must implement the exact
    /// INFRA-SCENE-SNAPSHOT-001 version and capture signatures in addition to
    /// the saved-scene ABI.
    pub unsafe fn load_scene_with_snapshot(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's reviewed snapshot ABI guarantees.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::SceneSnapshot,
            )
        }
    }

    /// Load a timed scene controller and require the additive snapshot capability.
    ///
    /// # Safety
    /// Paths must satisfy [`Self::load`]. The bridge must implement the exact
    /// INFRA-SCENE-SNAPSHOT-001 version and capture signatures in addition to
    /// the timed-scene ABI.
    pub unsafe fn load_timed_scene_with_snapshot(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's reviewed timed-scene snapshot ABI guarantees.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::TimedSceneSnapshot,
            )
        }
    }

    unsafe fn load_mode(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
        mode: HostMode,
    ) -> Result<Self, HostError> {
        let paths = HostPaths::validate(dotnet_root, hostfxr, bridge, runtime_config)?;
        // SAFETY: caller guarantees trusted hostfxr and reviewed managed signatures.
        let (library, load) = unsafe { load_runtime(hostfxr, &paths) }?;
        // SAFETY: the same caller guarantee covers every resolved managed entrypoint.
        let entries = unsafe { ManagedEntries::resolve(load, &paths, mode) }?;
        Ok(Self {
            _library: library,
            create: entries.create,
            create_scene: entries.create_scene,
            create_timed_scene: entries.create_timed_scene,
            invoke: entries.invoke,
            destroy: entries.destroy,
            scene_snapshot: entries.scene_snapshot,
            mode,
            _owner_thread: PhantomData,
        })
    }
}
