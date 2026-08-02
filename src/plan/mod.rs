//! ## Present
//!
//! - [`physical`] — kernels for the streaming rewrite: downward birth
//!   gating (`down_ok`), scope-tree ancestry, and activation frames.
//! - [`representation`] — logical identity types (`NodeId`, `OccurrenceId`,
//!   `OrderKey`). Path AST and relational IR arrive in later strata.
//!
//! ## Later
//!
//! In-memory document tree, path/relational denotation, hand builder, lowering
//! onto Engine, and a public `Plan` shell.

mod physical;
mod representation;
