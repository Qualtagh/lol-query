//! Opaque identity keys for logical rows and document nodes.

/// ID of a DOM node.
///
/// Shared across Engine chains: two chains matching the same `<a>` share one `NodeId`.
/// DistinctBy, union, and path join use this to detect "same element".
/// Not an Engine [`InstanceId`](crate::engine::InstanceId) (that is one chain's open match).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct NodeId(u64);

impl NodeId {
    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub(crate) const fn raw(self) -> u64 {
        self.0
    }
}

/// One derivation of a logical row.
///
/// This ID prevents two logical parents (e.g., union or expand) with the same destination
/// [`NodeId`](NodeId) from mixing child rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct OccurrenceId(u64);

impl OccurrenceId {
    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub(crate) const fn raw(self) -> u64 {
        self.0
    }
}

/// Observable result order.
///
/// Independent of when callbacks or join events fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct OrderKey(u64);

impl OrderKey {
    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub(crate) const fn raw(self) -> u64 {
        self.0
    }
}
