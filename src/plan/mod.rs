//! ## Present
//!
//! - [`physical`] - kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] - ids, path, expr, registry, graph, finalized [`Plan`].
//! - [`builder`] - typed hand API -> [`Plan`].
//! - [`offline`] - full-DOM facade and relational eval.
//! - [`lower`] / [`runtime`] / [`shell`] - streaming lower.
//!
//! ## Later
//!
//! Streaming nested Expand, ancestry combine, Sink, up/side axes.

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
