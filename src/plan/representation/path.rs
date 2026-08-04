//! Path algebra AST.
//!
//! Cheerio-like constructors live in [`super::path_extension`]. Evaluation is offline.

use std::rc::Rc;

use crate::match_pattern::MatchPattern;

/// Boolean node test (predicate): `false`, `true`, matcher, NOT, OR, AND.
///
/// A matcher arm holds a validated [`MatchPattern`] (Engine step chain) shared via [`Rc`].
#[derive(Debug, Clone)]
pub(crate) enum NodeTest {
    False,
    True,
    Match(Rc<MatchPattern>),
    Not(Box<NodeTest>),
    Or(Box<NodeTest>, Box<NodeTest>),
    And(Box<NodeTest>, Box<NodeTest>),
}

impl NodeTest {
    pub(crate) fn matcher(pattern: MatchPattern) -> Self {
        pattern.into()
    }

    pub(crate) fn not(self) -> Self {
        Self::Not(Box::new(self))
    }

    pub(crate) fn or(self, other: Self) -> Self {
        Self::Or(Box::new(self), Box::new(other))
    }

    pub(crate) fn and(self, other: Self) -> Self {
        Self::And(Box::new(self), Box::new(other))
    }
}

impl From<MatchPattern> for NodeTest {
    fn from(pattern: MatchPattern) -> Self {
        pattern.validate();
        Self::Match(Rc::new(pattern))
    }
}

/// Path expression over the child / next generators and tests.
#[derive(Debug, Clone)]
pub(crate) enum Path {
    /// Empty relation.
    Empty,
    /// Identity.
    Id,
    /// Child axis.
    Child,
    /// Next-sibling axis.
    Next,
    /// Test as subidentity.
    Test(NodeTest),
    /// Path union (r + s) - parallel composition (one path or another).
    Union(Box<Path>, Box<Path>),
    /// Composition (r ; s) - sequential composition.
    Seq(Box<Path>, Box<Path>),
    /// Reflexive transitive closure (r*) - Kleene star (zero or more repetitions).
    Star(Box<Path>),
    /// Converse - opposite direction (next<->prev, parent<->child).
    Converse(Box<Path>),
}

impl Path {
    pub(crate) fn empty() -> Self {
        Self::Empty
    }

    pub(crate) fn id() -> Self {
        Self::Id
    }

    pub(crate) fn child_axis() -> Self {
        Self::Child
    }

    pub(crate) fn next_axis() -> Self {
        Self::Next
    }

    pub(crate) fn parent_axis() -> Self {
        Self::Child.converse()
    }

    pub(crate) fn prev_axis() -> Self {
        Self::Next.converse()
    }

    pub(crate) fn test(phi: impl Into<NodeTest>) -> Self {
        Self::Test(phi.into())
    }

    /// Composition.
    pub(crate) fn then(self, other: Self) -> Self {
        Self::Seq(Box::new(self), Box::new(other))
    }

    /// Restrict destinations.
    pub(crate) fn filter(self, test: impl Into<NodeTest>) -> Self {
        match test.into() {
            NodeTest::True => self,
            other => self.then(Self::test(other)),
        }
    }

    pub(crate) fn union(self, other: Self) -> Self {
        Self::Union(Box::new(self), Box::new(other))
    }

    pub(crate) fn star(self) -> Self {
        Self::Star(Box::new(self))
    }

    /// Transitive closure (r+) - one or more repetitions.
    pub(crate) fn plus(self) -> Self {
        let rest = self.clone().star();
        self.then(rest)
    }

    pub(crate) fn converse(self) -> Self {
        Self::Converse(Box::new(self))
    }
}
