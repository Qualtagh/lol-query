//! Offline relational denotation on the scraper facade.
//!
//! Evaluates a frozen [`Plan`] to an ordered sequence of values (via Fold -> Return).
//! Emits logical order immediately; no readiness buffering.
//! Expand clones the source row and binds `to`, so nested Expand keeps outer columns.

use std::collections::HashMap;

use super::dom::Dom;
use super::path;
use crate::plan::representation::expr::{Expr, Projection};
use crate::plan::representation::id::{ColId, NodeId, OccurrenceId, OrderKey, RelationId};
use crate::plan::representation::registry::{Registry, Value};
use crate::plan::representation::relational::{Graph, RelationalOperator};

#[derive(Debug, Clone)]
struct Row {
    #[allow(dead_code)]
    occurrence: OccurrenceId,
    #[allow(dead_code)]
    order: OrderKey,
    cols: HashMap<ColId, Value>,
}

/// Evaluate `graph.ret` on `dom`.
pub(crate) fn eval(dom: &Dom, graph: &Graph, registry: &Registry) -> Value {
    let Some(ret) = &graph.ret else {
        return Value::Unit;
    };
    let mut state = State { dom, graph, registry, relation_cache: HashMap::new(), next_occurrence: 0, next_order: 0 };
    let rows = state.relation(ret.input);
    assert!(rows.len() == 1, "Return expects a single input row (Fold result); got {}", rows.len());
    let row = rows[0].clone();
    state.eval_expr(&row, &ret.value)
}

struct State<'a> {
    dom: &'a Dom,
    graph: &'a Graph,
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
        match self.graph.rel(id) {
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
                let mut acc = self.registry.monoid_identity(monoid_id);
                for src in &sources {
                    let item = self.eval_expr(src, &map);
                    acc = self.registry.monoid_append(monoid_id, acc, item);
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
