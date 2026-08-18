use super::{Builder, ScopeRef};
use crate::plan::Plan;
use crate::plan::offline::eval::EvalExt;
use crate::plan::representation::registry::Value;
use crate::plan::test_util::unwrap_or_empty_str;

fn strings(value: &Value) -> Vec<&str> {
    value.as_list().expect("expected List").iter().map(|v| v.as_str().expect("expected Str")).collect()
}

fn check(plan: &Plan, html: &str, expected: &[&str]) {
    assert_eq!(strings(&plan.eval(html)), expected);
}

fn ids(scope: ScopeRef) -> Plan {
    scope.attr("id").map(unwrap_or_empty_str).fold().build_plan()
}

/// Same cases as offline relational test: builder sugar -> Plan -> offline sequences.
#[test]
fn collect_item_ids() {
    let plan = ids(Builder::new().root().find_css(".item"));

    check(
        &plan,
        r#"
            <div class="item" id="a"></div>
            <span>skip</span>
            <div class="item" id="b"></div>
            <div class="item"></div>
        "#,
        &["a", "b", ""],
    );

    check(
        &plan,
        r#"
            <div class="item" id="outer">
                <div class="item" id="inner"></div>
            </div>
            <div class="item" id="sibling"></div>
        "#,
        &["outer", "inner", "sibling"],
    );
}

#[test]
fn nested_expand() {
    let tree = r#"
        <div class="parent" id="p1">
            <section id="a1">
                <div class="child" id="g1"></div>
            </section>
            <div class="child" id="d1">
                <div class="child" id="g2"></div>
            </div>
        </div>
        <div class="parent" id="p2">
            <div class="child" id="d2"></div>
        </div>
        <div class="child" id="orphan"></div>
    "#;

    check(&ids(Builder::new().root().find_css(".parent").find_css(".child")), tree, &["g1", "d1", "g2", "d2"]);
    check(&ids(Builder::new().root().find_css(".parent").children().children()), tree, &["g1", "g2"]);
    check(&ids(Builder::new().root().find_css(".parent").children_css(".child")), tree, &["d1", "d2"]);

    let nested_parents = r#"
        <div class="parent" id="outer">
            <div class="parent" id="inner">
                <div class="child" id="c"></div>
            </div>
        </div>
    "#;
    check(&ids(Builder::new().root().find_css(".parent").find_css(".child")), nested_parents, &["c", "c"]);
    check(&ids(Builder::new().root().find_css(".parent").children_css(".child")), nested_parents, &["c"]);
}
