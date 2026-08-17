use super::Builder;
use crate::plan::offline::eval::EvalExt;
use crate::plan::representation::registry::Value;
use crate::plan::test_util::unwrap_or_empty_str;

fn strings(value: &Value) -> Vec<&str> {
    value.as_list().expect("expected List").iter().map(|v| v.as_str().expect("expected Str")).collect()
}

/// Same cases as offline relational test: builder sugar -> Plan -> offline sequences.
#[test]
fn collect_item_ids() {
    let plan = Builder::new().root().find_css(".item").attr("id").map(unwrap_or_empty_str).fold().build_plan();

    let flat = r#"
        <div class="item" id="a"></div>
        <span>skip</span>
        <div class="item" id="b"></div>
        <div class="item"></div>
        "#;
    assert_eq!(strings(&plan.eval(flat)), ["a", "b", ""]);

    let nested = r#"
        <div class="item" id="outer">
            <div class="item" id="inner"></div>
        </div>
        <div class="item" id="sibling"></div>
        "#;
    assert_eq!(strings(&plan.eval(nested)), ["outer", "inner", "sibling"]);
}
