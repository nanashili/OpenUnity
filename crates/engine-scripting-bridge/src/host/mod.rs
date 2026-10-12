use crate::abi::{Create, CreateScene, CreateTimedScene, Destroy, Invoke};
use libloading::Library;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::rc::Rc;

mod creation;
mod entries;
mod loading;
mod session;

pub use creation::validate_time_source;
pub use session::ScriptSession;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HostMode {
    Script,
    Gameplay,
    Scene,
    TimedScene,
}

/// A process-resident runtime and its reviewed managed bridge.
///
/// Deliberately neither Send nor Sync: this first host dispatches from its owner
/// thread. The managed registry also serializes callbacks. Neither runtime unload
/// nor Unity reload semantics are promised.
pub struct ManagedHost {
    // Pin the host library through process exit, including error paths after init.
    // CoreCLR can remain loaded after hostfxr_close and retain associated code.
    _library: ManuallyDrop<Library>,
    create: Create,
    create_scene: Option<CreateScene>,
    create_timed_scene: Option<CreateTimedScene>,
    invoke: Invoke,
    destroy: Destroy,
    mode: HostMode,
    _owner_thread: PhantomData<Rc<()>>,
}
