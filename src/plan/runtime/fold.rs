//! Global Fold accumulator and Return cell.

use std::cell::RefCell;
use std::rc::Rc;

use crate::plan::representation::id::MonoidId;
use crate::plan::representation::registry::{Registry, Value};

/// Shared fold accumulator updated at row readiness; finalized in [`crate::plan::shell::Streaming::take`].
pub(crate) struct FoldAcc {
    monoid: MonoidId,
    acc: Value,
}

impl FoldAcc {
    pub(crate) fn new(registry: &Registry, monoid: MonoidId) -> Self {
        Self { monoid, acc: registry.monoid_identity(monoid) }
    }

    pub(crate) fn append(&mut self, registry: &Registry, item: Value) {
        let prev = std::mem::replace(&mut self.acc, Value::Unit);
        self.acc = registry.monoid_append(self.monoid, prev, item);
    }

    pub(crate) fn finish(self) -> Value {
        self.acc
    }
}

/// Live fold cell shared by enter hooks and [`crate::plan::shell::Streaming::take`].
#[derive(Clone)]
pub(crate) struct ReturnCell {
    fold: Rc<RefCell<Option<FoldAcc>>>,
}

impl ReturnCell {
    pub(crate) fn from_fold(fold: FoldAcc) -> Self {
        Self { fold: Rc::new(RefCell::new(Some(fold))) }
    }

    pub(crate) fn fold_slot(&self) -> Rc<RefCell<Option<FoldAcc>>> {
        self.fold.clone()
    }

    pub(crate) fn take(&self) -> Value {
        self.fold.borrow_mut().take().expect("Plan::take before rewrite finished (or take twice)").finish()
    }
}
