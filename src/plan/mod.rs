//! ## Present
//!
//! - [`physical`] - kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] - identity keys, path AST, expr/rel IR, frozen plan graph.
//! - [`offline`] - full-DOM facade over `scraper`, path denotation, and flat
//!   relational eval.
//!
//! ## Later
//!
//! Hand builder, lowering onto Engine, and a public `Plan` shell.

#[cfg(test)]
mod offline;
mod physical;
mod representation;
