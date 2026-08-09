//! Ordered-bag relational IR and the frozen plan graph.
//!
//! `Return` / `Sink` sit on [`Plan`] - they schedule observation, not DOM opcodes.

use super::expr::Expr;
use super::id::{ColId, MonoidId, RelationId, SinkId};
use super::path::Path;

/// Relational operator.
///
/// ```text
/// q ::= Root | Expand | Select | Project | UnionAll | DistinctBy | Fold | Take
/// ```
#[derive(Debug, Clone)]
pub(crate) enum RelationalOperator {
    /// One row binding the synthetic document root.
    Root {
        output: ColId,
    },
    /// Correlated path expand: for each input row, follow `path` from `from` into `to`.
    Expand {
        input: RelationId,
        from: ColId,
        to: ColId,
        path: Path,
    },
    /// Stable subsequence where `pred` is true.
    Select {
        input: RelationId,
        predicate: Expr,
    },
    /// One output occurrence per input; schema is exactly `outputs` (equal values do not collapse).
    Project {
        input: RelationId,
        outputs: Box<[(ColId, Expr)]>,
    },
    /// Concatenation of members; preserves duplicates.
    UnionAll {
        members: Box<[RelationId]>,
    },
    /// First occurrence wins per key tuple.
    DistinctBy {
        input: RelationId,
        keys: Box<[ColId]>,
    },
    /// Global ordered fold of mapped row values (one output row).
    ///
    /// `monoid` indexes a builder/runtime `(identity, append)` pair — not a user-facing type list.
    Fold {
        input: RelationId,
        map: Expr,
        monoid: MonoidId,
        output: ColId,
    },
    /// Positional prefix of the ordered input.
    Take {
        input: RelationId,
        n: usize,
    },
}

impl RelationalOperator {
    pub(crate) fn root(output: ColId) -> Self {
        Self::Root { output }
    }

    pub(crate) fn expand(input: RelationId, from: ColId, to: ColId, path: Path) -> Self {
        Self::Expand { input, from, to, path }
    }

    pub(crate) fn select(input: RelationId, predicate: Expr) -> Self {
        Self::Select { input, predicate }
    }

    pub(crate) fn project(input: RelationId, outputs: impl Into<Box<[(ColId, Expr)]>>) -> Self {
        Self::Project { input, outputs: outputs.into() }
    }

    pub(crate) fn union_all(members: impl Into<Box<[RelationId]>>) -> Self {
        Self::UnionAll { members: members.into() }
    }

    pub(crate) fn distinct_by(input: RelationId, keys: impl Into<Box<[ColId]>>) -> Self {
        Self::DistinctBy { input, keys: keys.into() }
    }

    pub(crate) fn fold(input: RelationId, map: Expr, monoid: MonoidId, output: ColId) -> Self {
        Self::Fold { input, map, monoid, output }
    }

    pub(crate) fn take(input: RelationId, n: usize) -> Self {
        Self::Take { input, n }
    }
}

/// Materialize a plan-owned scalar or relation value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Return {
    pub(crate) input: RelationId,
    pub(crate) value: Expr,
}

impl Return {
    pub(crate) fn new(input: RelationId, value: Expr) -> Self {
        Self { input, value }
    }
}

/// Run a caller closure when `value` is ready (scheduling only).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Sink {
    pub(crate) input: RelationId,
    pub(crate) value: Expr,
    pub(crate) id: SinkId,
}

impl Sink {
    pub(crate) fn new(input: RelationId, value: Expr, id: SinkId) -> Self {
        Self { input, value, id }
    }
}

/// Frozen logical plan: immutable after build.
///
/// `relations[i]` is the node addressed by [`RelationId::new`]`(i)`.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub(crate) relations: Box<[RelationalOperator]>,
    pub(crate) sinks: Box<[Sink]>,
    /// `None` means `Return(())` (sinks-only / unit plan).
    pub(crate) ret: Option<Return>,
}

impl Plan {
    pub(crate) fn new(relations: impl Into<Box<[RelationalOperator]>>, sinks: impl Into<Box<[Sink]>>, ret: Option<Return>) -> Self {
        Self { relations: relations.into(), sinks: sinks.into(), ret }
    }

    pub(crate) fn rel(&self, id: RelationId) -> &RelationalOperator {
        &self.relations[id.raw() as usize]
    }

    pub(crate) fn relation_id(index: usize) -> RelationId {
        RelationId::new(index as u64)
    }
}
