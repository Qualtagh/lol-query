//! Streaming lower: logical IR -> Engine subscriptions + physical kernels.
//!
//! Flat downward Expand first (`bind_down`); nesting / up / sibling come later.

#![allow(dead_code)]

mod bind_down;
mod compile;

pub(crate) use bind_down::{DownExpand, bind_chain, take_down_expand};
pub(crate) use compile::compile;
