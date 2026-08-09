//! Ancestry DFA runtime: compile chains, mint ids, emit handlers.
//!
//! Per element enter the Engine mints one raw node id shared by every chain that matches
//! that element (wrap with plan `NodeId::new`). [`InstanceId`] is separate: one open match
//! of one chain (two chains on the same `<a>` get two instances, one node id).

mod engine;
mod id;

pub(crate) use engine::{AggregatedTextCallback, Callback, CommentCallback, Engine, TextCallback};
pub(crate) use id::{ChainId, InstanceId};
