//! Offline (full-DOM) backend.
//!
//! Same logical denotation as the streaming Engine path; used as the reference
//! evaluator first, later as an optional runtime mode.

#![allow(dead_code)]

pub(crate) mod dom;
pub(crate) mod eval;
pub(crate) mod path;
pub(crate) mod relational;
