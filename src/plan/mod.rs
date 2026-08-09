//! ## Present
//!
//! - [`physical`] - kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] - identity keys, path AST, expr/rel IR, frozen plan graph.
//! - [`offline`] - full-DOM facade over `scraper` plus path denotation.
//!   Relational evaluation comes next.
//!
//! ## Later
//!
//! Offline relational denotation, hand builder, lowering onto Engine, and a
//! public `Plan` shell.

#[cfg(test)]
mod offline;
mod physical;
mod representation;
