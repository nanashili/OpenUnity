//! Private lifecycle records shared by the owning lifecycle modules.

use engine_object::{ObjectId, Registry};
use std::collections::{BTreeMap, VecDeque};

/// A copied callback delivery; no runtime storage or managed pointer escapes.
#[derive(Clone, Copy, Debug)]
pub struct LifecycleEvent {
    /// Monotonic, once-only acknowledgement token.
    pub token: i64,
    /// Attached script identity.
    pub component: ObjectId,
    /// Event code: Awake=1 through OnDestroy=7, FixedUpdate=8, hierarchy=9–10.
    pub event: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct Script {
    pub(in crate::lifecycle) owner: ObjectId,
    pub(in crate::lifecycle) order: i32,
    pub(in crate::lifecycle) committed: bool,
    pub(in crate::lifecycle) enabled: bool,
    pub(in crate::lifecycle) awakened: bool,
    pub(in crate::lifecycle) started: bool,
    pub(in crate::lifecycle) notified_enabled: bool,
    pub(in crate::lifecycle) in_flight: u32,
    pub(in crate::lifecycle) pending_destroy: bool,
    pub(in crate::lifecycle) notified_destroy: bool,
}

#[derive(Debug)]
pub(in crate::lifecycle) struct DestroyTarget {
    pub(in crate::lifecycle) id: ObjectId,
    pub(in crate::lifecycle) prepared: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Lifecycle {
    pub(crate) scripts: BTreeMap<i64, Script>,
    pub(in crate::lifecycle) deliveries: Vec<LifecycleEvent>,
    pub(in crate::lifecycle) hierarchy_notifications: VecDeque<(ObjectId, i32)>,
    pub(in crate::lifecycle) tokens: Registry<()>,
    pub(in crate::lifecycle) frame_count: i32,
    pub(in crate::lifecycle) frame_started: bool,
    pub(in crate::lifecycle) phase: i32,
    pub(in crate::lifecycle) phase_drained: bool,
    pub(in crate::lifecycle) snapshot: Vec<ObjectId>,
    pub(in crate::lifecycle) cursor: usize,
    pub(in crate::lifecycle) standalone_destroy: bool,
    pub(in crate::lifecycle) pending_destroy: Vec<DestroyTarget>,
    pub(in crate::lifecycle) faulted: bool,
}
