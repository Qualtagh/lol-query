use std::cell::RefCell;
use std::rc::Rc;

use lol_html::{HtmlRewriter, Settings};

use super::Matcher;
use crate::SettingsExt;

#[derive(Clone, Copy)]
struct Capture {
    each: bool,
    text: bool,
    aggregated_text: bool,
    comment: bool,
}

impl Capture {
    const AGGREGATED_TEXT: Self = Self { each: false, text: false, aggregated_text: true, comment: false };
    const ALL: Self = Self { each: true, text: true, aggregated_text: true, comment: true };
    const COMMENT: Self = Self { each: false, text: false, aggregated_text: false, comment: true };
    const EACH: Self = Self { each: true, text: false, aggregated_text: false, comment: false };
    const TEXT: Self = Self { each: false, text: true, aggregated_text: false, comment: false };
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
    if capture.aggregated_text {
        let log = log.clone();
        matcher = matcher.on_text(move |text| {
            if text.is_empty() {
                return;
            }
            log.borrow_mut().push(text.to_string());
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

fn check_aggregated(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::AGGREGATED_TEXT, expected);
}

fn check_comment(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::COMMENT, expected);
}

fn check_all(matcher: Matcher, html: &str, expected: &str) {
    run(vec![matcher], html, Capture::ALL, expected);
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

    // direct() — works with filter() and not() selectors
    check(
        Matcher::new().filter("div", |el| el.get_attribute("data").as_deref() == Some("main")).direct().not(Matcher::new().css("span")),
        r#"<div data="main"><i id="a"></i></div><div data="main"><span id="x"></span></div><div><i id="z"></i></div>"#,
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
        "a keep note keep",
    );

    // several matchers keep their own ancestry when added to the same rewriter
    run(
        vec![Matcher::new().css("div").css("i"), Matcher::new().css("section").css("i")],
        r#"<div><i id="a"></i></div><section><i id="b"></i></section><i id="x"></i>"#,
        Capture::EACH,
        "a b",
    );

    // build paths — css/filter/chain x on_each/on_text_chunk/on_comment/on_text/all
    check(Matcher::new().css("p"), r#"<p id="a"></p><div id="x"></div>"#, "a");
    check_text(Matcher::new().css("p"), r#"<p>hi<b>there</b></p><div>x</div>"#, "hi there");
    check_aggregated(Matcher::new().css("p"), r#"<p>hi<b>there</b></p><div>x</div>"#, "hithere");
    check_comment(Matcher::new().css("p"), r#"<p><!--note--></p><div><!--skip--></div>"#, "note");
    check_all(Matcher::new().css("p"), r#"<p id="a">one<!--c1--></p><div id="x">skip<!--miss--></div><p id="b">two<!--c2--></p>"#, "a one c1 one b two c2 two");

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
        "a c1 yes yes",
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
        "a inner c1 inner",
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
    check_all(Matcher::new().css("p"), r#"<p id="a">one<b>two</b></p>"#, "a one two onetwo");

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
