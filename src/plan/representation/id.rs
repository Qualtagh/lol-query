//! Opaque identity keys for logical rows, columns, and document nodes.

macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub(crate) struct $name(u64);

        impl $name {
            pub(crate) const fn new(raw: u64) -> Self {
                Self(raw)
            }

            pub(crate) const fn raw(self) -> u64 {
                self.0
            }
        }
    };
}

opaque_id! {
    /// ID of a DOM node.
    ///
    /// Shared across Engine chains: two chains matching the same `<a>` share one `NodeId`.
    /// DistinctBy, union, and path join use this to detect "same element".
    /// - Streaming: Engine mints one id per element enter (see [`crate::engine`]).
    /// - Offline: the scraper Dom facade assigns ids over the full tree.
    ///
    /// Not an Engine [`InstanceId`](crate::engine::InstanceId) (that is one chain's open match).
    NodeId
}

opaque_id! {
    /// One derivation of a logical row.
    ///
    /// This ID prevents two logical parents (e.g., union or expand) with the same destination
    /// [`NodeId`] from mixing child rows.
    OccurrenceId
}

opaque_id! {
    /// Observable result order.
    ///
    /// Independent of when callbacks or join events fire.
    OrderKey
}

opaque_id! {
    /// Index of a relation node in a frozen [`super::relational::Graph`].
    RelationId
}

opaque_id! {
    /// Column binding within a plan (node scope or projected value).
    ///
    /// Allocated by the builder; unique across the plan.
    ColId
}

opaque_id! {
    /// Caller-registered pure function for [`super::expr::Expr::Apply`].
    ApplyId
}

opaque_id! {
    /// Plan parameter slot (`Literal`-like input supplied at run time).
    ParamId
}

opaque_id! {
    /// Caller-registered fold monoid `(identity, append)` for Fold.
    MonoidId
}

opaque_id! {
    /// Caller closure scheduled by a Sink (lives outside the IR).
    SinkId
}
