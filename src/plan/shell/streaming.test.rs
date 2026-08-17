//! Shell streaming surface: streaming() -> rewrite -> take.
//!
//! Unlike [`crate::Matcher`], the Fold/Return lives in the plan: no caller-owned
//! accumulator in callbacks. Apply (`map`) is plan algebra, not ad-hoc callback logic.

use lol_html::{HtmlRewriter, Settings};

use super::Streaming;
use crate::SettingsExt;
use crate::plan::builder::Builder;
use crate::plan::representation::registry::Value;
use crate::plan::shell::StreamingExt;
use crate::plan::test_util::unwrap_or_empty_str;

fn strings(value: &Value) -> Vec<&str> {
    value.as_list().expect("expected List").iter().map(|v| v.as_str().expect("expected Str")).collect()
}

fn prefix_id(args: &[Value]) -> Value {
    assert!(args.len() == 1);
    Value::str(format!("id:{}", args[0].as_str().expect("Str")))
}

fn run(mut streaming: Streaming, html: &str) -> Value {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(streaming.into_handlers()), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
    streaming.take()
}

fn check_ids(html: &str, expected: &[&str]) {
    let plan = Builder::new().root().find_css(".item").attr("id").map(unwrap_or_empty_str).fold().build_plan();
    assert_eq!(strings(&run(plan.streaming(), html)), expected);
}

fn check_tags(html: &str, expected: &[&str]) {
    let plan = Builder::new().root().find_css(".item").tag().fold().build_plan();
    assert_eq!(strings(&run(plan.streaming(), html)), expected);
}

fn check_prefixed(html: &str, expected: &[&str]) {
    let plan = Builder::new().root().find_css(".item").attr("id").map(unwrap_or_empty_str).map(prefix_id).fold().build_plan();
    assert_eq!(strings(&run(plan.streaming(), html)), expected);
}

#[test]
fn plan_streaming_take() {
    // Plan-owned Fold/Return: result via take(), not a Vec closed over in on_each.
    check_ids(
        r#"
        <div class="item" id="a"></div>
        <span>skip</span>
        <div class="item" id="b"></div>
        "#,
        &["a", "b"],
    );

    // Missing attr -> Null -> map to "" inside the plan (Apply), then fold.
    check_ids(
        r#"
        <div class="item" id="a"></div>
        <div class="item"></div>
        <div class="item" id="c"></div>
        "#,
        &["a", "", "c"],
    );

    // Field(tag) fold — same handlers/take surface, different projection.
    check_tags(
        r#"
        <div class="item" id="a"></div>
        <span class="item" id="b"></span>
        "#,
        &["div", "span"],
    );

    // Chained Apply before Fold (Matcher has no plan-level map pipeline / Return).
    check_prefixed(
        r#"
        <div class="item" id="a"></div>
        <div class="item" id="b"></div>
        "#,
        &["id:a", "id:b"],
    );
}
