//! Ancestry DFA runtime: compile chains, mint InstanceIds, emit handlers.

mod engine;
mod id;

pub(crate) use engine::{AggregatedTextCallback, Callback, CommentCallback, Engine, TextCallback};
pub(crate) use id::{ChainId, InstanceId};
