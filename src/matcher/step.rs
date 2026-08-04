//! Ancestry-chain DSL shared by Matcher, MatchPattern, and Engine.

use crate::ElementView;

/// Predicate on a candidate element for [`Step::Filter`].
pub(crate) trait Predicate: for<'a> Fn(&ElementView<'a>) -> bool + 'static {}
impl<F: for<'a> Fn(&ElementView<'a>) -> bool + 'static> Predicate for F {}

/// One link of an ancestry chain.
pub(crate) enum Step {
    /// A CSS selector, optionally paired with a predicate on the candidate element.
    Filter(String, Option<Box<dyn Predicate>>),
    /// Elements that the nested chain does not match.
    Not(Vec<Step>),
    /// Elements that all of the nested chains match.
    Every(Vec<Vec<Step>>),
    /// Elements that at least one of the nested chains matches.
    Any(Vec<Vec<Step>>),
    /// A gap containing no element matched by the nested chain.
    GapWithout(Vec<Step>),
    /// A gap containing matches for every nested chain, in any order.
    GapWithEvery(Vec<Vec<Step>>),
    /// A gap containing an element matched by at least one nested chain.
    GapWithAny(Vec<Vec<Step>>),
    /// A zero-length gap: the next selector must match a direct child.
    Direct,
}

impl Step {
    pub(crate) fn is_gap(&self) -> bool {
        matches!(self, Step::GapWithout(_) | Step::GapWithEvery(_) | Step::GapWithAny(_) | Step::Direct)
    }

    pub(crate) fn is_element(&self) -> bool {
        matches!(self, Step::Filter(_, _) | Step::Not(_) | Step::Every(_) | Step::Any(_))
    }
}
