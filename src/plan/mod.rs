//! ## Present
//!
//! - [`physical`] - kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] - ids, path, expr, registry, graph, finalized [`Plan`].
//! - [`builder`] - typed hand API -> [`Plan`] (flat subset).
//! - [`offline`] - full-DOM facade.
//! - [`lower`] / [`runtime`] / [`shell`] - streaming lower.
//!
//! ## Later
//!
//! Nested Expand, ancestry combine, Sink, up/side axes.

mod builder;
mod lower;
mod physical;
mod representation;
mod runtime;
mod shell;

#[cfg(test)]
mod offline;
#[cfg(test)]
mod test_util;

pub(crate) use representation::plan::Plan;
