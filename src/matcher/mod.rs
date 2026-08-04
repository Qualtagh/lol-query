//! Public Matcher API and chain DSL (steps + validated patterns).

mod match_pattern;
mod matcher;
mod step;

pub use matcher::Matcher;

pub(crate) use match_pattern::MatchPattern;
pub(crate) use step::{Predicate, Step};
