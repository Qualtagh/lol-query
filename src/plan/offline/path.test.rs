use super::eval;
use crate::matcher::MatchPattern;
use crate::plan::offline::dom::Dom;
use crate::plan::offline::test_util::at;
use crate::plan::representation::id::NodeId;
use crate::plan::representation::path::{NodeTest, Path};

fn frag(html: &str) -> Dom {
    Dom::parse_fragment(html)
}

fn css(selector: &str) -> NodeTest {
    MatchPattern::new().css(selector).into()
}

fn ids(dom: &Dom, nodes: &[NodeId]) -> Vec<String> {
    nodes
        .iter()
        .map(|&id| {
            if id == dom.root() {
                return "@root".to_string();
            }
            dom.attr(id, "id").map(str::to_string).unwrap_or_else(|| dom.tag(id).unwrap_or("?").to_string())
        })
        .collect()
}

fn check(dom: &Dom, source: &str, path: Path, expect: &[&str]) {
    let got = eval(dom, &path, at(dom, source));
    assert_eq!(ids(dom, &got), expect, "path from `{source}`");
}

#[test]
fn axes_compose_filter() {
    let dom = frag(
        r#"
            <ul id="list">
                <li id="a">A</li>
                <li id="b">
                    <span id="s">B</span>
                </li>
                <li id="c">C</li>
            </ul>
        "#,
    );

    // C — immediate children, document order
    check(&dom, "list", Path::child_axis(), &["a", "b", "c"]);

    // C* — descendants, document order (nested span before following li)
    check(&dom, "list", Path::descendants(), &["a", "b", "s", "c"]);

    // P / P* — parent, then ancestors nearest-first
    check(&dom, "s", Path::parent(), &["b"]);
    check(&dom, "s", Path::parent_axis().plus(), &["b", "list", "html", "@root"]);

    // N / B
    check(&dom, "a", Path::prev(), &[]);
    check(&dom, "a", Path::next(), &["b"]);
    check(&dom, "b", Path::prev(), &["a"]);
    check(&dom, "b", Path::next(), &["c"]);
    check(&dom, "s", Path::prev(), &[]);
    check(&dom, "s", Path::next(), &[]);
    check(&dom, "c", Path::prev(), &["b"]);
    check(&dom, "c", Path::next(), &[]);

    // N* document order; B* nearest-first
    check(&dom, "a", Path::next_axis().plus(), &["b", "c"]);
    check(&dom, "c", Path::prev_axis().plus(), &["b", "a"]);

    // compose P;N
    check(&dom, "s", Path::parent().then(Path::next()), &["c"]);

    // C ; [css] — cheerio children().filter()
    check(&dom, "list", Path::child_axis().filter(css("#b")), &["b"]);
    check(&dom, "list", Path::child_axis().filter(css("span")), &[]);

    // C* ; [css]
    check(&dom, "list", Path::find(css("li")), &["a", "b", "c"]);
    check(&dom, "list", Path::find(css("#s")), &["s"]);

    // boolean test arms
    check(&dom, "list", Path::children(css("li").and(css("#b").not())), &["a", "c"]);
    check(&dom, "a", Path::test(NodeTest::True), &["a"]);
    check(&dom, "a", Path::test(NodeTest::False), &[]);

    // negative: no next sibling; empty path; filter miss
    check(&dom, "c", Path::next(), &[]);
    check(&dom, "list", Path::empty(), &[]);
    check(&dom, "list", Path::find(css("#missing")), &[]);
}
