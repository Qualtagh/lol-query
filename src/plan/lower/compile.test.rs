use lol_html::{HtmlRewriter, Settings};

use crate::SettingsExt;
use crate::plan::Plan;
use crate::plan::builder::Builder;
use crate::plan::offline::eval::EvalExt;
use crate::plan::representation::registry::Value;
use crate::plan::shell::{Streaming, StreamingExt};
use crate::plan::test_util::unwrap_or_empty_str;

fn strings(value: &Value) -> Vec<&str> {
    value.as_list().expect("expected List").iter().map(|v| v.as_str().expect("expected Str")).collect()
}

fn item_ids_plan() -> Plan {
    Builder::new().root().find_css(".item").attr("id").map(unwrap_or_empty_str).fold().build_plan()
}

fn run_streaming(mut streaming: Streaming, html: &str) -> Value {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(streaming.into_handlers()), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
    streaming.take()
}

/// Streaming sequence equals offline evaluator (and the expected gold list).
fn check_seq(html: &str, expected: &[&str]) {
    let plan = item_ids_plan();
    assert_eq!(strings(&plan.eval(html)), expected);
    assert_eq!(strings(&run_streaming(plan.streaming(), html)), expected);
}

#[test]
fn flat_expand_fold_return() {
    check_seq(
        r#"
        <div class="item" id="a"></div>
        <span>skip</span>
        <div class="item" id="b"></div>
        <div class="item"></div>
        "#,
        &["a", "b", ""],
    );

    check_seq(
        r#"
        <div class="item" id="outer">
            <div class="item" id="inner"></div>
        </div>
        <div class="item" id="sibling"></div>
        "#,
        &["outer", "inner", "sibling"],
    );
}
