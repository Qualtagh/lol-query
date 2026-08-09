use super::{Registry, Value, eval};
use crate::matcher::MatchPattern;
use crate::plan::offline::dom::Dom;
use crate::plan::representation::expr::Expr;
use crate::plan::representation::id::{ColId, MonoidId, RelationId};
use crate::plan::representation::path::Path;
use crate::plan::representation::relational::{Plan, RelationalOperator, Return};

fn css(selector: &str) -> Path {
    Path::find(MatchPattern::new().css(selector))
}

fn strings(value: &Value) -> Vec<&str> {
    value.as_list().expect("expected List").iter().map(|v| v.as_str().expect("expected Str")).collect()
}

/// Caller-side `unwrap_or_default` for missing attrs (`Null` → `""`).
fn unwrap_or_empty_str(args: &[Value]) -> Value {
    assert!(args.len() == 1, "unwrap_or_empty_str expects one argument");
    match &args[0] {
        Value::Null => Value::str(""),
        other => other.clone(),
    }
}

fn register_list_push(registry: &mut Registry) -> MonoidId {
    registry.register_monoid(Value::List(Vec::new()), |acc, item| {
        let Value::List(mut xs) = acc else {
            panic!("list_push monoid acc must be List");
        };
        xs.push(item);
        Value::List(xs)
    })
}

/// Example:
///
/// ```rust
/// #[lolquery]
/// fn ids(dom: &Dom) -> Vec<String> {
///     let mut out = vec![];
///     for el in dom.find(".item") {
///         out.push(el.attr("id").unwrap_or_default());
///     }
///     out
/// }
/// ```
///
/// Plan: Root → Expand(C⁺;[.item]) → Project(Apply(unwrap, Field id)) → Fold → Return
fn item_ids_plan(registry: &mut Registry) -> Plan {
    let unwrap = registry.register_apply(unwrap_or_empty_str);
    let list = register_list_push(registry);

    let root_col = ColId::new(0);
    let el_col = ColId::new(1);
    let id_col = ColId::new(2);
    let bag_col = ColId::new(3);

    let root = RelationId::new(0);
    let items = RelationId::new(1);
    let ids = RelationId::new(2);
    let bag = RelationId::new(3);

    Plan::new(
        [
            RelationalOperator::root(root_col),
            RelationalOperator::expand(root, root_col, el_col, css(".item")),
            RelationalOperator::project(items, [(id_col, Expr::apply(unwrap, [Expr::attr(el_col, "id")]))]),
            RelationalOperator::fold(ids, Expr::col(id_col), list, bag_col),
        ],
        [],
        Some(Return::new(bag, Expr::col(bag_col))),
    )
}

#[test]
fn collect_item_ids() {
    let mut registry = Registry::new();
    let plan = item_ids_plan(&mut registry);

    let flat = r#"
        <div class="item" id="a"></div>
        <span>skip</span>
        <div class="item" id="b"></div>
        <div class="item"></div>
    "#;
    let dom = Dom::parse_fragment(flat);
    assert_eq!(strings(&eval(&dom, &plan, &registry)), ["a", "b", ""]);

    let nested = r#"
        <div class="item" id="outer">
            <div class="item" id="inner"></div>
        </div>
        <div class="item" id="sibling"></div>
    "#;
    let dom = Dom::parse_fragment(nested);
    assert_eq!(strings(&eval(&dom, &plan, &registry)), ["outer", "inner", "sibling"]);
}
