use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A process-local identity, never an address or a serialized Unity file ID.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(NonZeroU64);

impl ObjectId {
    /// Decode a positive ABI identity. Liveness still requires a registry lookup.
    pub fn from_raw(raw: i64) -> Option<Self> {
        NonZeroU64::new(u64::try_from(raw).ok()?).map(Self)
    }

    /// Return the fixed-width ABI representation.
    pub fn to_raw(self) -> i64 {
        // Construction is restricted to the positive signed 63-bit range.
        self.0.get() as i64
    }
}

/// No further identities can be allocated without risking identity reuse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityExhausted;

impl fmt::Display for IdentityExhausted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("process identity space exhausted")
    }
}

impl std::error::Error for IdentityExhausted {}

/// Allocate a fresh process-local identity shared by all owning registries.
///
/// Allocation never reuses a prior identity, including after its owner is dropped.
/// This function assigns identity only; the caller owns value storage and liveness.
///
/// # Errors
/// Returns [`IdentityExhausted`] once the positive signed 63-bit space is exhausted.
pub fn allocate_next() -> Result<ObjectId, IdentityExhausted> {
    allocate(&NEXT_ID)
}

fn allocate(counter: &AtomicU64) -> Result<ObjectId, IdentityExhausted> {
    // The value after i64::MAX is a permanent exhaustion sentinel. Relaxed
    // ordering suffices for uniqueness; registry synchronization owns values.
    let raw = counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            (next <= i64::MAX as u64).then_some(next + 1)
        })
        .map_err(|_| IdentityExhausted)?;
    NonZeroU64::new(raw).map(ObjectId).ok_or(IdentityExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_exhaustion_never_wraps_or_recycles() {
        let counter = AtomicU64::new(i64::MAX as u64);
        assert_eq!(allocate(&counter).unwrap().to_raw(), i64::MAX);
        assert_eq!(allocate(&counter), Err(IdentityExhausted));
        assert_eq!(allocate(&counter), Err(IdentityExhausted));
    }
}
