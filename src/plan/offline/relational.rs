//! Offline relational denotation on the scraper facade.
//!
//! Evaluates a frozen [`Plan`] to an ordered sequence of values (via Fold → Return).
//! Emits logical order immediately; no readiness buffering.

use std::collections::HashMap;
use std::rc::Rc;

use super::dom::Dom;
use super::path;
use crate::plan::representation::expr::{Expr, Literal, Projection};
use crate::plan::representation::id::{ApplyId, ColId, MonoidId, NodeId, OccurrenceId, OrderKey, ParamId, RelationId};
use crate::plan::representation::relational::{Plan, RelationalOperator};

/// Runtime scalar / bag value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Unit,
    /// Missing attribute (or other nullish projection).
    Null,
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    Node(NodeId),
    List(Vec<Value>),
}

impl Value {
    pub(crate) fn str(s: impl AsRef<str>) -> Self {
        Self::Str(Rc::from(s.as_ref()))
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    pub(crate) fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(xs) => Some(xs),
            _ => None,
        }
    }
}

impl From<Literal> for Value {
    fn from(value: Literal) -> Self {
        match value {
            Literal::Unit => Self::Unit,
            Literal::Bool(b) => Self::Bool(b),
            Literal::Int(n) => Self::Int(n),
            Literal::Str(s) => Self::Str(s),
        }
    }
}

/// Caller-registered Apply / Fold monoid / parameter slots for a plan.
pub(crate) struct Registry {
    applies: Vec<Box<dyn Fn(&[Value]) -> Value>>,
    monoids: Vec<Monoid>,
    params: Vec<Value>,
}

struct Monoid {
    identity: Value,
    append: Box<dyn Fn(Value, Value) -> Value>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self { applies: Vec::new(), monoids: Vec::new(), params: Vec::new() }
    }

    pub(crate) fn register_apply(&mut self, f: impl Fn(&[Value]) -> Value + 'static) -> ApplyId {
        let id = ApplyId::new(self.applies.len() as u64);
        self.applies.push(Box::new(f));
        id
    }

    pub(crate) fn register_monoid(&mut self, identity: Value, append: impl Fn(Value, Value) -> Value + 'static) -> MonoidId {
        let id = MonoidId::new(self.monoids.len() as u64);
        self.monoids.push(Monoid { identity, append: Box::new(append) });
        id
    }

    pub(crate) fn set_param(&mut self, id: ParamId, value: Value) {
        let i = id.raw() as usize;
        if self.params.len() <= i {
            self.params.resize(i + 1, Value::Unit);
        }
        self.params[i] = value;
    }

    fn apply(&self, id: ApplyId, args: &[Value]) -> Value {
        self.applies.get(id.raw() as usize).expect("unregistered ApplyId")(args)
    }

    fn monoid(&self, id: MonoidId) -> &Monoid {
        self.monoids.get(id.raw() as usize).expect("unregistered MonoidId")
    }

    fn param(&self, id: ParamId) -> &Value {
        self.params.get(id.raw() as usize).expect("unset ParamId")
    }
}

#[derive(Debug, Clone)]
struct Row {
    #[allow(dead_code)]
    occurrence: OccurrenceId,
    #[allow(dead_code)]
    order: OrderKey,
    cols: HashMap<ColId, Value>,
}

/// Evaluate `plan.ret` on `dom`.
pub(crate) fn eval(dom: &Dom, plan: &Plan, registry: &Registry) -> Value {
    let Some(ret) = &plan.ret else {
        return Value::Unit;
    };
    let mut state = State { dom, plan, registry, relation_cache: HashMap::new(), next_occurrence: 0, next_order: 0 };
    let rows = state.relation(ret.input);
    assert!(rows.len() == 1, "Return expects a single input row (Fold result); got {}", rows.len());
    let row = rows[0].clone();
    state.eval_expr(&row, &ret.value)
}

struct State<'a> {
    dom: &'a Dom,
    plan: &'a Plan,
    registry: &'a Registry,
    relation_cache: HashMap<RelationId, Vec<Row>>,
    next_occurrence: u64,
    next_order: u64,
}

impl State<'_> {
    fn fresh_occurrence(&mut self) -> OccurrenceId {
        let id = OccurrenceId::new(self.next_occurrence);
        self.next_occurrence += 1;
        id
    }

    fn fresh_order(&mut self) -> OrderKey {
        let key = OrderKey::new(self.next_order);
        self.next_order += 1;
        key
    }

    fn relation(&mut self, id: RelationId) -> &[Row] {
        if self.relation_cache.contains_key(&id) {
            return &self.relation_cache[&id];
        }
        let rows = self.eval_rel(id);
        self.relation_cache.insert(id, rows);
        &self.relation_cache[&id]
    }

    fn node_col(&self, row: &Row, col: ColId) -> NodeId {
        match row.cols.get(&col) {
            Some(Value::Node(id)) => *id,
            Some(other) => panic!("column {} is not a node: {other:?}", col.raw()),
            None => panic!("missing node column {}", col.raw()),
        }
    }

    fn eval_rel(&mut self, id: RelationId) -> Vec<Row> {
        match self.plan.rel(id) {
            RelationalOperator::Root { output } => {
                let mut cols = HashMap::new();
                cols.insert(*output, Value::Node(self.dom.root()));
                vec![Row { occurrence: self.fresh_occurrence(), order: self.fresh_order(), cols }]
            },
            RelationalOperator::Expand { input, from, to, path } => {
                let input = *input;
                let from = *from;
                let to = *to;
                let path = path.clone();
                let sources: Vec<Row> = self.relation(input).to_vec();
                let mut out = Vec::new();
                for src in sources {
                    let source = self.node_col(&src, from);
                    for dest in path::eval(self.dom, &path, source) {
                        let mut cols = src.cols.clone();
                        cols.insert(to, Value::Node(dest));
                        out.push(Row { occurrence: self.fresh_occurrence(), order: self.fresh_order(), cols });
                    }
                }
                out
            },
            RelationalOperator::Project { input, outputs } => {
                let input = *input;
                let outputs = outputs.clone();
                let sources: Vec<Row> = self.relation(input).to_vec();
                let mut out = Vec::new();
                for src in sources {
                    let mut cols = HashMap::new();
                    for (col, expr) in outputs.iter() {
                        cols.insert(*col, self.eval_expr(&src, expr));
                    }
                    out.push(Row { occurrence: self.fresh_occurrence(), order: self.fresh_order(), cols });
                }
                out
            },
            RelationalOperator::Fold { input, map, monoid, output } => {
                let input = *input;
                let map = map.clone();
                let monoid_id = *monoid;
                let output = *output;
                let sources: Vec<Row> = self.relation(input).to_vec();
                let monoid = self.registry.monoid(monoid_id);
                let mut acc = monoid.identity.clone();
                for src in &sources {
                    let item = self.eval_expr(src, &map);
                    acc = (monoid.append)(acc, item);
                }
                let mut cols = HashMap::new();
                cols.insert(output, acc);
                vec![Row { occurrence: self.fresh_occurrence(), order: self.fresh_order(), cols }]
            },
            RelationalOperator::Select { .. } => panic!("offline rel: Select not implemented yet"),
            RelationalOperator::UnionAll { .. } => panic!("offline rel: UnionAll not implemented yet"),
            RelationalOperator::DistinctBy { .. } => panic!("offline rel: DistinctBy not implemented yet"),
            RelationalOperator::Take { .. } => panic!("offline rel: Take not implemented yet"),
        }
    }

    fn eval_expr(&self, row: &Row, expr: &Expr) -> Value {
        match expr {
            Expr::Literal(lit) => lit.clone().into(),
            Expr::Parameter(id) => self.registry.param(*id).clone(),
            Expr::Col(id) => row.cols.get(id).cloned().unwrap_or_else(|| panic!("missing column {}", id.raw())),
            Expr::Field { node, projection } => {
                let id = self.node_col(row, *node);
                match projection {
                    Projection::Attr(name) => match self.dom.attr(id, name) {
                        Some(v) => Value::str(v),
                        None => Value::Null,
                    },
                    Projection::Text => Value::str(self.dom.text(id)),
                    Projection::Tag => match self.dom.tag(id) {
                        Some(t) => Value::str(t),
                        None => Value::Null,
                    },
                }
            },
            Expr::Apply { op, args } => {
                let vals: Vec<Value> = args.iter().map(|a| self.eval_expr(row, a)).collect();
                self.registry.apply(*op, &vals)
            },
        }
    }
}

#[cfg(test)]
#[path = "relational.test.rs"]
mod test;
