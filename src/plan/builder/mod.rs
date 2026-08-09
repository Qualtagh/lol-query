//! Hand builder: typed sugar → logical IR.
//!
//! Flat subset: `root` → `find`/`expand` → `attr` → `map` → `fold` → `build_plan`.

#![allow(dead_code)]

mod builder;

#[allow(unused_imports)]
pub(crate) use builder::{Builder, ScopeRef, ValueRef};
