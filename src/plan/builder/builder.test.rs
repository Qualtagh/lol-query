use super::Builder;
use crate::plan::offline::dom::Dom;
use crate::plan::offline::relational::eval;
use crate::plan::registry::Value;

fn strings(value: &crate::plan::registry::Value) -> Vec<&str> {
    value.as_list().expect("expected List").iter().map(|v| v.as_str().expect("expected Str")).collect()
}

fn unwrap_or_empty_str(args: &[Value]) -> Value {
    assert!(args.len() == 1, "unwrap_or_empty_str expects one argument");
    match &args[0] {
        Value::Null => Value::str(""),
        other => other.clone(),
    }
}

/// Same cases as offline relational test: builder sugar → IR → offline sequences.
#[test]
fn collect_item_ids() {
    let builder = Builder::new();
    let (plan, registry) = builder.root().find_css(".item").attr("id").map(unwrap_or_empty_str).fold().build_plan();

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
