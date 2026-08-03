//! ## Present
//!
//! - [`physical`] — kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] — logical identity types (`NodeId`, `OccurrenceId`,
//!   `OrderKey`). Path AST and relational IR arrive in later strata.
//! - [`offline`] — full-DOM facade over `scraper` (plan root, element axes,
//!   attr/text, CSS match). Path and relational evaluation come next.
//!
//! ## Later
//!
//! Path/relational denotation, hand builder, lowering onto Engine, and a
//! public `Plan` shell.

#[cfg(test)]
mod offline;
mod physical;
mod representation;
