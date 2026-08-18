//! Typed handles that allocate columns / relations into a frozen [`Plan`].

use std::cell::RefCell;
use std::rc::Rc;

use crate::matcher::MatchPattern;
use crate::plan::Plan;
use crate::plan::representation::expr::Expr;
use crate::plan::representation::id::{ColId, RelationId};
use crate::plan::representation::path::{NodeTest, Path};
use crate::plan::representation::registry::{Registry, Value};
use crate::plan::representation::relational::{Graph, RelationalOperator, Return};

struct Inner {
    relations: Vec<RelationalOperator>,
    next_col: u64,
    registry: Registry,
    finished: bool,
}

impl Inner {
    fn alloc_col(&mut self) -> ColId {
        let id = ColId::new(self.next_col);
        self.next_col += 1;
        id
    }

    fn push(&mut self, op: RelationalOperator) -> RelationId {
        let id = RelationId::new(self.relations.len() as u64);
        self.relations.push(op);
        id
    }

    fn ensure_open(&self) {
        assert!(!self.finished, "builder already finished");
    }
}

/// Builds a logical [`Plan`] (graph + registry).
#[derive(Clone)]
pub(crate) struct Builder {
    inner: Rc<RefCell<Inner>>,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Self { inner: Rc::new(RefCell::new(Inner { relations: Vec::new(), next_col: 0, registry: Registry::new(), finished: false })) }
    }

    /// Document-root scope (one row binding the synthetic root).
    pub(crate) fn root(&self) -> ScopeRef {
        let mut inner = self.inner.borrow_mut();
        inner.ensure_open();
        let col = inner.alloc_col();
        let relation = inner.push(RelationalOperator::root(col));
        ScopeRef { inner: self.inner.clone(), relation, node: col }
    }
}

/// Node-bearing relation handle (selection / expand result).
#[derive(Clone)]
pub(crate) struct ScopeRef {
    inner: Rc<RefCell<Inner>>,
    relation: RelationId,
    node: ColId,
}

impl ScopeRef {
    /// Descendants matching `test` (`C*;test`).
    pub(crate) fn find(&self, test: impl Into<NodeTest>) -> ScopeRef {
        self.expand(Path::find(test))
    }

    /// Descendants matching a CSS selector.
    pub(crate) fn find_css(&self, selector: impl Into<String>) -> ScopeRef {
        self.find(MatchPattern::new().css(selector))
    }

    /// Immediate element children (`C`).
    pub(crate) fn children(&self) -> ScopeRef {
        self.expand(Path::child_axis())
    }

    /// Immediate children matching a CSS selector (`C;[css]`).
    pub(crate) fn children_css(&self, selector: impl Into<String>) -> ScopeRef {
        self.expand(Path::children(MatchPattern::new().css(selector)))
    }

    /// Correlated path expand from this scope's node column.
    pub(crate) fn expand(&self, path: Path) -> ScopeRef {
        let mut inner = self.inner.borrow_mut();
        inner.ensure_open();
        let to = inner.alloc_col();
        let relation = inner.push(RelationalOperator::expand(self.relation, self.node, to, path));
        ScopeRef { inner: self.inner.clone(), relation, node: to }
    }

    /// Attribute field (missing -> [`Value::Null`] at eval).
    pub(crate) fn attr(&self, name: impl AsRef<str>) -> ValueRef {
        self.inner.borrow().ensure_open();
        ValueRef { inner: self.inner.clone(), relation: self.relation, expr: Expr::attr(self.node, name) }
    }

    /// Aggregated descendant text.
    pub(crate) fn text(&self) -> ValueRef {
        self.inner.borrow().ensure_open();
        ValueRef { inner: self.inner.clone(), relation: self.relation, expr: Expr::text(self.node) }
    }

    /// Element tag name.
    pub(crate) fn tag(&self) -> ValueRef {
        self.inner.borrow().ensure_open();
        ValueRef { inner: self.inner.clone(), relation: self.relation, expr: Expr::tag(self.node) }
    }
}

/// Scalar expression over a relation's rows (not yet folded).
#[derive(Clone)]
pub(crate) struct ValueRef {
    inner: Rc<RefCell<Inner>>,
    relation: RelationId,
    expr: Expr,
}

impl ValueRef {
    /// Unary [`Expr::Apply`] via a registered opaque function.
    pub(crate) fn map(self, f: impl Fn(&[Value]) -> Value + 'static) -> ValueRef {
        let mut inner = self.inner.borrow_mut();
        inner.ensure_open();
        let op = inner.registry.register_apply(f);
        drop(inner);
        ValueRef { inner: self.inner, relation: self.relation, expr: Expr::apply(op, [self.expr]) }
    }

    /// Global ordered fold into a list (`Vec` monoid).
    pub(crate) fn fold(self) -> ValueRef {
        let mut inner = self.inner.borrow_mut();
        inner.ensure_open();
        let monoid = inner.registry.register_monoid(Value::List(Vec::new()), |acc, item| {
            let Value::List(mut xs) = acc else {
                panic!("list fold acc must be List");
            };
            xs.push(item);
            Value::List(xs)
        });
        let output = inner.alloc_col();
        let relation = inner.push(RelationalOperator::fold(self.relation, self.expr, monoid, output));
        drop(inner);
        ValueRef { inner: self.inner, relation, expr: Expr::col(output) }
    }

    /// Freeze the IR graph and registry into a [`Plan`].
    ///
    /// Same plan feeds offline eval [`EvalExt`](crate::plan::offline::eval::EvalExt) and streaming [`StreamingExt`](crate::plan::shell::StreamingExt).
    /// Expects a single-row result (typically after [`Self::fold`]).
    pub(crate) fn build_plan(self) -> Plan {
        let mut inner = self.inner.borrow_mut();
        inner.ensure_open();
        assert!(
            matches!(inner.relations.get(self.relation.raw() as usize), Some(RelationalOperator::Fold { .. })),
            "build_plan expects a Fold result (call fold() first)"
        );
        let graph = Graph::new(std::mem::take(&mut inner.relations), [], Some(Return::new(self.relation, self.expr)));
        let registry = std::mem::replace(&mut inner.registry, Registry::new());
        inner.finished = true;
        Plan::new(graph, registry)
    }
}

#[cfg(test)]
#[path = "builder.test.rs"]
mod test;
