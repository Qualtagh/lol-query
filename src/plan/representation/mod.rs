//! Logical plan representation.
//!
//! Identity keys, path AST, scalar expressions, and the frozen relational graph.

#![allow(dead_code)]

pub(crate) mod expr;
pub(crate) mod id;
pub(crate) mod path;
pub(crate) mod path_extension;
pub(crate) mod relational;
