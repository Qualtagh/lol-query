//! Streaming runtime state for physical plans.
//!
//! Fold / Return cells now; slots and sinks land with later commits.

#![allow(dead_code)]

mod fold;

pub(crate) use fold::{FoldAcc, ReturnCell};
