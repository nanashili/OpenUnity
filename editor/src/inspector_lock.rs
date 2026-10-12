//! Independent document-scoped Inspector view state. No native tracker is used.

use openunity_editor_core::scene::EntityId;

/// An identity is valid only within its owning document incarnation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InspectorTarget {
    /// Monotonic application document incarnation, independent of addresses.
    pub document_scope: u64,
    /// Existing canonical editor entity identity.
    pub entity_id: EntityId,
}

/// Committed view state delivered to lock observers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectorSnapshot {
    /// Whether selection-driven retargeting is suppressed.
    pub locked: bool,
    /// Current live displayed object, if supported.
    pub target: Option<InspectorTarget>,
    /// Owner of the actual panel draft.
    pub draft_target: Option<InspectorTarget>,
    /// Whether the panel contains unapplied changes.
    pub draft_changed: bool,
    /// Whether a lock activation is currently allowed.
    pub available: bool,
    /// Unavailable activation explanation.
    pub reason: Option<String>,
    /// A target invalidation notice, retained until the next accepted lock command.
    pub notice: Option<String>,
}

impl InspectorSnapshot {
    /// Read-only receipt representation of committed state.
    pub fn to_json(&self) -> serde_json::Value {
        fn target(value: Option<InspectorTarget>) -> serde_json::Value {
            value.map_or(serde_json::Value::Null, |t| serde_json::json!({"document_scope":t.document_scope,"entity_id":t.entity_id.0}))
        }
        serde_json::json!({"locked":self.locked,"target":target(self.target),"draft_target":target(self.draft_target),"draft_changed":self.draft_changed,"available":self.available,"reason":self.reason,"notice":self.notice})
    }
}

type Observer = Box<dyn FnMut(&InspectorSnapshot)>;

pub(crate) fn control_json(
    role: &str,
    response: &eframe::egui::Response,
    selected: bool,
) -> serde_json::Value {
    serde_json::json!({"role":role,"rect":crate::appearance::rect_json(response.rect),"widget_id":format!("{:?}",response.id),"enabled":response.enabled(),"selected":selected})
}

/// Production controller; callers supply current document liveness, never object pointers.
#[derive(Default)]
pub struct InspectorController {
    state: InspectorSnapshot,
    scope: Option<u64>,
    observers: Vec<Observer>,
    deferred: bool,
    pending_notifications: Vec<InspectorSnapshot>,
}

impl InspectorController {
    pub(crate) fn deferred() -> Self {
        Self {
            deferred: true,
            ..Self::default()
        }
    }
    /// Current committed state, including the target seen by observers.
    pub fn snapshot(&self) -> &InspectorSnapshot {
        &self.state
    }

    /// Observe changed lock booleans after target/draft reconciliation commits.
    pub fn observe(&mut self, observer: impl FnMut(&InspectorSnapshot) + 'static) {
        self.observers.push(Box::new(observer));
    }

    fn notify(&mut self) {
        if self.deferred {
            self.pending_notifications.push(self.state.clone());
            return;
        }
        for observer in &mut self.observers {
            observer(&self.state);
        }
    }

    pub(crate) fn deliver_notifications(&mut self) {
        for state in self.pending_notifications.drain(..) {
            for observer in &mut self.observers {
                observer(&state);
            }
        }
    }

    fn retarget(&mut self, target: Option<InspectorTarget>) {
        if self.state.target != target {
            self.state.target = target;
            self.state.draft_target = None;
            self.state.draft_changed = false;
        }
    }

    /// Reconcile a committed selection or document-validity notification.
    /// New document scopes reset silently; removal unlocks once after reconciliation.
    pub fn reconcile(
        &mut self,
        scope: u64,
        selection: Option<InspectorTarget>,
        live: impl Fn(InspectorTarget) -> bool,
    ) {
        let selected = selection.filter(|t| t.document_scope == scope && live(*t));
        if self.scope != Some(scope) {
            self.scope = Some(scope);
            self.state = InspectorSnapshot::default();
            self.retarget(selected);
        } else if self
            .state
            .target
            .is_some_and(|t| t.document_scope != scope || !live(t))
        {
            let was_locked = self.state.locked;
            self.state.locked = false;
            self.retarget(None);
            self.retarget(selected);
            self.state.notice = Some("Pinned Inspector target unavailable".into());
            self.update_availability();
            if was_locked {
                self.notify();
            }
            return;
        } else if !self.state.locked {
            self.retarget(selected);
        }
        self.update_availability();
    }

    fn update_availability(&mut self) {
        self.state.available = self.state.target.is_some();
        self.state.reason = (!self.state.available).then(|| "Inspector target unavailable".into());
    }

    /// Set actual panel draft ownership after rendering or draft reconciliation.
    pub(crate) fn draft(&mut self, owner: Option<InspectorTarget>, changed: bool) {
        self.state.draft_target = owner.filter(|t| Some(*t) == self.state.target);
        self.state.draft_changed = self.state.draft_target.is_some() && changed;
    }

    /// Record a panel draft only if it belongs to the displayed target.
    pub fn set_draft_state(
        &mut self,
        owner: Option<InspectorTarget>,
        changed: bool,
    ) -> Result<(), String> {
        if owner.is_some() && owner != self.state.target {
            return Err("Inspector draft owner is not the displayed target.".into());
        }
        self.draft(owner, changed);
        Ok(())
    }

    /// Validate an explicitly addressed edit against the current document incarnation.
    pub fn validate_target(
        &self,
        target: InspectorTarget,
        scope: u64,
        live: impl Fn(InspectorTarget) -> bool,
    ) -> Result<(), String> {
        if target.document_scope != scope || self.scope != Some(scope) || !live(target) {
            return Err("Inspector target no longer valid".into());
        }
        Ok(())
    }

    /// Execute a lock command with current selection and validity, committing before notification.
    pub fn set_lock(
        &mut self,
        locked: bool,
        scope: u64,
        selection: Option<InspectorTarget>,
        live: impl Fn(InspectorTarget) -> bool,
    ) -> Result<(), String> {
        if self.state.locked == locked {
            return Ok(());
        }
        let target = self
            .state
            .target
            .ok_or_else(|| "Inspector target unavailable".to_owned())?;
        self.validate_target(target, scope, &live)
            .map_err(|_| "Inspector target unavailable".to_owned())?;
        if !self.state.available {
            return Err(self.state.reason.clone().unwrap_or_default());
        }
        self.state.locked = locked;
        if !locked {
            self.retarget(selection.filter(|t| t.document_scope == scope && live(*t)));
        }
        self.state.notice = None;
        self.update_availability();
        self.notify();
        Ok(())
    }
}
