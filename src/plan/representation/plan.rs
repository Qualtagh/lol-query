//! Finalized plan: relational [`Graph`] plus opaque [`Registry`].
//!
//! Backend methods live on traits in [`crate::plan::shell`] and [`crate::plan::offline`].

use super::registry::Registry;
use super::relational::Graph;

/// Logical plan ready for streaming compile or offline evaluation.
pub(crate) struct Plan {
    pub(crate) graph: Graph,
    pub(crate) registry: Registry,
}

impl Plan {
    pub(crate) fn new(graph: Graph, registry: Registry) -> Self {
        Self { graph, registry }
    }
}
