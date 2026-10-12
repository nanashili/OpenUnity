//! Bounded snapshots for independently authored editor transactions.
//!
//! Callers validate and apply an edit before committing it. Scene integrators
//! must preserve their session allocation high-water mark when restoring a
//! snapshot; generic history cannot enforce that scene-specific invariant.

use std::collections::VecDeque;

#[derive(Clone, Debug)]
struct Entry<T> {
    label: String,
    before: T,
    after: T,
}

/// A bounded undo/redo history of successful, whole-state transactions.
///
/// Both undoable and redoable transactions count toward `capacity`. Oldest
/// transactions are evicted when a commit fills the history. A zero capacity
/// disables recording. Storage is private and snapshots are owned by history.
#[derive(Clone, Debug)]
pub struct History<T: Clone + PartialEq> {
    capacity: usize,
    undo: VecDeque<Entry<T>>,
    redo: Vec<Entry<T>>,
}

impl<T: Clone + PartialEq> History<T> {
    /// Creates an empty history, retaining at most `capacity` transactions.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            undo: VecDeque::new(),
            redo: Vec::new(),
        }
    }

    /// Records an already successful edit and returns whether it was recorded.
    ///
    /// Equal snapshots are a no-op: they preserve all history, including redo.
    /// A real edit discards redo and evicts the oldest undo transaction as
    /// needed. With zero capacity, this returns `false` and stores no snapshots.
    /// Failed edits must not be passed to this method.
    pub fn commit(&mut self, label: impl Into<String>, before: T, after: T) -> bool {
        if before == after {
            return false;
        }
        self.redo.clear();
        if self.capacity == 0 {
            return false;
        }
        if self.undo.len() == self.capacity {
            self.undo.pop_front();
        }
        self.undo.push_back(Entry {
            label: label.into(),
            before,
            after,
        });
        true
    }

    /// Moves the latest transaction to redo and returns its before snapshot.
    ///
    /// `current` is intentionally not compared or stored: restoration uses the
    /// committed snapshots. This permits callers to maintain session state,
    /// such as scene allocation high-water counters, outside restored state.
    /// Callers must commit their edits and clear history on document replacement.
    pub fn undo(&mut self, _current: &T) -> Option<T> {
        let entry = self.undo.pop_back()?;
        let restored = entry.before.clone();
        self.redo.push(entry);
        Some(restored)
    }

    /// Moves the next redo transaction to undo and returns its after snapshot.
    ///
    /// As with [`Self::undo`], `current` is not compared or stored.
    pub fn redo(&mut self, _current: &T) -> Option<T> {
        let entry = self.redo.pop()?;
        let restored = entry.after.clone();
        self.undo.push_back(entry);
        Some(restored)
    }

    /// Returns whether a transaction can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Returns whether a transaction can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Returns the label of the next transaction to undo.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|entry| entry.label.as_str())
    }

    /// Returns the label of the next transaction to redo.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|entry| entry.label.as_str())
    }

    /// Discards every undo and redo transaction, retaining the capacity.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn undo_redo_restores_transactions_and_labels_in_order() {
        let mut history = History::new(3);
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo(&0), None);
        assert_eq!(history.redo(&0), None);
        assert!(history.commit("create", 0, 1));
        assert!(history.commit("rename", 1, 2));
        assert_eq!(history.undo_label(), Some("rename"));
        assert_eq!(history.undo(&2), Some(1));
        assert_eq!(history.redo_label(), Some("rename"));
        assert_eq!(history.undo_label(), Some("create"));
        assert_eq!(history.undo(&1), Some(0));
        assert!(!history.can_undo());
        assert_eq!(history.redo(&0), Some(1));
        assert_eq!(history.redo(&1), Some(2));
        assert!(!history.can_redo());
    }

    #[test]
    fn no_op_preserves_redo_but_real_edit_discards_it() {
        let mut history = History::new(2);
        history.commit("first", 0, 1);
        history.undo(&1);
        assert!(!history.commit("unchanged", 0, 0));
        assert_eq!(history.redo_label(), Some("first"));
        assert_eq!(history.undo_label(), None);
        assert!(history.commit("branch", 0, 2));
        assert_eq!(history.redo(&2), None);
        assert_eq!(history.undo(&2), Some(0));
    }

    #[test]
    fn evicts_oldest_transaction_and_keeps_redo_bounded() {
        let mut history = History::new(2);
        history.commit("oldest", 0, 1);
        history.commit("middle", 1, 2);
        history.commit("newest", 2, 3);
        assert_eq!(history.undo(&3), Some(2));
        assert_eq!(history.undo(&2), Some(1));
        assert_eq!(history.undo(&1), None);
        assert_eq!(history.redo(&1), Some(2));
        assert_eq!(history.redo(&2), Some(3));
        history.commit("next", 3, 4);
        assert_eq!(history.undo(&4), Some(3));
        assert_eq!(history.undo(&3), Some(2));
        assert_eq!(history.undo(&2), None);
    }

    #[test]
    fn zero_capacity_disables_recording() {
        let mut history = History::new(0);
        assert!(!history.commit("edit", 0, 1));
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_label(), None);
        assert_eq!(history.redo_label(), None);
        assert_eq!(history.undo(&1), None);
        assert_eq!(history.redo(&1), None);
    }

    #[test]
    fn clear_discards_both_directions_and_retains_capacity() {
        let mut history = History::new(2);
        history.commit("first", 0, 1);
        history.commit("second", 1, 2);
        history.undo(&2);
        history.clear();
        assert_eq!(history.undo_label(), None);
        assert_eq!(history.redo_label(), None);
        assert_eq!(history.undo(&1), None);
        assert_eq!(history.redo(&1), None);
        assert!(history.commit("new", 7, 8));
        assert_eq!(history.undo(&8), Some(7));
    }

    #[test]
    fn restored_values_are_independent_and_current_does_not_replace_snapshots() {
        let mut history = History::new(1);
        history.commit("append", vec![1], vec![1, 2]);
        let mut restored = history.undo(&vec![99]).unwrap();
        restored.push(3);
        assert_eq!(history.redo(&restored), Some(vec![1, 2]));
        assert_eq!(history.undo(&vec![99]), Some(vec![1]));
    }
}
