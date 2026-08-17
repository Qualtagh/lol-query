//! Lower a finalized [`Plan`](crate::plan::Plan) onto Engine + Frames -> [`Streaming`].

use std::cell::RefCell;
use std::rc::Rc;

use lol_html::LocalHandlerTypes;
use lol_html::html_content::Element;

use crate::engine::Engine;
use crate::plan::Plan;
use crate::plan::lower::{DownExpand, bind_chain, take_down_expand};
use crate::plan::physical::down::down_ok;
use crate::plan::physical::frames::{Frame, Frames};
use crate::plan::representation::expr::{Expr, Projection};
use crate::plan::representation::id::{ColId, MonoidId};
use crate::plan::representation::path::Path;
use crate::plan::representation::registry::{Registry, Value};
use crate::plan::representation::relational::{Graph, RelationalOperator};
use crate::plan::runtime::{FoldAcc, ReturnCell};
use crate::plan::shell::Streaming;

type El<'r, 't> = Element<'r, 't, LocalHandlerTypes>;

const ROOT_STACK: usize = 0;
const EXPAND_STACK: usize = 1;
const ROOT_INSTANCE: u64 = u64::MAX;

/// Compile a flat `Root -> Expand(C*;p) -> (Project?) -> Fold -> Return` plan for streaming.
///
/// Consumes the plan so matcher steps move into Engine.
pub(crate) fn compile(plan: Plan) -> Streaming {
    let Plan { mut graph, registry } = plan;
    let flat = analyze(&mut graph);
    let mut engine = Engine::new();
    let extent = flat.expand.extent;
    let chain = bind_chain(&mut engine, flat.expand);

    let registry = Rc::new(registry);
    let result = ReturnCell::from_fold(FoldAcc::new(&registry, flat.monoid));
    let fold_slot = result.fold_slot();

    let frames = Rc::new(RefCell::new(Frames::new(2)));
    frames.borrow_mut().push(ROOT_STACK, Frame::new(ROOT_INSTANCE, 0, true));

    let map = flat.map;
    let expand_to = flat.expand_to;
    let frames_enter = frames.clone();
    let frames_exit = frames;
    let registry_enter = registry;
    let fold_enter = fold_slot;

    engine.on_enter(chain, move |instance, _node_id, depth, el| {
        let active = {
            let frames = frames_enter.borrow();
            let parent = frames.peek(ROOT_STACK).expect("root frame");
            down_ok(extent, parent.depth, depth)
        };
        frames_enter.borrow_mut().push(EXPAND_STACK, Frame::new(instance, depth, active));
        if !active {
            return;
        }
        let item = eval_at_enter(&map, expand_to, el, registry_enter.as_ref());
        fold_enter.borrow_mut().as_mut().expect("fold still live").append(registry_enter.as_ref(), item);
    });

    engine.on_exit(chain, move |_instance| {
        frames_exit.borrow_mut().pop(EXPAND_STACK);
    });

    Streaming::new(engine.into_handlers(), result)
}

struct FlatProgram {
    expand: DownExpand,
    expand_to: ColId,
    map: Expr,
    monoid: MonoidId,
}

fn analyze(graph: &mut Graph) -> FlatProgram {
    let ret = graph.ret.as_ref().expect("streaming compile: Return required");
    assert!(graph.sinks.is_empty(), "streaming compile: sinks not supported yet");

    let fold_id = ret.input;
    let RelationalOperator::Fold { input: fold_input, map: fold_map, monoid, output } = graph.rel(fold_id) else {
        panic!("streaming compile: expected Fold as Return input");
    };
    assert!(matches!(&ret.value, Expr::Col(c) if c == output), "streaming compile: Return must read the Fold output column");

    let fold_input = *fold_input;
    let fold_map = fold_map.clone();
    let monoid = *monoid;

    let (expand_id, map) = match graph.rel(fold_input) {
        RelationalOperator::Expand { .. } => (fold_input, fold_map),
        RelationalOperator::Project { input, outputs } => {
            let Expr::Col(col) = &fold_map else {
                panic!("streaming compile: Fold after Project must map a column");
            };
            let expr =
                outputs.iter().find(|(c, _)| c == col).map(|(_, e)| e.clone()).unwrap_or_else(|| panic!("streaming compile: Fold column missing from Project"));
            (*input, expr)
        },
        other => panic!("streaming compile: Fold input must be Expand or Project, got {other:?}"),
    };

    let (expand_input, expand_to, path) = match &mut graph.relations[expand_id.raw() as usize] {
        RelationalOperator::Expand { input, to, path, .. } => {
            let path = std::mem::replace(path, Path::Empty);
            (*input, *to, path)
        },
        _ => panic!("streaming compile: expected Expand"),
    };
    assert!(matches!(graph.rel(expand_input), RelationalOperator::Root { .. }), "streaming compile: Expand must be from Root");

    let expand = take_down_expand(path).unwrap_or_else(|| panic!("streaming compile: unsupported Expand path"));
    assert!(
        matches!(expand.extent, crate::plan::physical::down::DownExtent::Descendant),
        "streaming compile: only C* (descendant) Expand is supported"
    );

    FlatProgram { expand, expand_to, map, monoid }
}

fn eval_at_enter(expr: &Expr, expand_to: ColId, el: &El<'_, '_>, registry: &Registry) -> Value {
    match expr {
        Expr::Literal(lit) => lit.clone().into(),
        Expr::Parameter(id) => registry.param(*id).clone(),
        Expr::Col(id) => panic!("streaming compile: unbound column {} at Enter", id.raw()),
        Expr::Field { node, projection } => {
            assert_eq!(*node, expand_to, "streaming compile: Field must read the Expand destination column");
            match projection {
                Projection::Attr(name) => match el.get_attribute(name) {
                    Some(v) => Value::str(v),
                    None => Value::Null,
                },
                Projection::Tag => Value::str(el.tag_name()),
                Projection::Text => panic!("streaming compile: text Field needs Exit readiness"),
            }
        },
        Expr::Apply { op, args } => {
            let vals: Vec<Value> = args.iter().map(|a| eval_at_enter(a, expand_to, el, registry)).collect();
            registry.apply(*op, &vals)
        },
    }
}

#[cfg(test)]
#[path = "compile.test.rs"]
mod test;
