//! Scalar expressions over row columns.
//!
//! `Apply` is pure and deterministic. Ops are opaque ids into a builder/runtime
//! registry: readiness is the max of argument readiness; the IR does not interpret bodies.

use std::rc::Rc;

use super::id::{ApplyId, ColId, ParamId};

/// Projection of a node column onto a scalar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Projection {
    /// Element attribute (missing → nullish at eval time).
    Attr(Rc<str>),
    /// Aggregated descendant text (cheerio-like).
    Text,
    /// Element tag name.
    Tag,
}

impl Projection {
    pub(crate) fn attr(name: impl AsRef<str>) -> Self {
        Self::Attr(Rc::from(name.as_ref()))
    }
}

/// Compile-time constant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Literal {
    Unit,
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
}

impl Literal {
    pub(crate) fn str(s: impl AsRef<str>) -> Self {
        Self::Str(Rc::from(s.as_ref()))
    }
}

impl From<bool> for Literal {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for Literal {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<()> for Literal {
    fn from((): ()) -> Self {
        Self::Unit
    }
}

/// Scalar expression.
///
/// ```text
/// e ::= Literal | Parameter | Col | Field(node, projection) | Apply(op, e1, ..., eN)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Expr {
    Literal(Literal),
    Parameter(ParamId),
    /// Column already on the row (node binding or prior project output).
    Col(ColId),
    /// Node field projection.
    Field {
        node: ColId,
        projection: Projection,
    },
    Apply {
        op: ApplyId,
        args: Box<[Expr]>,
    },
}

impl Expr {
    pub(crate) fn literal(value: impl Into<Literal>) -> Self {
        Self::Literal(value.into())
    }

    pub(crate) fn param(id: ParamId) -> Self {
        Self::Parameter(id)
    }

    pub(crate) fn col(id: ColId) -> Self {
        Self::Col(id)
    }

    pub(crate) fn field(node: ColId, projection: Projection) -> Self {
        Self::Field { node, projection }
    }

    pub(crate) fn attr(node: ColId, name: impl AsRef<str>) -> Self {
        Self::field(node, Projection::attr(name))
    }

    pub(crate) fn text(node: ColId) -> Self {
        Self::field(node, Projection::Text)
    }

    pub(crate) fn tag(node: ColId) -> Self {
        Self::field(node, Projection::Tag)
    }

    pub(crate) fn apply(op: ApplyId, args: impl Into<Box<[Expr]>>) -> Self {
        Self::Apply { op, args: args.into() }
    }
}

impl From<Literal> for Expr {
    fn from(value: Literal) -> Self {
        Self::Literal(value)
    }
}
