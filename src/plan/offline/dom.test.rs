use scraper::Selector;

use super::Dom;
use crate::plan::offline::test_util::at;

fn frag(html: &str) -> Dom {
    Dom::parse_fragment(html)
}

fn doc(html: &str) -> Dom {
    Dom::parse_document(html)
}

fn check_parent(dom: &Dom, child: &str, parent: Option<&str>) {
    assert_eq!(dom.parent(at(dom, child)), parent.map(|p| at(dom, p)));
}

fn check_first_child(dom: &Dom, parent: &str, child: Option<&str>) {
    assert_eq!(dom.first_child(at(dom, parent)), child.map(|c| at(dom, c)));
}

fn check_next(dom: &Dom, from: &str, to: Option<&str>) {
    assert_eq!(dom.next_sibling(at(dom, from)), to.map(|t| at(dom, t)));
}

fn check_prev(dom: &Dom, from: &str, to: Option<&str>) {
    assert_eq!(dom.prev_sibling(at(dom, from)), to.map(|t| at(dom, t)));
}

fn check_tag(dom: &Dom, node: &str, tag: Option<&str>) {
    assert_eq!(dom.tag(at(dom, node)), tag);
}

fn check_attr(dom: &Dom, node: &str, name: &str, value: Option<&str>) {
    assert_eq!(dom.attr(at(dom, node), name), value);
}

fn check_text(dom: &Dom, node: &str, text: &str) {
    assert_eq!(dom.text(at(dom, node)), text);
}

fn check_matches(dom: &Dom, node: &str, css: &str, expect: bool) {
    let sel = Selector::parse(css).unwrap_or_else(|e| panic!("selector `{css}`: {e:?}"));
    assert_eq!(dom.matches(at(dom, node), &sel), expect);
}

#[test]
fn list() {
    let dom = frag(r#"<ul id="list"><li id="a">A</li><li id="b">B</li><li id="c">C</li></ul>"#);
    check_parent(&dom, "@root", None);
    check_parent(&dom, "html", Some("@root"));
    check_parent(&dom, "list", Some("html"));
    check_parent(&dom, "a", Some("list"));
    check_parent(&dom, "b", Some("list"));
    check_parent(&dom, "c", Some("list"));

    check_first_child(&dom, "@root", Some("html"));
    check_first_child(&dom, "html", Some("list"));
    check_first_child(&dom, "list", Some("a"));
    check_tag(&dom, "html", Some("html"));
    check_tag(&dom, "list", Some("ul"));
    check_attr(&dom, "list", "id", Some("list"));
    check_attr(&dom, "a", "id", Some("a"));
    check_attr(&dom, "b", "id", Some("b"));
    check_attr(&dom, "c", "id", Some("c"));

    check_next(&dom, "a", Some("b"));
    check_next(&dom, "b", Some("c"));
    check_next(&dom, "c", None);
    check_prev(&dom, "c", Some("b"));
    check_prev(&dom, "b", Some("a"));
    check_prev(&dom, "a", None);
}

#[test]
fn mixed() {
    let dom = frag(r#"<div id="box">Hello <b>world</b>!</div>"#);
    check_text(&dom, "box", "Hello world!");
    check_text(&dom, "@root", "");
    check_text(&dom, "b", "world");
    check_tag(&dom, "b", Some("b"));
    check_first_child(&dom, "box", Some("b"));
    check_matches(&dom, "box", "#box", true);
    check_matches(&dom, "box", "b", false);
    check_matches(&dom, "@root", "#box", false);
    check_matches(&dom, "b", "b", true);
}

#[test]
fn page() {
    let dom = doc("<!DOCTYPE html><html><body><p id=\"p\">x</p></body></html>");
    check_first_child(&dom, "@root", Some("html"));
    check_parent(&dom, "html", Some("@root"));
    check_parent(&dom, "p", Some("body"));
    check_attr(&dom, "p", "id", Some("p"));
    check_text(&dom, "p", "x");
}
