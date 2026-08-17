//! Logical plan representation.
//!
//! Identity keys, path AST, scalar expressions, registry bodies, frozen relational
//! graph, and the finalized [`Plan`] (graph + registry).

#![allow(dead_code)]

pub(crate) mod expr;
pub(crate) mod id;
pub(crate) mod path;
pub(crate) mod path_extension;
pub(crate) mod plan;
pub(crate) mod registry;
pub(crate) mod relational;
