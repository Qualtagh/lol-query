//! Immutable ancestry / gap hierarchy shared by [`Matcher`](crate::Matcher) and plan IR.

use super::{Predicate, Step};

/// Validated Matcher hierarchy: element selectors, gaps, and nested combinators.
pub(crate) struct MatchPattern {
    steps: Vec<Step>,
}

impl MatchPattern {
    pub(crate) fn new() -> Self {
        Self { steps: vec![] }
    }

    /// CSS selector plus custom predicate.
    pub(crate) fn filter(self, selector: impl Into<String>, predicate: impl Predicate) -> Self {
        self.push_element(Step::Filter(selector.into(), Some(Box::new(predicate))))
    }

    /// CSS selector with no predicate.
    pub(crate) fn css(self, selector: impl Into<String>) -> Self {
        self.push_element(Step::Filter(selector.into(), None))
    }

    /// Negated nested pattern (`:not()`).
    pub(crate) fn not(self, pattern: MatchPattern) -> Self {
        pattern.validate();
        self.push_element(Step::Not(pattern.into_steps()))
    }

    /// Conjunction of nested patterns.
    pub(crate) fn every(self, patterns: Vec<MatchPattern>) -> Self {
        assert!(!patterns.is_empty(), "every() requires at least one matcher");
        self.push_element(Step::Every(Self::nested_chains(patterns)))
    }

    /// Disjunction of nested patterns.
    pub(crate) fn any(self, patterns: Vec<MatchPattern>) -> Self {
        assert!(!patterns.is_empty(), "any() requires at least one matcher");
        self.push_element(Step::Any(Self::nested_chains(patterns)))
    }

    /// Gap with no nested match.
    pub(crate) fn gap_without(self, pattern: MatchPattern) -> Self {
        pattern.validate();
        self.push_gap(Step::GapWithout(pattern.into_steps()))
    }

    /// Gap containing every nested pattern (any order).
    pub(crate) fn gap_with_every(self, patterns: Vec<MatchPattern>) -> Self {
        assert!(!patterns.is_empty(), "gap_with_every() requires at least one matcher");
        self.push_gap(Step::GapWithEvery(Self::nested_chains(patterns)))
    }

    /// Gap containing any nested pattern.
    pub(crate) fn gap_with_any(self, patterns: Vec<MatchPattern>) -> Self {
        assert!(!patterns.is_empty(), "gap_with_any() requires at least one matcher");
        self.push_gap(Step::GapWithAny(Self::nested_chains(patterns)))
    }

    /// Zero-length gap (`>` combinator). Must sit between two element selectors.
    pub(crate) fn direct(self) -> Self {
        assert!(!self.steps.is_empty(), "direct() cannot be the first selector");
        assert!(self.steps.last().is_some_and(Step::is_element), "direct() must follow an element selector");
        self.push_gap(Step::Direct)
    }

    /// Non-empty and not ending on a gap. Call before Engine compile or nesting.
    pub(crate) fn validate(&self) {
        assert!(!self.steps.is_empty(), "a matcher needs at least one selector");
        assert!(!self.steps.last().unwrap().is_gap(), "a gap selector cannot be final in a chain");
    }

    pub(crate) fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub(crate) fn into_steps(self) -> Vec<Step> {
        self.steps
    }

    fn nested_chains(patterns: Vec<MatchPattern>) -> Vec<Vec<Step>> {
        patterns
            .into_iter()
            .map(|pattern| {
                pattern.validate();
                pattern.into_steps()
            })
            .collect()
    }

    fn push_element(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    fn push_gap(mut self, gap: Step) -> Self {
        assert!(!matches!(self.steps.last(), Some(Step::Direct)), "direct() must be followed by an element selector");
        self.steps.push(gap);
        self
    }
}

impl std::fmt::Debug for MatchPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MatchPattern").field(&format_args!("{} steps", self.steps.len())).finish()
    }
}
