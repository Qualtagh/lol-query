//! Logical plan representation.
//!
//! Identity keys and path AST. Relational operators arrive in later strata.

#![allow(dead_code)]

pub(crate) mod id;
pub(crate) mod path;
pub(crate) mod path_extension;
