//! Cheerio-like path constructors. Extensions to the algebra in [`super::path`].

use super::path::{NodeTest, Path};

impl Path {
    /// Children.
    pub(crate) fn children(test: impl Into<NodeTest>) -> Self {
        Self::child_axis().filter(test)
    }

    /// All descendants.
    pub(crate) fn descendants() -> Self {
        Self::child_axis().plus()
    }

    /// Descendants matching a predicate.
    pub(crate) fn find(test: impl Into<NodeTest>) -> Self {
        Self::child_axis().plus().filter(test)
    }

    /// Parent.
    pub(crate) fn parent() -> Self {
        Self::parent_axis()
    }

    /// Ancestors matching a predicate.
    pub(crate) fn parents(test: impl Into<NodeTest>) -> Self {
        Self::parent_axis().plus().filter(test)
    }

    /// Next sibling.
    pub(crate) fn next() -> Self {
        Self::next_axis()
    }

    /// Following siblings matching a predicate.
    pub(crate) fn next_all(test: impl Into<NodeTest>) -> Self {
        Self::next_axis().plus().filter(test)
    }

    /// Previous sibling.
    pub(crate) fn prev() -> Self {
        Self::prev_axis()
    }

    /// Preceding siblings matching a predicate.
    pub(crate) fn prev_all(test: impl Into<NodeTest>) -> Self {
        Self::prev_axis().plus().filter(test)
    }

    /// Siblings matching a predicate.
    pub(crate) fn siblings(test: impl Into<NodeTest>) -> Self {
        Self::next_axis().plus().union(Self::prev_axis().plus()).filter(test)
    }

    /// Closest ancestor-or-self matching a predicate.
    pub(crate) fn closest(test: impl Into<NodeTest>) -> Self {
        let p: NodeTest = test.into();
        let climb_while_not = Self::test(p.clone().not()).then(Self::parent_axis());
        climb_while_not.star().then(Self::test(p))
    }

    /// Next siblings until stop (stop excluded).
    pub(crate) fn next_until(stop: impl Into<NodeTest>) -> Self {
        Self::until_axis(Self::next_axis(), stop)
    }

    /// Previous siblings until stop (stop excluded).
    pub(crate) fn prev_until(stop: impl Into<NodeTest>) -> Self {
        Self::until_axis(Self::prev_axis(), stop)
    }

    /// Ancestors until stop (stop excluded).
    pub(crate) fn parents_until(stop: impl Into<NodeTest>) -> Self {
        Self::until_axis(Self::parent_axis(), stop)
    }

    fn until_axis(axis: Self, stop: impl Into<NodeTest>) -> Self {
        axis.then(Self::test(stop.into().not())).plus()
    }
}
