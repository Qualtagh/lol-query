//! Streaming shell: lol-html handlers + result cell after rewrite.
//!
//! Crate-internal for now; a Matcher-easy public hand API is planned separately.

#![allow(dead_code)]

mod streaming;

pub(crate) use streaming::{Streaming, StreamingExt};
