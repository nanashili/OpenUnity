//! Native lifecycle ownership, scheduling and callback delivery.

mod access;
mod activity;
mod components;
mod delivery;
mod destruction_commit;
mod destruction_requests;
mod eligibility;
mod frames;
mod hierarchy_notifications;
mod state;
mod timed_frames;

pub(crate) use state::Lifecycle;
pub use state::LifecycleEvent;
