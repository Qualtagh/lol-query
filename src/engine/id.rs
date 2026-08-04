//! Engine activation and chain identity.

/// One open match of one Engine chain.
///
/// Two chains matching the same `<a>` get two instances.
/// Frames, exit hooks, and text aggregation key off this (“this activation ended”).
/// Not a [`NodeId`](crate::plan::representation::id::NodeId) (that is the DOM node ID).
pub(crate) type InstanceId = u64;

/// Index of a compiled ancestry chain in an [`Engine`](super::Engine).
pub(crate) type ChainId = usize;
