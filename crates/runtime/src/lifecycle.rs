//! Native ownership and callback eligibility for INFRA-LIFECYCLE-001.

use super::{ObjectKind, ObjectRecord, Runtime, RuntimeError};
use openunity_object_model::{ObjectId, Registry};
use std::collections::BTreeMap;

/// A copied callback delivery; no runtime storage or managed pointer escapes.
#[derive(Clone, Copy, Debug)]
pub struct LifecycleEvent {
    /// Monotonic, once-only acknowledgement token.
    pub token: i64,
    /// Attached script identity.
    pub component: ObjectId,
    /// Reviewed event code, Awake=1 through OnDestroy=7, additive FixedUpdate=8.
    pub event: i32,
}

#[derive(Clone, Debug)]
pub(super) struct Script {
    owner: ObjectId,
    order: i32,
    committed: bool,
    enabled: bool,
    awakened: bool,
    started: bool,
    notified_enabled: bool,
    in_flight: bool,
    pending_destroy: bool,
    notified_destroy: bool,
}

#[derive(Debug)]
struct DestroyTarget {
    id: ObjectId,
    prepared: bool,
}

/// Fully validated removal work. No callback or fallible hierarchy lookup occurs
/// between planning this batch and committing it.
#[derive(Debug)]
struct DestructionPlan {
    targets: Vec<PlannedDestruction>,
}

#[derive(Debug)]
enum PlannedDestruction {
    Script(ObjectId),
    Plain(ObjectId),
    Subtree {
        root_owner: ObjectId,
        root_transform: ObjectId,
        parent: Option<ObjectId>,
        nodes: Vec<DestructionNode>,
    },
}

#[derive(Debug)]
struct DestructionNode {
    transform: ObjectId,
    owner: ObjectId,
    scripts: Vec<ObjectId>,
}

#[derive(Debug, Default)]
pub(super) struct Lifecycle {
    pub(super) scripts: BTreeMap<i64, Script>,
    deliveries: Vec<LifecycleEvent>,
    tokens: Registry<()>,
    frame_count: i32,
    frame_started: bool,
    phase: i32,
    phase_drained: bool,
    snapshot: Vec<ObjectId>,
    cursor: usize,
    standalone_destroy: bool,
    pending_destroy: Vec<DestroyTarget>,
    faulted: bool,
}

impl Runtime {
    fn lifecycle_ready(&self) -> Result<(), RuntimeError> {
        if self.lifecycle.faulted {
            Err(RuntimeError::DispatchFaulted)
        } else {
            Ok(())
        }
    }

    pub(super) fn scene_lifecycle_ready(&self, healthy: bool) -> Result<(), RuntimeError> {
        if healthy {
            self.lifecycle_ready()?;
        }
        if (healthy && !self.frame_idle()) || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }

    pub(super) fn scene_owned_scripts(
        &self,
        owners: &std::collections::BTreeSet<i64>,
    ) -> Vec<ObjectId> {
        self.lifecycle
            .scripts
            .iter()
            .filter_map(|(&raw, script)| {
                owners
                    .contains(&script.owner.to_raw())
                    .then(|| ObjectId::from_raw(raw))
                    .flatten()
            })
            .collect()
    }

    pub(super) fn scene_release_lifecycle(&mut self, ids: &std::collections::BTreeSet<i64>) {
        self.lifecycle
            .scripts
            .retain(|id, script| !ids.contains(id) && !ids.contains(&script.owner.to_raw()));
        let cursor = self.lifecycle.snapshot
            [..self.lifecycle.cursor.min(self.lifecycle.snapshot.len())]
            .iter()
            .filter(|id| !ids.contains(&id.to_raw()))
            .count();
        self.lifecycle
            .snapshot
            .retain(|id| !ids.contains(&id.to_raw()));
        self.lifecycle.cursor = cursor;
        self.lifecycle
            .pending_destroy
            .retain(|target| !ids.contains(&target.id.to_raw()));
    }

    fn script(&self, id: ObjectId) -> Result<&Script, RuntimeError> {
        match self
            .objects
            .get(id)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            ObjectKind::Script { .. } => self
                .lifecycle
                .scripts
                .get(&id.to_raw())
                .ok_or(RuntimeError::InvalidObject),
            _ => Err(RuntimeError::InvalidKind),
        }
    }

    fn committed_script(&self, id: ObjectId) -> Result<&Script, RuntimeError> {
        let script = self.script(id)?;
        if script.committed {
            Ok(script)
        } else {
            Err(RuntimeError::Lifecycle)
        }
    }

    fn owner_scripts(&self, owner: ObjectId) -> Result<&[ObjectId], RuntimeError> {
        match &self
            .objects
            .get(owner)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            ObjectKind::GameObject { scripts, .. } => Ok(scripts),
            _ => Err(RuntimeError::InvalidKind),
        }
    }

    pub(super) fn subtree_has_scripts(&self, root: ObjectId) -> Result<bool, RuntimeError> {
        let mut pending = vec![root];
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?;
            if !self.owner_scripts(data.owner)?.is_empty()
                || self
                    .lifecycle
                    .scripts
                    .values()
                    .any(|script| script.owner == data.owner)
            {
                return Ok(true);
            }
            pending.extend(data.children.iter().copied());
        }
        Ok(false)
    }

    fn owner_pending_destroy(&self, owner: ObjectId) -> Result<bool, RuntimeError> {
        let mut current = Some(self.gameobject_transform(owner)?);
        while let Some(transform) = current {
            let data = self.transform(transform)?;
            if self
                .lifecycle
                .pending_destroy
                .iter()
                .any(|target| target.id == data.owner)
            {
                return Ok(true);
            }
            current = data.parent;
        }
        Ok(false)
    }

    /// Reserve a hidden script identity before managed construction.
    /// # Errors
    /// Rejects a foreign/wrong/pending owner, faulted context, or exhausted identities.
    pub fn component_reserve(
        &mut self,
        owner: ObjectId,
        order: i32,
    ) -> Result<ObjectId, RuntimeError> {
        self.owner_scripts(owner)?;
        self.lifecycle_ready()?;
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        let id = self
            .objects
            .insert(ObjectRecord {
                name: String::new(),
                kind: ObjectKind::Script { owner },
            })
            .map_err(|_| RuntimeError::IdentityExhausted)?;
        self.lifecycle.scripts.insert(
            id.to_raw(),
            Script {
                owner,
                order,
                committed: false,
                enabled: true,
                awakened: false,
                started: false,
                notified_enabled: false,
                in_flight: false,
                pending_destroy: false,
                notified_destroy: false,
            },
        );
        Ok(id)
    }

    /// Publish a successfully constructed reservation in owner insertion order.
    /// # Errors
    /// Rejects invalid/committed reservations, pending owners, and faulted dispatch.
    pub fn component_commit(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        let script = self.script(id)?;
        let owner = script.owner;
        if script.committed || self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle_ready()?;
        if let ObjectKind::GameObject { scripts, .. } = &mut self
            .objects
            .get_mut(owner)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            scripts.push(id);
        }
        self.lifecycle
            .scripts
            .get_mut(&id.to_raw())
            .ok_or(RuntimeError::InvalidObject)?
            .committed = true;
        Ok(())
    }

    /// Consume an unpublished reservation after a constructor fails.
    /// # Errors
    /// Rejects absent, wrong-kind, or already committed identities.
    pub fn component_abort(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        if self.script(id)?.committed {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle.scripts.remove(&id.to_raw());
        self.objects.remove(id);
        Ok(())
    }

    /// Return mandatory Transform plus committed scripts.
    /// # Errors
    /// Rejects absent/wrong owners and unrepresentable counts.
    pub fn gameobject_component_count(&self, owner: ObjectId) -> Result<i32, RuntimeError> {
        i32::try_from(self.owner_scripts(owner)?.len() + 1).map_err(|_| RuntimeError::CountOverflow)
    }

    /// Resolve the Transform-first native component enumeration.
    /// # Errors
    /// Rejects absent/wrong owners or an invalid index.
    pub fn gameobject_component_at(
        &self,
        owner: ObjectId,
        index: i32,
    ) -> Result<ObjectId, RuntimeError> {
        let scripts = self.owner_scripts(owner)?;
        if index < 0 {
            return Err(RuntimeError::InvalidKind);
        }
        if index == 0 {
            self.gameobject_transform(owner)
        } else {
            usize::try_from(index - 1)
                .ok()
                .and_then(|index| scripts.get(index).copied())
                .ok_or(RuntimeError::InvalidKind)
        }
    }

    /// Read native self or hierarchy activity.
    /// # Errors
    /// Rejects an absent or wrong-kind owner.
    pub fn gameobject_active(
        &self,
        owner: ObjectId,
        hierarchy: bool,
    ) -> Result<bool, RuntimeError> {
        let mut current = Some(self.gameobject_transform(owner)?);
        while let Some(transform) = current {
            let data = self.transform(transform)?;
            match self
                .objects
                .get(data.owner)
                .ok_or(RuntimeError::InvalidObject)?
                .kind
            {
                ObjectKind::GameObject {
                    active_self: false, ..
                } => return Ok(false),
                ObjectKind::GameObject { .. } => {}
                _ => return Err(RuntimeError::InvalidKind),
            }
            current = if hierarchy { data.parent } else { None };
        }
        Ok(true)
    }

    /// Change activity; callback eligibility is reevaluated by the mutation pump.
    /// # Errors
    /// Rejects absent/wrong owners, pending destruction, or faulted dispatch.
    pub fn gameobject_set_active(
        &mut self,
        owner: ObjectId,
        value: bool,
    ) -> Result<(), RuntimeError> {
        self.owner_scripts(owner)?;
        self.lifecycle_ready()?;
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        if let ObjectKind::GameObject { active_self, .. } = &mut self
            .objects
            .get_mut(owner)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            *active_self = value;
        }
        Ok(())
    }

    /// Read the native enabled flag for a committed behaviour.
    /// # Errors
    /// Rejects absent/wrong or unpublished scripts.
    pub fn behaviour_enabled(&self, id: ObjectId) -> Result<bool, RuntimeError> {
        Ok(self.committed_script(id)?.enabled)
    }

    /// Change a committed script's enabled flag.
    /// # Errors
    /// Rejects absent/wrong/unpublished/pending scripts and faulted dispatch.
    pub fn behaviour_set_enabled(&mut self, id: ObjectId, value: bool) -> Result<(), RuntimeError> {
        if self.committed_script(id)?.pending_destroy {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle_ready()?;
        self.lifecycle
            .scripts
            .get_mut(&id.to_raw())
            .ok_or(RuntimeError::InvalidObject)?
            .enabled = value;
        Ok(())
    }

    /// Read effective native behaviour activity.
    /// # Errors
    /// Rejects absent/wrong or unpublished scripts.
    pub fn behaviour_active(&self, id: ObjectId) -> Result<bool, RuntimeError> {
        let script = self.committed_script(id)?;
        Ok(script.awakened
            && script.enabled
            && !script.pending_destroy
            && self.gameobject_active(script.owner, true)?)
    }

    /// Read the actual per-context frame counter, initially zero.
    pub fn lifecycle_frame_count(&self) -> i32 {
        self.lifecycle.frame_count
    }

    fn frame_idle(&self) -> bool {
        !self.lifecycle.frame_started || (self.lifecycle.phase == 5 && self.lifecycle.phase_drained)
    }

    /// Increment the native frame counter and prepare the five ordered phases.
    /// # Errors
    /// Rejects unfinished/reentrant frames, faults, and counter overflow without mutation.
    pub fn lifecycle_begin_frame(&mut self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if self.clock.enabled() {
            return Err(RuntimeError::Lifecycle);
        }
        let count = self.next_frame_count()?;
        self.commit_frame(count);
        Ok(())
    }

    fn next_frame_count(&self) -> Result<i32, RuntimeError> {
        self.lifecycle_ready()?;
        if !self.frame_idle() || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle
            .frame_count
            .checked_add(1)
            .ok_or(RuntimeError::CountOverflow)
    }

    fn commit_frame(&mut self, count: i32) {
        self.lifecycle.frame_count = count;
        self.lifecycle.frame_started = true;
        self.lifecycle.phase = 0;
        self.lifecycle.phase_drained = true;
        self.lifecycle.standalone_destroy = false;
    }

    /// Configure the additive native clock before any frame has run.
    /// # Errors
    /// Rejects invalid scalars, staging, faults, used contexts and reconfiguration.
    pub fn time_configure(&mut self, source: i32, step: f64) -> Result<(), RuntimeError> {
        super::validate_time_configuration(source, step)?;
        self.time_ready()?;
        if self.lifecycle.frame_count != 0
            || !self.frame_idle()
            || !self.lifecycle.deliveries.is_empty()
        {
            return Err(RuntimeError::Lifecycle);
        }
        self.clock.configure(source, step)
    }

    fn time_ready(&self) -> Result<(), RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        self.lifecycle_ready()
    }

    /// Update one configured setting without changing an already published frame.
    /// # Errors
    /// Rejects invalid scalars, staging and faulted dispatch without mutation.
    pub fn time_set(&mut self, property: i32, value: f32) -> Result<(), RuntimeError> {
        super::validate_time_property(property, value)?;
        self.time_ready()?;
        self.clock.set(property, value)
    }

    /// Copy timing values without exposing native storage.
    /// # Errors
    /// Rejects scene staging or faulted dispatch.
    pub fn time_snapshot(&self) -> Result<super::TimeSnapshot, RuntimeError> {
        self.time_ready()?;
        Ok(self.clock.snapshot(self.lifecycle.frame_count))
    }

    /// Atomically publish one timed frame and increment the actual native counter.
    /// # Errors
    /// Rejects unfinished frames, clock arithmetic failure, faults or count overflow.
    pub fn time_begin_frame(&mut self) -> Result<(), RuntimeError> {
        self.time_ready()?;
        let count = self.next_frame_count()?;
        let clock = self.clock.frame_candidate()?;
        self.clock = clock;
        self.commit_frame(count);
        Ok(())
    }

    /// Install one due fixed iteration after the first Start phase has drained.
    /// # Errors
    /// Rejects invalid order, reentry, arithmetic failure or an excessive step budget.
    pub fn time_begin_fixed(&mut self) -> Result<bool, RuntimeError> {
        self.time_ready()?;
        if !self.lifecycle.frame_started
            || self.lifecycle.phase != 1
            || !self.lifecycle.phase_drained
            || !self.lifecycle.deliveries.is_empty()
            || self.lifecycle.standalone_destroy
        {
            return Err(RuntimeError::Lifecycle);
        }
        let (clock, active) = self.clock.fixed_candidate()?;
        let snapshot = if active {
            self.ordered_eligible(true)
        } else {
            Vec::new()
        };
        self.clock = clock;
        self.lifecycle.snapshot = snapshot;
        self.lifecycle.cursor = 0;
        Ok(active)
    }

    /// Begin the next phase: Start, Update, LateUpdate, Start, destruction.
    /// # Errors
    /// Rejects invalid, reentrant, out-of-order, or undrained phases and faults.
    pub fn lifecycle_begin_phase(&mut self, phase: i32) -> Result<(), RuntimeError> {
        if !(1..=5).contains(&phase) {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle_ready()?;
        if !self.lifecycle.frame_started
            || !self.lifecycle.deliveries.is_empty()
            || !self.lifecycle.phase_drained
            || phase != self.lifecycle.phase + 1
            || self.lifecycle.standalone_destroy
            || (self.clock.enabled() && phase == 2 && !self.clock.fixed_complete())
        {
            return Err(RuntimeError::Lifecycle);
        }
        let snapshot = if phase == 2 || phase == 3 {
            self.ordered_eligible(true)
        } else {
            Vec::new()
        };
        self.lifecycle.phase = phase;
        self.lifecycle.phase_drained = false;
        self.lifecycle.snapshot = snapshot;
        self.lifecycle.cursor = 0;
        Ok(())
    }

    /// Enter an explicit destruction-only boundary without changing Time.
    /// # Errors
    /// Rejects an unfinished/reentrant frame or faulted dispatch.
    pub fn lifecycle_flush_destroy(&mut self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if !self.frame_idle() || !self.lifecycle.deliveries.is_empty() {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle.phase = 5;
        self.lifecycle.phase_drained = false;
        self.lifecycle.frame_started = true;
        self.lifecycle.standalone_destroy = true;
        Ok(())
    }

    fn ordered_eligible(&self, started: bool) -> Vec<ObjectId> {
        let mut scripts: Vec<_> = self
            .lifecycle
            .scripts
            .iter()
            .filter_map(|(&raw, script)| {
                (script.committed
                    && script.awakened
                    && script.started == started
                    && script.notified_enabled
                    && script.enabled
                    && !script.pending_destroy
                    && !script.in_flight
                    && self.gameobject_active(script.owner, true).unwrap_or(false))
                .then_some((script.order, raw))
            })
            .collect();
        scripts.sort_unstable();
        scripts
            .into_iter()
            .filter_map(|(_, raw)| ObjectId::from_raw(raw))
            .collect()
    }

    fn mutation_event(&self) -> Option<(ObjectId, i32)> {
        self.lifecycle
            .scripts
            .iter()
            .filter_map(|(&raw, script)| {
                if !script.committed || script.in_flight {
                    return None;
                }
                let owner_active = self.gameobject_active(script.owner, true).unwrap_or(false);
                let active = owner_active && script.enabled && !script.pending_destroy;
                let event = if script.notified_enabled && !active {
                    6
                } else if !script.pending_destroy && !script.awakened && owner_active {
                    1
                } else if script.awakened && !script.notified_enabled && active {
                    2
                } else {
                    return None;
                };
                Some((script.order, raw, event))
            })
            .min_by_key(|&(order, raw, _)| (order, raw))
            .and_then(|(_, raw, event)| Some((ObjectId::from_raw(raw)?, event)))
    }

    fn prepare_destruction(&mut self) -> Result<(), RuntimeError> {
        let roots: Vec<_> = self
            .lifecycle
            .pending_destroy
            .iter()
            .filter(|target| !target.prepared)
            .map(|target| target.id)
            .collect();
        if roots.is_empty() {
            return Ok(());
        }

        // Requests suppress the scripts present at request time. Final ownership
        // follows the live hierarchy: escaped scripts survive, incoming scripts
        // are destroyed, and an explicitly requested component stays queued.
        // Resolve every membership before changing flags or owner activity.
        let membership: Vec<_> = self
            .lifecycle
            .scripts
            .iter()
            .map(|(&raw, script)| {
                let explicitly_requested = self
                    .lifecycle
                    .pending_destroy
                    .iter()
                    .any(|target| target.id.to_raw() == raw);
                let destroying =
                    explicitly_requested || self.owner_pending_destroy(script.owner)?;
                Ok((raw, destroying))
            })
            .collect::<Result<_, RuntimeError>>()?;

        for (raw, destroying) in membership {
            let script = self
                .lifecycle
                .scripts
                .get_mut(&raw)
                .ok_or(RuntimeError::InvalidObject)?;
            script.pending_destroy = destroying;
            // Do not change enabled here. Escaped scripts retain the request's
            // disabled value; newly included scripts retain their own value.
        }
        for root in roots {
            if let Some(ObjectRecord {
                kind: ObjectKind::GameObject { active_self, .. },
                ..
            }) = self.objects.get_mut(root)
            {
                *active_self = false;
            }
        }
        for target in &mut self.lifecycle.pending_destroy {
            target.prepared = true;
        }
        Ok(())
    }

    fn phase_event(&mut self) -> Result<Option<(ObjectId, i32)>, RuntimeError> {
        match self.lifecycle.phase {
            1 | 4 => Ok(self
                .ordered_eligible(false)
                .first()
                .copied()
                .map(|id| (id, 3))),
            2 | 3 => Ok(self.snapshot_event(self.lifecycle.phase + 2)),
            5 => Ok(self.lifecycle.scripts.iter().find_map(|(&raw, script)| {
                (script.committed
                    && script.pending_destroy
                    && script.awakened
                    && !script.notified_destroy
                    && !script.in_flight)
                    .then(|| ObjectId::from_raw(raw).map(|id| (id, 7)))
                    .flatten()
            })),
            _ => Err(RuntimeError::Lifecycle),
        }
    }

    fn snapshot_event(&mut self, event: i32) -> Option<(ObjectId, i32)> {
        while let Some(&id) = self.lifecycle.snapshot.get(self.lifecycle.cursor) {
            self.lifecycle.cursor += 1;
            if self
                .committed_script(id)
                .is_ok_and(|script| script.started && script.notified_enabled && !script.in_flight)
                && self.behaviour_active(id).unwrap_or(false)
            {
                return Some((id, event));
            }
        }
        None
    }

    fn validate_delivery_request(&self, mode: i32) -> Result<(), RuntimeError> {
        if !(0..=1).contains(&mode) {
            return Err(RuntimeError::InvalidKind);
        }
        self.lifecycle_ready()?;
        if mode == 1
            && (!self.lifecycle.deliveries.is_empty()
                || !self.lifecycle.frame_started
                || self.lifecycle.phase == 0)
        {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }

    fn next_callback(&mut self, mode: i32) -> Result<Option<(ObjectId, i32)>, RuntimeError> {
        if mode == 1 && self.lifecycle.phase == 5 {
            // Incoming enabled scripts must disable before their destruction.
            self.prepare_destruction()?;
        }
        if let Some(event) = self.mutation_event() {
            return Ok(Some(event));
        }
        if mode == 1 {
            if self.clock.fixed_active() {
                return Ok(self.snapshot_event(8));
            }
            self.phase_event()
        } else {
            Ok(None)
        }
    }

    fn complete_drained_phase(&mut self) -> Result<(), RuntimeError> {
        if self.clock.fixed_active() {
            return self.clock.finish_fixed();
        }
        if self.lifecycle.phase == 5 {
            self.finalize_destruction()?;
        }
        self.lifecycle.phase_drained = true;
        if self.lifecycle.standalone_destroy {
            self.lifecycle.frame_started = false;
            self.lifecycle.standalone_destroy = false;
        }
        Ok(())
    }

    fn commit_delivery(
        &mut self,
        token: ObjectId,
        component: ObjectId,
        event: i32,
    ) -> Result<LifecycleEvent, RuntimeError> {
        let script = self
            .lifecycle
            .scripts
            .get_mut(&component.to_raw())
            .ok_or(RuntimeError::InvalidObject)?;
        match event {
            1 => script.awakened = true,
            2 => script.notified_enabled = true,
            3 => script.started = true,
            6 => script.notified_enabled = false,
            7 => script.notified_destroy = true,
            _ => {}
        }
        script.in_flight = true;
        let delivery = LifecycleEvent {
            token: token.to_raw(),
            component,
            event,
        };
        self.lifecycle.deliveries.push(delivery);
        Ok(delivery)
    }

    fn deliver_reserved_token(
        &mut self,
        mode: i32,
        token: ObjectId,
    ) -> Result<Option<LifecycleEvent>, RuntimeError> {
        let Some((component, event)) = self.next_callback(mode)? else {
            if mode == 1 {
                self.complete_drained_phase()?;
            }
            return Ok(None);
        };
        self.commit_delivery(token, component, event).map(Some)
    }

    fn release_undelivered_token(
        &mut self,
        token: ObjectId,
        outcome: Result<Option<LifecycleEvent>, RuntimeError>,
    ) -> Result<Option<LifecycleEvent>, RuntimeError> {
        if self.lifecycle.tokens.remove(token).is_none() {
            // Only this call owns an undelivered token. A missing token means
            // an internal ownership violation: fault future dispatch while
            // preserving the original operation error when one already exists.
            self.lifecycle.faulted = true;
            return Err(outcome.err().unwrap_or(RuntimeError::Lifecycle));
        }
        outcome
    }

    /// Pull a value-only delivery; mode 0 never advances frame callbacks.
    /// # Errors
    /// Rejects invalid/reentrant frame mode, faults, or exhausted tokens.
    pub fn lifecycle_next(&mut self, mode: i32) -> Result<Option<LifecycleEvent>, RuntimeError> {
        self.validate_delivery_request(mode)?;
        if mode == 1 && self.clock.enabled() {
            return Err(RuntimeError::Lifecycle);
        }
        self.pull_delivery(mode)
    }

    /// Pull a native timed delivery through the existing token ownership protocol.
    /// # Errors
    /// Rejects disabled timing, invalid/reentrant polling, staging and dispatch faults.
    pub fn time_next(&mut self, mode: i32) -> Result<Option<LifecycleEvent>, RuntimeError> {
        self.validate_delivery_request(mode)?;
        self.time_ready()?;
        if !self.clock.enabled() {
            return Err(RuntimeError::Lifecycle);
        }
        self.pull_delivery(mode)
    }

    fn pull_delivery(&mut self, mode: i32) -> Result<Option<LifecycleEvent>, RuntimeError> {
        // Reserve before any phase cursor or destruction mutation. Ownership
        // transfers only when commit_delivery records a delivery for ack.
        let token = self
            .lifecycle
            .tokens
            .insert(())
            .map_err(|_| RuntimeError::IdentityExhausted)?;
        let outcome = self.deliver_reserved_token(mode, token);
        if matches!(outcome, Ok(Some(_))) {
            outcome
        } else {
            self.release_undelivered_token(token, outcome)
        }
    }

    /// Acknowledge the top token once; callback failure permanently faults dispatch.
    /// # Errors
    /// Rejects invalid status, stale/foreign/out-of-order tokens without mutation.
    pub fn lifecycle_ack(&mut self, token: i64, callback_status: i32) -> Result<(), RuntimeError> {
        if callback_status != 0 && callback_status != -6 {
            return Err(RuntimeError::Lifecycle);
        }
        let delivery = self
            .lifecycle
            .deliveries
            .last()
            .filter(|delivery| delivery.token == token)
            .copied()
            .ok_or(RuntimeError::Lifecycle)?;
        let script = self
            .lifecycle
            .scripts
            .get_mut(&delivery.component.to_raw())
            .ok_or(RuntimeError::Lifecycle)?;
        let token_id = ObjectId::from_raw(token).ok_or(RuntimeError::Lifecycle)?;
        self.lifecycle
            .tokens
            .remove(token_id)
            .ok_or(RuntimeError::Lifecycle)?;
        script.in_flight = false;
        self.lifecycle
            .deliveries
            .truncate(self.lifecycle.deliveries.len() - 1);
        self.lifecycle.faulted |= callback_status == -6;
        Ok(())
    }

    /// Queue owned destruction while preserving identities for callback queries.
    /// # Errors
    /// Rejects absent/wrong targets, unpublished reservations, and faulted dispatch.
    pub fn object_destroy_deferred(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        let record = self.objects.get(id).ok_or(RuntimeError::InvalidObject)?;
        let mut scripts = Vec::new();
        match record.kind {
            ObjectKind::Transform(_) => return Err(RuntimeError::InvalidKind),
            ObjectKind::Script { .. } => {
                self.committed_script(id)?;
                scripts.push(id);
            }
            ObjectKind::GameObject { transform, .. } => {
                let mut pending = vec![transform.ok_or(RuntimeError::InvalidKind)?];
                while let Some(transform) = pending.pop() {
                    let data = self.transform(transform)?;
                    scripts.extend(self.owner_scripts(data.owner)?.iter().copied());
                    pending.extend(data.children.iter().copied());
                }
            }
            ObjectKind::Plain => {}
        }
        self.lifecycle_ready()?;
        for script in scripts {
            let script = self
                .lifecycle
                .scripts
                .get_mut(&script.to_raw())
                .ok_or(RuntimeError::InvalidObject)?;
            script.pending_destroy = true;
            script.enabled = false;
        }
        if !self
            .lifecycle
            .pending_destroy
            .iter()
            .any(|target| target.id == id)
        {
            self.lifecycle.pending_destroy.push(DestroyTarget {
                id,
                prepared: false,
            });
        }
        Ok(())
    }

    fn remove_script(&mut self, id: ObjectId) {
        if let Some(script) = self.lifecycle.scripts.remove(&id.to_raw()) {
            if let Some(ObjectRecord {
                kind: ObjectKind::GameObject { scripts, .. },
                ..
            }) = self.objects.get_mut(script.owner)
            {
                scripts.retain(|&component| component != id);
            }
        }
        // An overlapping queued target may have already removed this record.
        drop(self.objects.remove(id));
    }

    fn plan_subtree_destruction(
        &self,
        owner: ObjectId,
        root: ObjectId,
    ) -> Result<PlannedDestruction, RuntimeError> {
        let parent = self.transform(root)?.parent;
        if let Some(parent) = parent {
            self.transform(parent)?;
        }
        let mut nodes = Vec::new();
        let mut pending = vec![root];
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?;
            self.owner_scripts(data.owner)?;
            pending.extend(data.children.iter().copied());
            // Include unpublished reservations without exposing their identities.
            let scripts = self
                .lifecycle
                .scripts
                .iter()
                .filter_map(|(&raw, script)| {
                    (script.owner == data.owner)
                        .then(|| ObjectId::from_raw(raw))
                        .flatten()
                })
                .collect();
            nodes.push(DestructionNode {
                transform,
                owner: data.owner,
                scripts,
            });
        }
        Ok(PlannedDestruction::Subtree {
            root_owner: owner,
            root_transform: root,
            parent,
            nodes,
        })
    }

    fn plan_destruction(&self) -> Result<DestructionPlan, RuntimeError> {
        let mut targets = Vec::new();
        for target in &self.lifecycle.pending_destroy {
            let Some(record) = self.objects.get(target.id) else {
                continue;
            };
            let planned = match record.kind {
                ObjectKind::Script { .. } => PlannedDestruction::Script(target.id),
                ObjectKind::Plain => PlannedDestruction::Plain(target.id),
                ObjectKind::GameObject { transform, .. } => {
                    let root = transform.ok_or(RuntimeError::InvalidKind)?;
                    self.plan_subtree_destruction(target.id, root)?
                }
                ObjectKind::Transform(_) => return Err(RuntimeError::InvalidKind),
            };
            targets.push(planned);
        }
        Ok(DestructionPlan { targets })
    }

    fn commit_subtree_destruction(
        &mut self,
        root_owner: ObjectId,
        root_transform: ObjectId,
        parent: Option<ObjectId>,
        nodes: Vec<DestructionNode>,
    ) {
        // An earlier overlapping request may already have removed this subtree.
        if self.objects.get(root_owner).is_none() {
            return;
        }
        if let Some(parent) = parent {
            if let Some(ObjectRecord {
                kind: ObjectKind::Transform(data),
                ..
            }) = self.objects.get_mut(parent)
            {
                data.children.retain(|&child| child != root_transform);
            }
        }
        for node in nodes {
            for script in node.scripts {
                self.remove_script(script);
            }
            // Removal is idempotent for overlapping targets. All hierarchy
            // lookups were validated before this infallible commit begins.
            drop(self.objects.remove(node.owner));
            drop(self.objects.remove(node.transform));
        }
    }

    fn commit_destruction(&mut self, plan: DestructionPlan) {
        for target in plan.targets {
            match target {
                PlannedDestruction::Script(id) => self.remove_script(id),
                PlannedDestruction::Plain(id) => drop(self.objects.remove(id)),
                PlannedDestruction::Subtree {
                    root_owner,
                    root_transform,
                    parent,
                    nodes,
                } => self.commit_subtree_destruction(root_owner, root_transform, parent, nodes),
            }
        }
        self.lifecycle.pending_destroy.clear();
    }

    fn finalize_destruction(&mut self) -> Result<(), RuntimeError> {
        // Retain every pending request if any validation or traversal fails.
        // No callback runs between this complete plan and its infallible commit.
        let plan = self.plan_destruction()?;
        self.commit_destruction(plan);
        Ok(())
    }
}
