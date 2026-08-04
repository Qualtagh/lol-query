use std::cell::RefCell;
use std::rc::Rc;

use lol_html::html_content::ContentType;
use lol_html::{HtmlRewriter, Settings};

use super::Matcher;
use crate::SettingsExt;

type Log = Rc<RefCell<Vec<String>>>;

#[derive(Clone, Default)]
struct Logs {
    each: Option<Log>,
    text_chunk: Option<Log>,
    text: Option<Log>,
    comment: Option<Log>,
}

#[derive(Default)]
struct Expected<'a> {
    each: Option<&'a str>,
    text_chunk: Option<&'a str>,
    text: Option<&'a str>,
    comment: Option<&'a str>,
}

fn log() -> Log {
    Rc::new(RefCell::new(vec![]))
}

fn exec(handlers: Vec<crate::HandlerEntry<'static, 'static>>, html: &str) {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(handlers), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
}

fn attach(matcher: Matcher, logs: &Logs) -> Vec<crate::HandlerEntry<'static, 'static>> {
    let mut matcher = matcher;
    if let Some(log) = logs.each.clone() {
        matcher = matcher.on_each(move |el| log.borrow_mut().push(el.get_attribute("id").unwrap_or_default()));
    }
    if let Some(log) = logs.text_chunk.clone() {
        matcher = matcher.on_text_chunk(move |chunk| {
            let s = chunk.as_str();
            if s.is_empty() {
                return;
            }
            log.borrow_mut().push(s.to_string());
        });
    }
    if let Some(log) = logs.text.clone() {
        matcher = matcher.on_text(move |aggregated| {
            if aggregated.is_empty() {
                return;
            }
            log.borrow_mut().push(aggregated.to_string());
        });
    }
    if let Some(log) = logs.comment.clone() {
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

fn check_logs(matchers: Vec<Matcher>, html: &str, expected: Expected<'_>) {
    let logs = Logs {
        each: expected.each.map(|_| log()),
        text_chunk: expected.text_chunk.map(|_| log()),
        text: expected.text.map(|_| log()),
        comment: expected.comment.map(|_| log()),
    };
    exec(matchers.into_iter().flat_map(|matcher| attach(matcher, &logs)).collect(), html);
    for (log, expected) in
        [(&logs.each, expected.each), (&logs.comment, expected.comment), (&logs.text, expected.text), (&logs.text_chunk, expected.text_chunk)]
    {
        if let (Some(log), Some(expected)) = (log, expected) {
            assert_eq!(log.borrow().join(" "), expected);
        }
    }
}

fn check(matcher: Matcher, html: &str, expected: &str) {
    check_logs(vec![matcher], html, Expected { each: Some(expected), ..Default::default() });
}

fn check_text(matcher: Matcher, html: &str, expected: &str) {
    check_logs(vec![matcher], html, Expected { text_chunk: Some(expected), ..Default::default() });
}

fn check_aggregated(matcher: Matcher, html: &str, expected: &str) {
    check_logs(vec![matcher], html, Expected { text: Some(expected), ..Default::default() });
}

fn check_comment(matcher: Matcher, html: &str, expected: &str) {
    check_logs(vec![matcher], html, Expected { comment: Some(expected), ..Default::default() });
}

fn check_all(matcher: Matcher, html: &str, expected_each: &str, expected_comment: &str, expected_text: &str, expected_text_chunk: &str) {
    check_logs(vec![matcher], html, Expected {
        each: Some(expected_each),
        comment: Some(expected_comment),
        text: Some(expected_text),
        text_chunk: Some(expected_text_chunk),
    });
}

fn rewrite(handlers: Vec<crate::HandlerEntry<'static, 'static>>, html: &str) -> String {
    let mut out = vec![];
    let mut rw = HtmlRewriter::new(Settings::new().add_handlers(handlers), |c: &[u8]| out.extend_from_slice(c));
    rw.write(html.as_bytes()).unwrap();
    rw.end().unwrap();
    String::from_utf8(out).unwrap()
}

fn check_html(matcher: Matcher, html: &str, expected: &str) {
    assert_eq!(rewrite(matcher.build(), html), expected);
}

fn nested_html(depth: usize) -> String {
    let mut html = String::with_capacity(depth * 32);
    for level in 0..depth {
        let class = if level == 0 {
            "root"
        } else if level == depth / 3 {
            "red"
        } else if level == depth * 2 / 3 {
            "blue"
        } else if level + 1 == depth {
            "target"
        } else {
            "node"
        };
        html.push_str("<div class=\"");
        html.push_str(class);
        html.push_str("\">");
    }
    html.push_str(&"</div>".repeat(depth));
    html
}

fn matcher_operation_count(depth: usize) -> usize {
    let matches = Rc::new(RefCell::new(0));
    let matches_for_callback = matches.clone();
    let handlers = Matcher::new()
        .css(".root")
        .gap_with_every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")])
        .css(".target")
        .on_each(move |_| *matches_for_callback.borrow_mut() += 1)
        .build();

    crate::general_regex::reset_operation_count();
    exec(handlers, &nested_html(depth));
    assert_eq!(*matches.borrow(), 1);
    crate::general_regex::operation_count()
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

    // chain — repeated universal selectors still consume distinct ancestry levels
    check(
        Matcher::new().css("*").css("*").css("*"),
        r#"<main id="x"><section id="y"><i id="a"></i><b id="b"><u id="c"></u></b></section></main><aside id="z"></aside>"#,
        "a b c",
    );

    // chain — an ancestor stops matching once it is closed
    check(Matcher::new().css("div").css("span"), r#"<div><span id="a"></span></div><span id="b"></span>"#, "a");

    // chain — an element is not its own ancestor
    check(Matcher::new().css("div").css("div"), r#"<div id="x"><div id="a"></div></div>"#, "a");

    // direct() — equivalent of the ".a > .b" selector
    check(
        Matcher::new().css(".a").direct().css(".b"),
        r#"<div class="a"><span class="b" id="a"></span><div><span class="b" id="x"></span></div></div><span class="b" id="y"></span>"#,
        "a",
    );

    // direct() — unlike a descendant chain, intermediate elements break the match
    check(
        Matcher::new().css(".root").direct().css(".item"),
        r#"<div class="root"><span class="item" id="a"></span></div><div class="root"><span><span class="item" id="x"></span></span></div>"#,
        "a",
    );

    // direct() — three direct-child hops
    check(
        Matcher::new().css("ul").direct().css("li").direct().css("b"),
        r#"<ul><li><b id="a"></b></li></ul><ul><li><span><b id="x"></b></span></li></ul><li><b id="y"></b></li>"#,
        "a",
    );

    // direct() — works with any() as the child selector
    check(
        Matcher::new().css(".root").direct().any(vec![Matcher::new().css("a"), Matcher::new().css("img")]),
        r#"<div class="root"><a id="a"></a><section><img id="x"></section></div><a id="y"></a>"#,
        "a",
    );

    // chain — filter() as an ancestor
    check(
        Matcher::new().filter("div", |el| el.get_attribute("data").as_deref() == Some("main")).css("span"),
        r#"<div data="main"><span id="a"></span></div><div><span id="b"></span></div>"#,
        "a",
    );

    // chain — a void element never becomes an ancestor
    check(Matcher::new().css("img").css("span"), r#"<img id="x"><span id="y"></span>"#, "");

    // chain — void match must not leave text_open stuck
    check_text(Matcher::new().css("div").css("img"), r#"<div><img></div><p>leak</p>"#, "");

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

    // gap_without() — empty and clear gaps match, a blocked ancestor does not
    check(
        Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css(".target"),
        r#"<div class="root"><i class="target" id="a"></i><section><i class="target" id="b"></i></section><section class="blocked"><i class="target" id="x"></i></section><section class="blocked"></section><i class="target" id="c"></i></div>"#,
        "a b c",
    );

    // gap_without() — the two boundary elements are not part of the gap
    check(
        Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css(".target"),
        r#"<div class="root blocked"><i class="target blocked" id="a"></i></div>"#,
        "a",
    );

    // gap_without() — a leading gap constrains the whole ancestry before the final selector
    check(
        Matcher::new().gap_without(Matcher::new().css(".blocked")).css(".target"),
        r#"<section><i class="target" id="a"></i></section><section class="blocked"><i class="target" id="x"></i></section><i class="target blocked" id="b"></i>"#,
        "a b",
    );

    // Consecutive gaps are concatenated, so their order can affect the match.
    check(
        Matcher::new().gap_without(Matcher::new().css(".x")).gap_without(Matcher::new().css(".y")).css(".target"),
        r#"<div class="y"><div class="x"><i class="target" id="a"></i></div></div><div class="x"><div class="y"><i class="target" id="x"></i></div></div><div><i class="target" id="b"></i></div>"#,
        "a b",
    );

    // gap_without() — the nested matcher may itself be a combinator
    check(
        Matcher::new()
            .css(".root")
            .gap_without(Matcher::new().every(vec![
                Matcher::new().css(".marker"),
                Matcher::new().not(Matcher::new().css(".allowed")),
            ]))
            .css(".target"),
        r#"<div class="root"><section class="marker allowed"><i class="target" id="a"></i></section><section class="marker"><i class="target" id="x"></i></section></div>"#,
        "a",
    );

    // gap_with_any() — at least one matching intermediate ancestor is required
    check(
        Matcher::new().css(".root").gap_with_any(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")]).css(".target"),
        r#"<div class="root"><section class="red"><i class="target" id="a"></i></section><section class="blue"><i class="target" id="b"></i></section><section><i class="target" id="x"></i></section><i class="target red" id="y"></i></div>"#,
        "a b",
    );

    // gap_with_every() — requirements may occur in either order or on one element
    check(
        Matcher::new().css(".root").gap_with_every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")]).css(".target"),
        r#"<div class="root"><section class="red"><b class="blue"><i class="target" id="a"></i></b></section><section class="blue"><b class="red"><i class="target" id="b"></i></b></section><section class="red blue"><i class="target" id="c"></i></section><section class="red"><i class="target" id="x"></i></section><section class="blue"><i class="target" id="y"></i></section></div>"#,
        "a b c",
    );

    // Different gap kinds may be concatenated; each consumes its own part of the ancestry.
    check(
        Matcher::new()
            .css(".root")
            .gap_with_any(vec![Matcher::new().css(".a")])
            .gap_with_every(vec![Matcher::new().css(".b"), Matcher::new().css(".c")])
            .css(".target"),
        r#"<div class="root"><section class="a"><div class="b"><div class="c"><i class="target" id="a"></i></div></div></section><section class="a"><div class="c"><div class="b"><i class="target" id="b"></i></div></div></section><section class="a"><div class="b c"><i class="target" id="c"></i></div></section><section class="b"><div class="c"><div class="a"><i class="target" id="x"></i></div></div></section><section class="a b c"><i class="target" id="y"></i></section></div>"#,
        "a b c",
    );

    // Multiple constrained gaps compose with nested ancestry matchers.
    check(
        Matcher::new().every(vec![
            Matcher::new().css(".target"),
            Matcher::new()
                .css(".root")
                .gap_with_every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")])
                .css(".middle")
                .gap_without(Matcher::new().any(vec![Matcher::new().css(".blocked"), Matcher::new().css(".hidden")]))
                .css(".target"),
        ]),
        r#"<div class="root"><div class="red"><div class="blue"><section class="middle"><div><i class="target" id="a"></i></div><div class="blocked"><i class="target" id="x"></i></div></section></div></div><div class="red"><section class="middle"><i class="target" id="y"></i></section></div></div>"#,
        "a",
    );

    // Gap selectors use the same element, text and comment callback path.
    check_all(
        Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css(".target"),
        r#"<div class="root"><section><i class="target" id="a">keep<!--note--></i></section><section class="blocked"><i class="target" id="x">skip<!--miss--></i></section></div>"#,
        "a",
        "note",
        "keep",
        "keep",
    );

    // several matchers keep their own ancestry when added to the same rewriter
    check_logs(
        vec![Matcher::new().css("div").css("i"), Matcher::new().css("section").css("i")],
        r#"<div><i id="a"></i></div><section><i id="b"></i></section><i id="x"></i>"#,
        Expected { each: Some("a b"), ..Default::default() },
    );

    // build paths — css/filter/chain x on_each/on_text_chunk/on_comment/on_text/all
    check(Matcher::new().css("p"), r#"<p id="a"></p><div id="x"></div>"#, "a");
    check_text(Matcher::new().css("p"), r#"<p>hi<b>there</b></p><div>x</div>"#, "hi there");
    check_aggregated(Matcher::new().css("p"), r#"<p>hi<b>there</b></p><div>x</div>"#, "hithere");
    check_comment(Matcher::new().css("p"), r#"<p><!--note--></p><div><!--skip--></div>"#, "note");
    check_all(
        Matcher::new().css("p"),
        r#"<p id="a">one<!--c1--></p><div id="x">skip<!--miss--></div><p id="b">two<!--c2--></p>"#,
        "a b",
        "c1 c2",
        "one two",
        "one two",
    );

    check(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main" id="a"></span><span id="b"></span>"#,
        "a",
    );
    check_text(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main"><b>answer:</b>yes</span><span data-x="other">no</span>"#,
        "answer: yes",
    );
    check_aggregated(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main"><b>answer:</b>yes</span><span data-x="other">no</span>"#,
        "answer:yes",
    );
    check_comment(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main"><!--yes--></span><span data-x="other"><!--no--></span>"#,
        "yes",
    );
    check_all(
        Matcher::new().filter("span", |el| el.get_attribute("data-x").as_deref() == Some("main")),
        r#"<span data-x="main" id="a"><!--c1-->yes</span><span id="b">no<!--c2--></span>"#,
        "a",
        "c1",
        "yes",
        "yes",
    );

    check(Matcher::new().css(".root").css("span"), r#"<div class="root"><span id="a"></span></div><span id="b"></span>"#, "a");
    check_text(
        Matcher::new().css(".root").css("span"),
        r#"<div class="root">outer<span>inner</span><span>inner<b>2</b></span></div><span>other</span>"#,
        "inner inner 2",
    );
    check_aggregated(
        Matcher::new().css(".root").css("span"),
        r#"<div class="root">outer<span>inner</span><span>inner<b>2</b></span></div><span>other</span>"#,
        "inner inner2",
    );
    check_comment(Matcher::new().css(".root").css("span"), r#"<div class="root">outer<span><!--inner--></span></div><span><!--other--></span>"#, "inner");
    check_all(
        Matcher::new().css(".root").css("span"),
        r#"<div class="root">outer<span id="a">inner<!--c1--></span><!--div comment--></div><span id="b">other<!--c2--></span>"#,
        "a",
        "c1",
        "inner",
        "inner",
    );

    // on_text_chunk() — text in nested elements is included
    check_text(Matcher::new().css("p"), r#"<p>one <b>two</b></p>"#, "one  two");

    // on_text_chunk() — text outside the excluded selector
    check_text(Matcher::new().not(Matcher::new().css("a")), r#"<a>skip</a><span>keep</span>"#, "keep");

    // on_text() — descendant text is concatenated once per matched element
    check_aggregated(Matcher::new().css("p"), r#"<p>one <b>two</b></p>"#, "one two");

    // on_text() — no space is inserted between sibling elements (cheerio textContent)
    check_aggregated(Matcher::new().css("div"), r#"<div>alpha<b>beta</b>gamma</div>"#, "alphabetagamma");

    // on_text() — nested matches emit once per element
    check_aggregated(Matcher::new().css("div"), r#"<div>outer<div>inner</div></div>"#, "inner outerinner");

    // on_text_chunk() vs on_text() — chunks are per text node, on_text is per element
    check_all(Matcher::new().css("p"), r#"<p id="a">one<b>two</b></p>"#, "a", "", "onetwo", "one two");

    // on_text() — text outside the excluded selector
    check_aggregated(Matcher::new().not(Matcher::new().css("a")), r#"<a>skip</a><span>keep</span>"#, "keep");

    // on_text() — chained selector
    check_aggregated(Matcher::new().css(".root").css("span"), r#"<div class="root">outer<span>inner</span></div><span>other</span>"#, "inner");

    // on_comment() — comments in nested elements are included
    check_comment(Matcher::new().css("p"), r#"<p><!--one--><b><!--two--></b></p>"#, "one two");

    // on_comment() — comments outside the excluded selector
    check_comment(Matcher::new().not(Matcher::new().css("a")), r#"<a><!--skip--></a><span><!--keep--></span>"#, "keep");

    // on_comment() — comment inside a comment
    check_comment(Matcher::new().css("a"), r#"<a><!--outer <!--inner--></a><span><!--skip--></span>"#, "outer <!--inner");
}

#[test]
fn matcher_comparisons() {
    // not() — matched ancestor must satisfy not(); gap_without() — gap ancestry must satisfy not()
    let blocked_html = r#"<div class="root"><i id="a"></i><section><i id="b"></i></section><section class="blocked"><i id="x"></i></section></div>"#;
    check(Matcher::new().css(".root").not(Matcher::new().css(".blocked")).css("i"), blocked_html, "b");
    check(Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css("i"), blocked_html, "a b");

    // every() — all matchers on the same element; gap_with_every() — matchers on distinct gap elements
    let red_blue_html = r#"<div class="root"><div class="red blue"><i id="nested-both"></i></div><section class="red"><b class="blue"><i id="via-gap"></i></b></section></div>"#;
    check(Matcher::new().css(".root").every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")]).css("i"), red_blue_html, "nested-both");
    check(
        Matcher::new().css(".root").gap_with_every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")]).css("i"),
        red_blue_html,
        "nested-both via-gap",
    );

    // direct() — same matches as the ".a > .b" CSS combinator
    let direct_html = r#"<div class="a"><span class="b" id="a"></span></div><span class="b" id="y"></span>"#;
    check(Matcher::new().css(".a > .b"), direct_html, "a");
    check(Matcher::new().css(".a").direct().css(".b"), direct_html, "a");

    // direct() + filter() — CSS ".a > .b" cannot apply an arbitrary predicate on the parent
    let filter_direct_html = r#"<div data="main"><i id="a"></i></div><div data="main"><span id="x"></span></div><div><i id="z"></i></div>"#;
    check(
        Matcher::new().filter("div", |el| el.get_attribute("data").as_deref() == Some("main")).direct().not(Matcher::new().css("span")),
        filter_direct_html,
        "a",
    );
    check(Matcher::new().css("div > i"), filter_direct_html, "a z");
}

#[test]
#[should_panic(expected = "a gap selector cannot be final in a chain")]
fn matcher_gap_cannot_be_final() {
    Matcher::new().css("div").gap_without(Matcher::new().css("span")).on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "direct() cannot be the first selector")]
fn matcher_direct_cannot_be_first() {
    Matcher::new().direct().css("div").on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "direct() must follow an element selector")]
fn matcher_direct_must_follow_element() {
    Matcher::new().css("a").gap_without(Matcher::new().css("b")).direct().css("i").on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "direct() must be followed by an element selector")]
fn matcher_direct_must_precede_element() {
    Matcher::new().css("a").direct().gap_without(Matcher::new().css("b")).css("i").on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "a matcher needs at least one selector")]
fn matcher_nested_empty() {
    Matcher::new().not(Matcher::new()).on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "a gap selector cannot be final in a chain")]
fn matcher_nested_ends_with_gap() {
    Matcher::new().not(Matcher::new().css("a").direct()).on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "a nested matcher must not have an on_each() callback")]
fn matcher_nested_with_callback() {
    Matcher::new().not(Matcher::new().css("a").on_each(|_| {})).on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "selectors must be added before callbacks")]
fn matcher_selectors_before_callbacks() {
    Matcher::new().css("div").on_each(|_| {}).css("span").build();
}

#[test]
#[should_panic(expected = "build() requires on_each(), on_text_chunk(), on_text() or on_comment()")]
fn matcher_build_requires_callback() {
    Matcher::new().css("div").build();
}

#[test]
#[should_panic(expected = "every() requires at least one matcher")]
fn matcher_every_requires_matchers() {
    Matcher::new().every(vec![]).on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "gap_with_every() requires at least one matcher")]
fn matcher_gap_with_every_requires_matchers() {
    Matcher::new().gap_with_every(vec![]).css("i").on_each(|_| {}).build();
}

#[test]
#[should_panic(expected = "gap_with_any() requires at least one matcher")]
fn matcher_gap_with_any_requires_matchers() {
    Matcher::new().gap_with_any(vec![]).css("i").on_each(|_| {}).build();
}

#[test]
fn matcher_asymptotic_complexity() {
    // D = N here: every element except the deepest one is an ancestor.
    // Doubling N must exactly double the number of DFA transitions.
    let depths = [32, 64, 128, 256, 512];
    let baseline = matcher_operation_count(depths[0]);
    assert!(baseline > 0);
    for depth in depths.into_iter().skip(1) {
        assert_eq!(matcher_operation_count(depth) * depths[0], baseline * depth);
    }
}

#[test]
fn matcher_modifications() {
    // css() — set_attribute()
    check_html(
        Matcher::new().css("i").on_each(|el| {
            el.set_attribute("class", "done").unwrap();
        }),
        r#"<i id="a"></i><b id="b"></b>"#,
        r#"<i id="a" class="done"></i><b id="b"></b>"#,
    );

    // chain — set_inner_content()
    check_html(
        Matcher::new().css(".root").css(".target").on_each(|el| {
            el.set_inner_content("X", ContentType::Html);
        }),
        r#"<div class="root"><i class="target">old</i></div><i class="target">skip</i>"#,
        r#"<div class="root"><i class="target">X</i></div><i class="target">skip</i>"#,
    );

    // filter() — remove_attribute()
    check_html(
        Matcher::new().filter("i", |el| el.get_attribute("data-x").as_deref() == Some("main")).on_each(|el| el.remove_attribute("data-x")),
        r#"<i data-x="main" id="a"></i><i data-x="other" id="b"></i>"#,
        r#"<i id="a"></i><i data-x="other" id="b"></i>"#,
    );

    // not() — replace()
    check_html(
        Matcher::new().not(Matcher::new().css(".skip")).on_each(|el| {
            el.replace("<section/>", ContentType::Html);
        }),
        r#"<div id="a"></div><div class="skip" id="b"></div>"#,
        r#"<section/><div class="skip" id="b"></div>"#,
    );

    // gap_without() — append()
    check_html(
        Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css(".target").on_each(|el| el.append("!", ContentType::Text)),
        r#"<div class="root"><i class="target">a</i><section class="blocked"><i class="target">x</i></section><i class="target">b</i></div>"#,
        r#"<div class="root"><i class="target">a!</i><section class="blocked"><i class="target">x</i></section><i class="target">b!</i></div>"#,
    );

    // gap_with_every() — prepend()
    check_html(
        Matcher::new()
            .css(".root")
            .gap_with_every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")])
            .css(".target")
            .on_each(|el| el.prepend("*", ContentType::Text)),
        r#"<div class="root"><section class="red"><b class="blue"><i class="target">a</i></b></section><section class="red"><i class="target">x</i></section></div>"#,
        r#"<div class="root"><section class="red"><b class="blue"><i class="target">*a</i></b></section><section class="red"><i class="target">x</i></section></div>"#,
    );

    // gap_with_any() — after()
    check_html(
        Matcher::new()
            .css(".root")
            .gap_with_any(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")])
            .css(".target")
            .on_each(|el| el.after("<mark/>", ContentType::Html)),
        r#"<div class="root"><section class="red"><i class="target">a</i></section><section><i class="target">x</i></section></div>"#,
        r#"<div class="root"><section class="red"><i class="target">a</i><mark/></section><section><i class="target">x</i></section></div>"#,
    );

    // every() — set_attribute()
    check_html(
        Matcher::new().every(vec![Matcher::new().css(".item"), Matcher::new().not(Matcher::new().css("span"))]).on_each(|el| {
            el.set_attribute("data-done", "1").unwrap();
        }),
        r#"<div class="item" id="a"></div><div id="b"></div><span class="item" id="c"></span>"#,
        r#"<div class="item" id="a" data-done="1"></div><div id="b"></div><span class="item" id="c"></span>"#,
    );

    // any() — remove()
    check_html(
        Matcher::new().any(vec![Matcher::new().css(".a"), Matcher::new().css(".b")]).on_each(|el| el.remove()),
        r#"<div class="a" id="x"></div><span class="b" id="y"></span><p id="z"></p>"#,
        r#"<p id="z"></p>"#,
    );

    // direct() — before()
    check_html(
        Matcher::new().css(".a").direct().css(".b").on_each(|el| {
            el.before("<mark/>", ContentType::Html);
        }),
        r#"<div class="a"><span class="b"></span></div><span class="b"></span>"#,
        r#"<div class="a"><mark/><span class="b"></span></div><span class="b"></span>"#,
    );

    // nested combinators — remove_and_keep_content()
    check_html(
        Matcher::new()
            .every(vec![
                Matcher::new().css(".target"),
                Matcher::new()
                    .css(".root")
                    .gap_with_every(vec![Matcher::new().css(".red"), Matcher::new().css(".blue")])
                    .css(".middle")
                    .gap_without(Matcher::new().any(vec![Matcher::new().css(".blocked"), Matcher::new().css(".hidden")]))
                    .css(".target"),
            ])
            .on_each(|el| el.remove_and_keep_content()),
        r#"<div class="root"><div class="red"><div class="blue"><section class="middle"><i class="target">keep</i><div class="blocked"><i class="target">skip</i></div></section></div></div></div>"#,
        r#"<div class="root"><div class="red"><div class="blue"><section class="middle">keep<div class="blocked"><i class="target">skip</i></div></section></div></div></div>"#,
    );

    // on_text_chunk() — set_str()
    check_html(
        Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css(".target").on_text_chunk(|chunk| {
            if chunk.last_in_text_node() {
                chunk.set_str("new".into());
            } else {
                chunk.remove();
            }
        }),
        r#"<div class="root"><i class="target">old</i><section class="blocked"><i class="target">old</i></section></div>"#,
        r#"<div class="root"><i class="target">new</i><section class="blocked"><i class="target">old</i></section></div>"#,
    );

    // on_comment() — set_text()
    check_html(
        Matcher::new().css(".root").gap_without(Matcher::new().css(".blocked")).css(".target").on_comment(|comment| {
            comment.set_text("ok").unwrap();
        }),
        r#"<div class="root"><i class="target"><!--a--></i><section class="blocked"><i class="target"><!--b--></i></section></div>"#,
        r#"<div class="root"><i class="target"><!--ok--></i><section class="blocked"><i class="target"><!--b--></i></section></div>"#,
    );
}
