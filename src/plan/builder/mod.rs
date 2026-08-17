//! Hand builder: typed sugar -> [`Plan`](crate::plan::Plan).
//!
//! Flat subset: `root` -> `find`/`expand` -> `attr` -> `map` -> `fold` -> `build_plan`.

#![allow(dead_code)]

mod builder;

#[allow(unused_imports)]
pub(crate) use builder::{Builder, ScopeRef, ValueRef};
