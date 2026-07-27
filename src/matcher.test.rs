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
    let ids: Ids = Rc::new(RefCell::new(vec![]));
    let capture = ids.clone();
    let handlers = matcher.on_each(move |el| capture.borrow_mut().push(el.get_attribute("id").unwrap_or_default())).build();
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
}
