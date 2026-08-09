//! ## Present
//!
//! - [`physical`] - kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] - identity keys, path AST, expr/rel IR, frozen plan graph.
//! - [`registry`] - opaque Apply / Fold monoids and runtime values.
//! - [`builder`] - typed hand API → logical IR (flat subset).
//! - [`offline`] - full-DOM facade over `scraper`, path denotation, and flat
//!   relational eval (test reference; later runtime mode).
//!
//! ## Later
//!
//! Lowering onto Engine, and a public `Plan` shell.

mod builder;
mod physical;
mod registry;
mod representation;

#[cfg(test)]
mod offline;
