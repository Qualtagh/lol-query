//! Offline (full-DOM) backend.
//!
//! Same logical denotation as the streaming Engine path; used as the reference
//! evaluator first, later as an optional runtime mode.

#![allow(dead_code)]

#[cfg(test)]
mod test_util;

pub(crate) mod dom;
pub(crate) mod path;
