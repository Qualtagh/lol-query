use std::cell::RefCell;
use std::rc::Rc;

use lol_html::{HtmlRewriter, Settings};

use super::Matcher;
use crate::SettingsExt;

#[derive(Clone, Copy)]
struct Capture {
    each: bool,
    text: bool,
    comment: bool,
}

impl Capture {
    const ALL: Self = Self { each: true, text: true, comment: true };
    const COMMENT: Self = Self { each: false, text: false, comment: true };
    const EACH: Self = Self { each: true, text: false, comment: false };
    const TEXT: Self = Self { each: false, text: true, comment: false };
}

type Log = Rc<RefCell<Vec<String>>>;

fn exec(handlers: Vec<crate::HandlerEntry<'static, 'static>>, html: &str) {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(handlers), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
}

fn attach(matcher: Matcher, log: Log, capture: Capture) -> Vec<crate::HandlerEntry<'static, 'static>> {
    let mut matcher = matcher;
    if capture.each {
        let log = log.clone();
        matcher = matcher.on_each(move |el| log.borrow_mut().push(el.get_attribute("id").unwrap_or_default()));
    }
    if capture.text {
        let log = log.clone();
        matcher = matcher.on_text_chunk(move |chunk| {
            let s = chunk.as_str();
            if s.is_empty() {
                return;
            }
            log.borrow_mut().push(s.to_string());
        });
    }
    if capture.comment {
        let log = log.clone();
        matcher = matcher.on_comment(move |comment| {
            let s = comment.text();
            if s.is_empty() {
                return;
            }
            log.borrow_mut().push(s);
        });
    }
    matcher.build()
}

fn run(matchers: Vec<Matcher>, html: &str, capture: Capture, expected: &str) {
    let log: Log = Rc::new(RefCell::new(vec![]));
    let handlers = matchers.into_iter().flat_map(|matcher| attach(matcher, log.clone(), capture)).collect();
    exec(handlers, html);
    assert_eq!(log.borrow().join(" "), expected);
}

fn check(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::EACH, expected);
}

fn check_text(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::TEXT, expected);
}

fn check_comment(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::COMMENT, expected);
}

fn check_all(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::ALL, expected);
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
    run(
        vec![Matcher::new().css("div").css("i"), Matcher::new().css("section").css("i")],
        r#"<div><i id="a"></i></div><section><i id="b"></i></section><i id="x"></i>"#,
        Capture::EACH,
        "a b",
    );

    // build paths — css/filter/chain x on_each/on_text_chunk/on_comment/all
    check(Matcher::new().css("p"), r#"<p id="a"></p><div id="x"></div>"#, "a");
    check_text(Matcher::new().css("p"), r#"<p>hi</p><div>x</div>"#, "hi");
    check_comment(Matcher::new().css("p"), r#"<p><!--note--></p><div><!--skip--></div>"#, "note");
    check_all(Matcher::new().css("p"), r#"<p id="a">one<!--c1--></p><div id="x">skip<!--miss--></div><p id="b">two<!--c2--></p>"#, "a one c1 b two c2");

    check(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main" id="a"></span><span id="b"></span>"#,
        "a",
    );
    check_text(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main">yes</span><span data-x="other">no</span>"#,
        "yes",
    );
    check_comment(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main"><!--yes--></span><span data-x="other"><!--no--></span>"#,
        "yes",
    );
    check_all(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main" id="a"><!--c1-->yes</span><span id="b">no<!--c2--></span>"#,
        "a c1 yes",
    );

    check(Matcher::new().css(".root").css("span"), r#"<div class="root"><span id="a"></span></div><span id="b"></span>"#, "a");
    check_text(Matcher::new().css(".root").css("span"), r#"<div class="root">outer<span>inner</span></div><span>other</span>"#, "inner");
    check_comment(Matcher::new().css(".root").css("span"), r#"<div class="root">outer<span><!--inner--></span></div><span><!--other--></span>"#, "inner");
    check_all(
        Matcher::new().css(".root").css("span"),
        r#"<div class="root">outer<span id="a">inner<!--c1--></span><!--div comment--></div><span id="b">other<!--c2--></span>"#,
        "a inner c1",
    );

    // on_text_chunk() — text in nested elements is included
    check_text(Matcher::new().css("p"), r#"<p>one <b>two</b></p>"#, "one  two");

    // on_text_chunk() — text outside the excluded selector
    check_text(Matcher::new().not(Matcher::new().css("a")), r#"<a>skip</a><span>keep</span>"#, "keep");

    // on_comment() — comments in nested elements are included
    check_comment(Matcher::new().css("p"), r#"<p><!--one--><b><!--two--></b></p>"#, "one two");

    // on_comment() — comments outside the excluded selector
    check_comment(Matcher::new().not(Matcher::new().css("a")), r#"<a><!--skip--></a><span><!--keep--></span>"#, "keep");

    // on_comment() — comment inside a comment
    check_comment(Matcher::new().css("a"), r#"<a><!--outer <!--inner--></a><span><!--skip--></span>"#, "outer <!--inner");
}
