use std::cell::RefCell;
use std::rc::Rc;

use lol_html::{HtmlRewriter, Settings};

use super::Matcher;
use crate::SettingsExt;

type Ids = Rc<RefCell<Vec<String>>>;

fn exec(handlers: Vec<crate::HandlerEntry<'static, 'static>>, html: &str) {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(handlers), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
}

fn check(matcher: Matcher, html: &str, expected: &str) {
    check_all(vec![matcher], html, expected);
}

/// Collects the ids reported by all `matchers` sharing one rewriter.
fn check_all(matchers: Vec<Matcher>, html: &str, expected: &str) {
    let ids: Ids = Rc::new(RefCell::new(vec![]));
    let handlers = matchers
        .into_iter()
        .flat_map(|matcher| {
            let capture = ids.clone();
            matcher.on_each(move |el| capture.borrow_mut().push(el.get_attribute("id").unwrap_or_default())).build()
        })
        .collect();
    exec(handlers, html);
    assert_eq!(ids.borrow().join(" "), expected);
}

#[test]
fn matcher() {
    // css() — simple tag selector
    check(Matcher::new().css("div"), r#"<div id="a"></div><span id="b"></span>"#, "a");

    // css() — multiple matches
    check(Matcher::new().css("p"), r#"<p id="a"></p><p id="b"></p>"#, "a b");

    // css() — class selector
    check(Matcher::new().css(".item"), r#"<div class="item" id="a"></div><div id="b"></div>"#, "a");

    // css() — descendant selector
    check(Matcher::new().css(".root .item"), r#"<div class="root"><span class="item" id="a"></span></div><span class="item" id="b"></span>"#, "a");

    // filter() — attribute presence
    check(
        Matcher::new().filter(".item", |el| el.get_attribute("data-x").is_some()),
        r#"<div class="item" id="a" data-x="1"></div><div class="item" id="b"></div>"#,
        "a",
    );

    // filter() — attribute value match
    check(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span id="a" data-x="main"></span><span id="b" data-x="other"></span><span id="c"></span>"#,
        "a",
    );

    // filter() — none pass predicate
    check(Matcher::new().filter("div", |el| el.has_attribute("data-missing")), r#"<div id="a"></div><div id="b"></div>"#, "");

    // chain — equivalent of the "a img" selector
    check(Matcher::new().css("a").css("img"), r#"<a id="x"><img id="a"></a><img id="b">"#, "a");

    // chain — ancestors do not have to be parents
    check(Matcher::new().css(".root").css("i"), r#"<div class="root"><span><i id="a"></i></span></div><i id="b"></i>"#, "a");

    // chain — three levels
    check(Matcher::new().css("ul").css("li").css("b"), r#"<ul><li><b id="a"></b></li></ul><li><b id="b"></b></li>"#, "a");

    // chain — an ancestor stops matching once it is closed
    check(Matcher::new().css("div").css("span"), r#"<div><span id="a"></span></div><span id="b"></span>"#, "a");

    // chain — an element is not its own ancestor
    check(Matcher::new().css("div").css("div"), r#"<div id="x"><div id="a"></div></div>"#, "a");

    // chain — filter() as an ancestor
    check(
        Matcher::new().filter("div", |el| el.get_attribute("data").as_deref() == Some("main")).css("span"),
        r#"<div data="main"><span id="a"></span></div><div><span id="b"></span></div>"#,
        "a",
    );

    // chain — a void element never becomes an ancestor
    check(Matcher::new().css("img").css("span"), r#"<img id="x"><span id="y"></span>"#, "");

    // chain — an implicitly closed ancestor stays open until its parent closes (known limitation)
    check(Matcher::new().css("p").css("p"), r#"<div><p id="x">one<p id="a">two</div>"#, "a");

    // not() — equivalent of the "*:not(a)" selector
    check(Matcher::new().not(Matcher::new().css("a")), r#"<a id="x"></a><span id="a"></span>"#, "a");

    // not() — as a link of a chain
    check(Matcher::new().css("div").not(Matcher::new().css("span")), r#"<div id="x"><span id="y"></span><i id="a"></i></div>"#, "a");

    // not() — negation of a whole chain
    check(Matcher::new().not(Matcher::new().css("div").css("span")), r#"<div id="a"><span id="x"></span></div><span id="b"></span>"#, "a b");

    // every() — all matchers have to match
    check(
        Matcher::new().every(vec![Matcher::new().css("div"), Matcher::new().css(".item")]),
        r#"<div class="item" id="a"></div><div id="x"></div><span class="item" id="y"></span>"#,
        "a",
    );

    // every() — combined with not()
    check(
        Matcher::new().every(vec![Matcher::new().css(".item"), Matcher::new().not(Matcher::new().css("span"))]),
        r#"<div class="item" id="a"></div><span class="item" id="x"></span>"#,
        "a",
    );

    // any() — at least one matcher has to match
    check(Matcher::new().any(vec![Matcher::new().css("div"), Matcher::new().css("span")]), r#"<div id="a"></div><span id="b"></span><p id="x"></p>"#, "a b");

    // any() — an element matched by several selectors is reported once
    check(Matcher::new().any(vec![Matcher::new().css("div"), Matcher::new().css(".item")]), r#"<div class="item" id="a"></div>"#, "a");

    // any() — as an ancestor of a chain
    check(
        Matcher::new().any(vec![Matcher::new().css("main"), Matcher::new().css("aside")]).css("img"),
        r#"<main><img id="a"></main><aside><img id="b"></aside><div><img id="x"></div>"#,
        "a b",
    );

    // several matchers keep their own ancestry when added to the same rewriter
    check_all(
        vec![Matcher::new().css("div").css("i"), Matcher::new().css("section").css("i")],
        r#"<div><i id="a"></i></div><section><i id="b"></i></section><i id="x"></i>"#,
        "a b",
    );
}
